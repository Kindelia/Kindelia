use bit_vec::BitVec;
use im::HashSet;
use json::object;
use primitive_types::U256;
use priority_queue::PriorityQueue;
use rand::seq::IteratorRandom;
use sha3::Digest;

use std::collections::HashMap;
use std::net::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{SyncSender, Receiver};

use tokio::sync::oneshot;

use std::hash::{BuildHasherDefault};
use crate::{NoHashHasher as NHH, print_with_timestamp};

use crate::util::*;
use crate::bits::*;
use crate::hvm::{self,*};

// Types
// -----

// Kindelia's block format is agnostic to HVM. A Transaction is just a vector of bytes. A Body
// groups transactions in a single combined vector of bytes, using the following format:
//
//   body ::= TX_COUNT | LEN(tx_0) | tx_0 | LEN(tx_1) | tx_1 | ...
//
// TX_COUNT is a single byte storing the number of transactions in this block. The length of each
// transaction is stored using 2 bytes, called LEN.

// A u64 HashMap / HashSet
pub type Map<A> = HashMap<u64, A, BuildHasherDefault<NHH::NoHashHasher<u64>>>;
pub type Set    = HashSet<u64, BuildHasherDefault<NHH::NoHashHasher<u64>>>;

#[derive(Debug, Clone)]
pub struct Transaction {
  pub data: Vec<u8>,
  pub hash: U256,
}

// TODO: store number of used bits
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
  // TODO: optimize size
  pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Block {
  pub time: u128, // block timestamp
  pub meta: u128, // block metadata
  pub prev: U256, // previous block (32 bytes)
  pub body: Body, // block contents (1280 bytes) 
  pub hash: U256, // cached block hash // TODO: refactor out
}

// Blocks have 4 states of inclusion:
//
//   has wait_list? | is on .pending? | is on .block? | meaning
//   -------------- | --------------- | ------------- | ------------------------------------------------------
//   no             | no              | no            | unseen   : never seen, may not exist
//   yes            | no              | no            | missing  : some block cited it, but it wasn't downloaded
//   yes            | yes             | no            | pending  : downloaded, but waiting ancestors for inclusion
//   no             | no              | yes           | included : fully included, as well as all its ancestors
#[derive(Debug, Clone, PartialEq)]
pub enum InclusionState {
  UNSEEN, MISSING, PENDING, INCLUDED
}

// TODO: refactor .block as map to struct? Better safety, less unwraps. Why not?
// TODO: dashmap?
//
// The was_mined field stores which transactions were mined, to avoid re-inclusion. It is NOT
// reversible, though. As such, if a transaction is included, then there is a block reorg that
// drops it, then this node will NOT try to mine it again. It can still be mined by other nodes, or
// re-submitted. FIXME: `was_mined` should be removed. Instead, we just need a priority-queue with
// fast removal of mined transactions. An immutable map should suffice.
pub struct Node {
  pub path       : PathBuf,                          // path where files are saved
  pub socket     : UdpSocket,                        // UDP socket
  pub port       : u16,                              // UDP port
  pub tip        : U256,                             // current tip
  pub block      : U256Map<Block>,                   // block_hash -> block
  pub pending    : U256Map<Block>,                   // block_hash -> downloaded block, waiting for ancestors
  pub ancestor   : U256Map<U256>,                    // block_hash -> hash of its most recent missing ancestor (shortcut jump table)
  pub wait_list  : U256Map<Vec<U256>>,               // block_hash -> hashes of blocks that are waiting for this one
  pub children   : U256Map<Vec<U256>>,               // block_hash -> hashes of this block's children
  pub work       : U256Map<U256>,                    // block_hash -> accumulated work
  pub target     : U256Map<U256>,                    // block_hash -> this block's target
  pub height     : U256Map<u128>,                    // block_hash -> cached height
  pub results    : U256Map<Vec<StatementResult>>,    // block_hash -> results of the statements in this block
  pub pool       : PriorityQueue<Transaction, u64>,  // transactions to be mined
  pub peers      : PeersStore,                       // peers store and state control
  pub runtime    : Runtime,                          // Kindelia's runtime
  pub receiver   : Receiver<Request>,                // Receives an API request
}

// Peers
// =====

pub struct PeersStore {
  seen: HashMap<Address, Peer>,
  active: HashMap<Address, Peer>,
}

impl PeersStore {
  pub fn new() -> PeersStore {
    PeersStore {
      seen: HashMap::new(),
      active: HashMap::new(),
    }
  }

  pub fn see_peer(&mut self, peer: Peer) {
    let addr = peer.address;
    // print_with_timestamp!("- see peer {}", addr);
    match self.seen.get(&addr) {
      None => { // New peer, not seen before
        // print_with_timestamp!("- new peer {}", addr);
        self.seen.insert(addr, peer);
        self.active.insert(addr, peer);
      }
      Some(index) => { // Peer seen before, but maybe not active
        // print_with_timestamp!("- peer {} already seen", addr);
        let old_peer = self.active.get_mut(&addr);
        match old_peer {
          None => { // Peer not active, so activate it
            // print_with_timestamp!("- activating peer {}", addr);
            self.active.insert(addr, peer);
          }
          Some(old_peer) => { // Peer already active, so update it
            // print_with_timestamp!("\t- old peer {:?}", old_peer);
            old_peer.seen_at = std::cmp::max(peer.seen_at, old_peer.seen_at);
            // print_with_timestamp!("\t- new peer {:?}", old_peer);
          }
        }
      }
    }
  }

  fn timeout(&mut self) {
    let mut forget = Vec::new();
    for (id,peer) in &self.active {
      // print_with_timestamp!("- Peer {}: {}", id, peer.address);
      // print_with_timestamp!("... {} < {} {}", peer.seen_at, get_time() - PEER_TIMEOUT, peer.seen_at < get_time() - PEER_TIMEOUT);
      if peer.seen_at < get_time() - PEER_TIMEOUT {
        // print_with_timestamp!("... forgetting {}", id);
        forget.push(peer.address);
      }
    }
    for addr in forget {
      self.inactivate_peer(&addr);
    }
  }

  pub fn inactivate_peer(&mut self, addr: &Address) {
    self.active.remove(addr);
  }

  pub fn get_all_active(&self) -> Vec<Peer> {
    self.active.values().cloned().collect()
  }

  pub fn get_random_active(&self, amount: u128) -> Vec<Peer> {
    let amount = amount as usize;
    let mut rng = rand::thread_rng();
    let peers = self.active.values().cloned().choose_multiple(&mut rng, amount);
    // print_with_timestamp!("- get random peers {:?}", peers.iter().map(|p| p.address).collect::<Vec<_>>());
    peers
  }
}

// API
// ===

#[derive(Debug)]
pub struct BlockInfo {
  pub block: Block,
  pub hash: U256,
  pub height: u64,
  pub content: Vec<hvm::Statement>,
  pub results: Option<Vec<hvm::StatementResult>>,
}

#[derive(Debug)]
pub struct FuncInfo {
  pub func: Func,
}

type RequestAnswer<T> = oneshot::Sender<T>;

// TODO: store and serve tick where stuff where last changed
// TODO: interaction API
pub enum Request {
  GetTick {
    tx: RequestAnswer<u128>,
  },
  GetBlock {
    hash: U256,
    tx: RequestAnswer<Option<BlockInfo>>,
  },
  GetBlocks {
    range: (i64, i64),
    tx: RequestAnswer<Vec<BlockInfo>>,
  },
  GetFunctions {
    tx: RequestAnswer<HashSet<u128>>,
  },
  GetFunction {
    name: u128,
    tx: RequestAnswer<Option<FuncInfo>>,
  },
  GetState {
    name: u128,
    tx: RequestAnswer<Option<Term>>,
  },
  TestCode {
    code: String,
    tx: RequestAnswer<Vec<StatementResult>>,
  },
  PostCode {
    code: String,
    tx: RequestAnswer<Result<(), String>>,
  },
}

#[derive(Debug, Clone)]
pub enum MinerMessage {
  Request {
    prev: U256,
    body: Body,
    targ: U256, 
  },
  Answer {
    block: Block
  },
  Stop
}

#[derive(Debug, Clone)]
pub struct MinerCommunication {
  message: Arc<Mutex<MinerMessage>>
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Message {
  NoticeTheseBlocks {
    gossip: bool,
    blocks: Vec<Block>,
    peers: Vec<Peer>,
  },
  GiveMeThatBlock {
    bhash: Hash
  },
  PleaseMineThisTransaction {
    trans: Transaction
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Address {
  IPv4 {
    val0: u8,
    val1: u8,
    val2: u8,
    val3: u8,
    port: u16,
  }
}

#[derive(Debug, Copy, Clone)]
pub struct Peer {
  pub seen_at: u128,
  pub address: Address,
}

// Constants
// =========

// UDP port to listen to
pub const UDP_PORT : u16 = 42000;

// Size of a hash, in bytes
pub const HASH_SIZE : usize = 32;

// Size of a block's body, in bytes
pub const MAX_BODY_SIZE : usize = 1280;

// Max size of a big UDP packet, in bytes
pub const MAX_UDP_SIZE_SLOW : usize = 8000;

// Max size of a fast UDP packet, in bytes
pub const MAX_UDP_SIZE_FAST : usize = 1500;

// Size of a block, in bytes
//pub const BLOCK_SIZE : usize = HASH_SIZE + (U128_SIZE * 4) + BODY_SIZE;

// Size of an IPv4 address, in bytes
pub const IPV4_SIZE : usize = 4;

// Size of an IPv6 address, in bytes
pub const IPV6_SIZE : usize = 16;

// Size of an IP port, in bytes
pub const PORT_SIZE : usize = 2;

// How many nodes we gossip an information to?
pub const GOSSIP_FACTOR : u128 = 16;

// How many times the mining thread attempts before unblocking?
pub const MINE_ATTEMPTS : u128 = 1024;

// Desired average time between mined blocks, in milliseconds
pub const TIME_PER_BLOCK : u128 = 3000;

// Don't accept blocks from N milliseconds in the future
pub const DELAY_TOLERANCE : u128 = 60 * 60 * 1000;
  
// Readjust difficulty every N blocks
pub const BLOCKS_PER_PERIOD : u128 = 20;

// Readjusts difficulty every N seconds
pub const TIME_PER_PERIOD : u128 = TIME_PER_BLOCK * BLOCKS_PER_PERIOD;

// Initial difficulty, in expected hashes per block
pub const INITIAL_DIFFICULTY : u128 = 256;

// How many milliseconds without notice until we forget a peer?
pub const PEER_TIMEOUT : u128 = 10 * 1000;

// How many peers we need to keep minimum?
pub const PEER_COUNT_MINIMUM : u128 = 256;

// How many peers we send when asked?
pub const SHARE_PEER_COUNT : u128 = 3;

// How many peers we keep on the last_seen object?
pub const LAST_SEEN_SIZE : u128 = 2;

// Delay between handling of network messages, in ms
pub const HANDLE_MESSAGE_DELAY : u128 = 20;

// Delay between handling of API requests, in ms
pub const HANDLE_REQUEST_DELAY : u128 = 20;

// This limits how many messages we accept at once
// FIXME:
// With a handle_message_delay of 20ms, and the message limit of 5, we can handle up to 250
// messages per second. This number is made up. I do not know how many messages we're able to
// handle. We must stress test and benchmark the performance of Node::handle_message, in order to
// come up with a constant that is aligned. Furthermore, we can also greatly optimize the
// performance of Node::handle_message with some key changes, which would allow us to increase that
// limit considerably.
// 1. Cache block hashes:
//   The `NoticeThisBlock` handler hashes the same block twice: one inside `add_block`, and another
//   on the missing ancestor discovery logic. We should avoid that. A good idea might be to include
//   the block hash on the Block struct, so that it is always cached, avoiding re-hashing.
// 2. Use a faster hash function:
//   We can replace every usage of Keccak by K12 on Kindelia, with the only exception being the
//   hash of a public address to end up with an account's name, since Keccak is required to achieve
//   Ethereum account compatibility.
// 3. Receive the hash from the peer:
//   We can *perhaps*, receive a block's hash from the peer that sends it. Obviously, this would
//   open door for several vulnerabilities, but it might be used as a heuristic to avoid slow
//   branches. Of course, when the hash is needed for critical purposes, we must compute it.
// 4. Cache the "first missing ancestor":
//   Every time the `NoticeThisBlock` handler receives a tip, it must find the first missing block
//   on that tips ancestry. That information may be cached, avoiding that loop.
pub const HANDLE_MESSAGE_LIMIT : u128 = 5;


// UDP
// ===

/// Builds an IPV4 Address. TODO: refactor into `impl Address`
pub fn ipv4(val0: u8, val1: u8, val2: u8, val3: u8, port: u16) -> Address {
  Address::IPv4 { val0, val1, val2, val3, port }
}

/// Starts listening to UDP messages on one port of a set of ports
pub fn udp_init(ports: &[u16]) -> Option<(UdpSocket,u16)> {
  for port in ports {
    if let Ok(socket) = UdpSocket::bind(&format!("0.0.0.0:{}",port)) {
      socket.set_nonblocking(true).ok();
      return Some((socket, *port));
    }
  }
  return None;
}

/// Sends an UDP message to many addresses
pub fn udp_send(socket: &mut UdpSocket, addresses: Vec<Address>, message: &Message) {
  let bits = bitvec_to_bytes(&serialized_message(message));
  for address in addresses {
    match address {
      Address::IPv4 { val0, val1, val2, val3, port } => {
        let addr = SocketAddrV4::new(Ipv4Addr::new(val0, val1, val2, val3), port);
        socket.send_to(bits.as_slice(), addr).ok();
      }
    }
  }
}

// Receives an UDP messages
// Non-blocking, returns a vector of received messages on buffer
pub fn udp_recv(socket: &mut UdpSocket) -> Vec<(Address, Message)> {
  let mut buffer = [0; 65536];
  let mut messages = Vec::new();
  while let Ok((msg_len, sender_addr)) = socket.recv_from(&mut buffer) {
    let bits = BitVec::from_bytes(&buffer[0 .. msg_len]);
    if let Some(msge) = deserialized_message(&bits) {
      let addr = match sender_addr.ip() {
        std::net::IpAddr::V4(v4addr) => {
          let [val0, val1, val2, val3] = v4addr.octets();
          Address::IPv4 { val0, val1, val2, val3, port: sender_addr.port() }
        }
        _ => {
          panic!("TODO: IPv6")
        }
      };
      messages.push((addr, msge));
    }
  }
  return messages;
}

// Stringification
// ===============

// Converts a string to an address
pub fn read_address(code: &str) -> Address {
  let strs = code.split(':').collect::<Vec<&str>>();
  let vals = strs[0].split('.').map(|o| o.parse::<u8>().unwrap()).collect::<Vec<u8>>();
  let port = strs.get(1).map(|s| s.parse::<u16>().unwrap()).unwrap_or(UDP_PORT);
  Address::IPv4 {
    val0: vals[0],
    val1: vals[1],
    val2: vals[2],
    val3: vals[3],
    port: port,
  }
}

// Shows an address's hostname
pub fn show_address_hostname(address: &Address) -> String {
  match address {
    Address::IPv4{ val0, val1, val2, val3, port } => {
      return format!("{}.{}.{}.{}", val0, val1, val2, val3);
    }
  }
}

// Algorithms
// ----------

// Converts a target to a difficulty (see below)
pub fn target_to_difficulty(target: U256) -> U256 {
  let p256 = U256::from("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
  return p256 / (p256 - target);
}

// Converts a difficulty to a target (see below)
pub fn difficulty_to_target(difficulty: U256) -> U256 {
  let p256 = U256::from("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
  return p256 - p256 / difficulty;
}

// Target is a U256 number. A hash larger than or equal to that number hits the target.
// Difficulty is an estimation of how many hashes it takes to hit a given target.

// Computes next target by scaling the current difficulty by a `scale` factor.
// Since the factor is an integer, it is divided by 2^32 to allow integer division.
// - compute_next_target(t, 2n**32n / 2n): difficulty halves
// - compute_next_target(t, 2n**32n * 1n): nothing changes
// - compute_next_target(t, 2n**32n * 2n): difficulty doubles
pub fn compute_next_target(last_target: U256, scale: U256) -> U256 {
  let p32 = U256::from("0x100000000");
  let last_difficulty = target_to_difficulty(last_target);
  let next_difficulty = u256(1) + (last_difficulty * scale - u256(1)) / p32;
  return difficulty_to_target(next_difficulty);
}

// Computes the next target, scaling by a floating point factor.
pub fn compute_next_target_f64(last_target: U256, scale: f64) -> U256 {
  return compute_next_target(last_target, u256(scale as u128));
}

// Estimates how many hashes were necessary to get this one.
pub fn get_hash_work(hash: U256) -> U256 {
  if hash == u256(0) {
    return u256(0);
  } else {
    return target_to_difficulty(hash);
  }
}

// Hashes a U256 value.
pub fn hash_u256(value: U256) -> U256 {
  return hash_bytes(u256_to_bytes(value).as_slice());
}

// Hashes a byte array.
pub fn hash_bytes(bytes: &[u8]) -> U256 {
  let mut hasher = sha3::Keccak256::new();
  hasher.update(&bytes);
  let hash = hasher.finalize();
  return U256::from_little_endian(&hash);
}

// Creates a new block.
pub fn new_block(prev: U256, time: u128, meta: u128, body: Body) -> Block {
  let hash = if time == 0 {
    hash_bytes(&[])
  } else {
    let mut bytes : Vec<u8> = Vec::new();
    bytes.extend_from_slice(&u256_to_bytes(prev));
    bytes.extend_from_slice(&u128_to_bytes(time));
    bytes.extend_from_slice(&u128_to_bytes(meta));
    bytes.extend_from_slice(&body.data);
    hash_bytes(&bytes)
  };
  return Block { prev, time, meta, body, hash };
}

// Converts a byte array to a Body.
pub fn bytes_to_body(bytes: &[u8]) -> Body {
  Body { data: bytes.to_vec() }
}

// Converts a string (with a list of statements) to a body.
pub fn code_to_body(code: &str) -> Body {
  let (_rest, acts) = crate::hvm::read_statements(code).unwrap(); // TODO: handle error
  let bits = serialized_statements(&acts);
  let body = bytes_to_body(&bitvec_to_bytes(&bits));
  return body;
}

// Encodes a transaction length as a pair of 2 bytes
fn encode_length(len: usize) -> (u8, u8) {
  let num = (len as u16).reverse_bits();
  (((num >> 8) & 0xFF) as u8, (num & 0xFF) as u8)
}

// Decodes an encoded transaction length
fn decode_length(pair: (u8,u8)) -> usize {
  (((pair.0 as u16) << 8) | (pair.1 as u16)).reverse_bits() as usize
}

// Converts a body to a vector of transactions.
pub fn extract_transactions(body: &Body) -> Vec<Transaction> {
  let mut transactions = Vec::new();
  let mut index = 1;
  let tx_count = body.data[0];
  for i in 0 .. tx_count {
    if index >= body.data.len() { break; }
    let tx_len = decode_length((body.data[index], body.data[index + 1]));
    index += 2;
    if index + tx_len > body.data.len() { break; }
    transactions.push(Transaction::new(body.data[index .. index + tx_len].to_vec()));
    index += tx_len;
  }
  return transactions;
}

// Initial target of 256 hashes per block
pub fn INITIAL_TARGET() -> U256 {
  return difficulty_to_target(u256(INITIAL_DIFFICULTY));
}

// The hash of the genesis block's parent.
pub fn ZERO_HASH() -> U256 {
  return hash_u256(u256(0)); // why though
}

// The genesis block.
pub fn GENESIS_BLOCK() -> Block {
  return new_block(ZERO_HASH(), 0, 0, Body { data: vec![0] });
}

// Converts a block to a string.
// FIXME: is this still used?
//pub fn show_block(block: &Block) -> String {
  //return format!(
    //"time: {}\nrand: {}\nbody: {}\nprev: {}\nhash: {} ({})\n-----\n",
    //block.time,
    //block.meta,
    //body_to_string(&block.body),
    //block.prev,
    //hex::encode(u256_to_bytes(block.hash)),
    //get_hash_work(block.hash),
  //);
//}



impl Transaction {
  pub fn new(mut data: Vec<u8>) -> Self {
    // Transaction length is always a non-zero multiple of 5
    while data.len() == 0 || data.len() % 5 != 0 {
      data.push(0);
    }
    let hash = hash_bytes(&data);
    return Transaction { data, hash };
  }

  pub fn encode_length(&self) -> (u8, u8) {
    return encode_length(self.data.len());
  }

  pub fn to_statement(&self) -> Option<Statement> {
    return deserialized_statement(&BitVec::from_bytes(&self.data));
  }
}

impl PartialEq for Transaction {
  fn eq(&self, other: &Self) -> bool {
    self.hash == other.hash
  }
}

impl Eq for Transaction {}

impl std::hash::Hash for Transaction {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.hash.hash(state);
  }
}

// Mining
// ------

// Given a target, attempts to mine a block by changing its nonce up to `max_attempts` times
pub fn try_mine(prev: U256, body: Body, targ: U256, max_attempts: u128) -> Option<Block> {
  let rand = rand::random::<u128>();
  let time = get_time();
  let mut block = new_block(prev, time, rand, body);
  for _i in 0 .. max_attempts {
    if block.hash >= targ {
      return Some(block);
    } else {
      block.meta = block.meta.wrapping_add(1);
    }
  }
  return None;
}

impl MinerCommunication {
  // Creates a shared MinerCommunication object
  pub fn new() -> Self {
    MinerCommunication {
      message: Arc::new(Mutex::new(MinerMessage::Stop))
    }
  }

  // Writes the shared MinerCommunication object
  pub fn write(&mut self, new_message: MinerMessage) {
    let mut value = self.message.lock().unwrap();
    *value = new_message;
  }

  pub fn read(&self) -> MinerMessage {
    return (*self.message.lock().unwrap()).clone();
  }
}

// Main miner loop: if asked, attempts to mine a block
pub fn miner_loop(mut miner_communication: MinerCommunication) {
  loop {
    if let MinerMessage::Request { prev, body, targ } = miner_communication.read() {
      //print_with_timestamp!("[miner] mining with target: {}", hex::encode(u256_to_bytes(targ)));
      let mined = try_mine(prev, body, targ, MINE_ATTEMPTS);
      if let Some(block) = mined {
        //print_with_timestamp!("[miner] mined a block!");
        miner_communication.write(MinerMessage::Answer { block });
      }
    }
  }
}

// Node
// ----

impl Node {
  pub fn new(
    kindelia_path: PathBuf,
    init_peers: &Option<Vec<Address>>,
  ) -> (SyncSender<Request>, Self) {
    let try_ports = [UDP_PORT, UDP_PORT + 1, UDP_PORT + 2, UDP_PORT + 3];
    let (socket, port) = udp_init(&try_ports).expect("Couldn't open UDP socket.");
    let (query_sender, query_receiver) = mpsc::sync_channel(1);
    let mut node = Node {
      path       : kindelia_path,
      socket     : socket,
      port       : port,
      block      : u256map_from([(ZERO_HASH(), GENESIS_BLOCK())]),
      pending    : u256map_new(),
      ancestor   : u256map_new(),
      wait_list  : u256map_new(),
      children   : u256map_from([(ZERO_HASH(), vec![])]),
      work       : u256map_from([(ZERO_HASH(), u256(0))]),
      height     : u256map_from([(ZERO_HASH(), 0)]),
      target     : u256map_from([(ZERO_HASH(), INITIAL_TARGET())]),
      results    : u256map_from([(ZERO_HASH(), vec![])]),
      tip        : ZERO_HASH(),
      pool       : PriorityQueue::new(),
      peers      : PeersStore::new(),
      runtime    : init_runtime(None),
      receiver   : query_receiver,
    };

    let now = get_time();

    if let Some(init_peers) = init_peers {
      init_peers.iter().for_each(|address| {
        return node.peers.see_peer(Peer { address: *address, seen_at: now });
      });
    }

    // TODO: For testing purposes. Remove later.
    for &peer_port in try_ports.iter() {
      if peer_port != port {
        let address = Address::IPv4 { val0: 127, val1: 0, val2: 0, val3: 1, port: peer_port };
        node.peers.see_peer(Peer { address: address, seen_at: now })
      }
    }

    (query_sender, node)
  }

  // Registers a block on the node's database. This performs several actions:
  // - If this block is too far into the future, ignore it.
  // - If this block's parent isn't available:
  //   - Add this block to the parent's wait_list
  //   - When the parent is available, register this block again
  // - If this block's parent is available:
  //   - Compute the block accumulated work, target, etc.
  //   - If this block is the new tip:
  //     - In case of a reorg, rollback to the block before it
  //     - Run that block's code, updating the HVM state
  //     - Updates the longest chain saved on disk
  pub fn add_block(&mut self, block: &Block) {
    // Adding a block might trigger the addition of other blocks
    // that were waiting for it. Because of that, we loop here.
    let mut must_include = vec![block.clone()]; // blocks to be added
    //print_with_timestamp!("- add_block");
    // While there is a block to add...
    while let Some(block) = must_include.pop() {
      let btime = block.time; // the block timestamp
      //print_with_timestamp!("- add block time={}", btime);
      // If block is too far into the future, ignore it
      if btime >= get_time() + DELAY_TOLERANCE {
        //print_with_timestamp!("# new block: too late");
        continue;
      }
      let bhash = block.hash; // hash of the block
      // If we already registered this block, ignore it
      if self.block.get(&bhash).is_some() {
        //print_with_timestamp!("# new block: already in");
        continue;
      }
      let phash = block.prev; // hash of the previous block
      // If previous block is available, add the block to the chain
      if self.block.get(&phash).is_some() {
        //print_with_timestamp!("- previous available");
        let work = get_hash_work(bhash); // block work score
        self.block.insert(bhash, block.clone()); // inserts the block
        self.work.insert(bhash, u256(0)); // inits the work attr
        self.height.insert(bhash, 0); // inits the height attr
        self.target.insert(bhash, u256(0)); // inits the target attr
        self.children.insert(bhash, vec![]); // inits the children attrs
        self.ancestor.remove(&bhash); // remove it from the ancestor jump table
        // Checks if this block PoW hits the target
        let has_enough_work = bhash >= self.target[&phash];
        // Checks if this block's timestamp is larger than its parent's timestamp
        // Note: Bitcoin checks if it is larger than the median of the last 11 blocks; should we?
        let advances_time = btime > self.block[&phash].time;
        // If the PoW hits the target and the block's timestamp is valid...
        if has_enough_work && advances_time {
          //print_with_timestamp!("# new_block: enough work & advances_time");
          self.work.insert(bhash, self.work[&phash] + work); // sets this block accumulated work
          self.height.insert(bhash, self.height[&phash] + 1); // sets this block accumulated height
          // If this block starts a new period, computes the new target
          if self.height[&bhash] > 0 && self.height[&bhash] > BLOCKS_PER_PERIOD && self.height[&bhash] % BLOCKS_PER_PERIOD == 1 {
            // Finds the checkpoint hash (hash of the first block of the last period)
            let mut checkpoint_hash = phash;
            for _ in 0 .. BLOCKS_PER_PERIOD - 1 {
              checkpoint_hash = self.block[&checkpoint_hash].prev;
            }
            // Computes how much time the last period took to complete
            let period_time = btime - self.block[&checkpoint_hash].time;
            // Computes the target of this period
            let last_target = self.target[&phash];
            let next_scaler = 2u128.pow(32) * TIME_PER_PERIOD / period_time;
            let next_target = compute_next_target(last_target, u256(next_scaler));
            // Sets the new target
            self.target.insert(bhash, next_target);
          // Otherwise, keep the old target
          } else {
            self.target.insert(bhash, self.target[&phash]);
          }
          // Removes this block's transactions from mempool
          for tx in extract_transactions(&block.body) {
            self.pool.remove(&tx);
          }
          // Updates the tip work and block hash
          let old_tip = self.tip;
          let new_tip = bhash;
          if self.work[&new_tip] > self.work[&old_tip] {
            self.tip = bhash;
            //print_with_timestamp!("- hash: {:x}", bhash);
            //print_with_timestamp!("- work: {}", self.work[&new_tip]);
            if true {
              // Block reorganization (* marks blocks for which we have runtime snapshots):
              // tick: |  0 | *1 |  2 |  3 |  4 | *5 |  6 | *7 | *8 |
              // hash: |  A |  B |  C |  D |  E |  F |  G |  H |    |  <- old timeline
              // hash: |  A |  B |  C |  D |  P |  Q |  R |  S |  T |  <- new timeline
              //               |         '-> highest common block shared by both timelines
              //               '-----> highest runtime snapshot before block D
              let mut must_compute = Vec::new();
              let mut old_bhash = old_tip;
              let mut new_bhash = new_tip;
              // 1. Finds the highest block with same height on both timelines
              //    On the example above, we'd have `H, S`
              while self.height[&new_bhash] > self.height[&old_bhash] {
                must_compute.push(new_bhash);
                new_bhash = self.block[&new_bhash].prev;
              }
              while self.height[&old_bhash] > self.height[&new_bhash] {
                old_bhash = self.block[&old_bhash].prev;
              }
              // 2. Finds highest block with same value on both timelines
              //    On the example above, we'd have `D`
              while old_bhash != new_bhash {
                must_compute.push(new_bhash);
                old_bhash = self.block[&old_bhash].prev;
                new_bhash = self.block[&new_bhash].prev;
              }
              // 3. Saves overwritten blocks to disk
              for bhash in must_compute.iter().rev() {
                let file_path = self.get_blocks_path().join(format!("{:0>32x}.kindelia_block.bin", self.height[bhash]));
                let file_buff = bitvec_to_bytes(&serialized_block(&self.block[bhash]));
                std::fs::write(file_path, file_buff).expect("Couldn't save block to disk.");
              }
              // 4. Reverts the runtime to a state older than that block
              //    On the example above, we'd find `runtime.tick = 1`
              let mut tick = self.height[&old_bhash];
              //print_with_timestamp!("- tick: old={} new={}", self.runtime.get_tick(), tick);
              self.runtime.rollback(tick);
              // 5. Finds the last block included on the reverted runtime state
              //    On the example above, we'd find `new_bhash = B`
              while tick > self.runtime.get_tick() {
                must_compute.push(new_bhash);
                new_bhash = self.block[&new_bhash].prev;
                tick -= 1;
              }
              // 6. Computes every block after that on the new timeline
              //    On the example above, we'd compute `C, D, P, Q, R, S, T`
              for block in must_compute.iter().rev() {
                self.compute_block(&self.block[block].clone());
              }
            }
          }
        }
        // Registers this block as a child of its parent
        self.children.insert(phash, vec![bhash]);
        // If there were blocks waiting for this one, include them on the next loop
        // This will cause the block to be moved from self.pending to self.block
        if let Some(wait_list) = self.wait_list.get(&bhash) {
          for waiting_for_me in wait_list {
            must_include.push(self.pending.remove(waiting_for_me).expect("block"));
          }
          self.wait_list.remove(&bhash);
        }
      // Otherwise, if the previous block isn't available,
      // include this block on .pending, and on its parent's wait_list
      } else if self.pending.get(&bhash).is_none() {
        self.pending.insert(bhash, block.clone());
        self.wait_list.entry(phash).or_insert_with(|| Vec::new()).push(bhash);
      }
    }
  }

  pub fn compute_block(&mut self, block: &Block) {
    //print_with_timestamp!("Computing block...");
    //print_with_timestamp!("==================");
    let transactions = extract_transactions(&block.body);
    let mut statements = Vec::new();
    for transaction in transactions {
      if let Some(statement) = transaction.to_statement() {
        //print_with_timestamp!("- {}", view_statement(&statement));
        statements.push(statement);
      }
    }
    self.runtime.set_time(block.time >> 8);
    self.runtime.set_meta(block.meta >> 8);
    self.runtime.set_hax0((block.hash >>   0).low_u128() >> 8);
    self.runtime.set_hax1((block.hash >> 120).low_u128() >> 8);
    let result = self.runtime.run_statements(&statements, false);
    self.results.insert(block.hash, result);
    self.runtime.tick();
  }

  // Get the current target
  pub fn get_tip_target(&self) -> U256 {
    self.target[&self.tip]
  }

  pub fn get_longest_chain(&self, num: Option<usize>) -> Vec<U256> {
    let mut longest = Vec::new();
    let mut bhash = self.tip;
    let mut count = 0;
    while self.block.contains_key(&bhash) && bhash != ZERO_HASH() {
      let block = self.block.get(&bhash).unwrap();
      longest.push(bhash);
      bhash = block.prev;
      count += 1;
      if let Some(num) = num {
        if count >= num {
          break;
        }
      }
    }
    longest.reverse();
    return longest;
  }

  pub fn receive_message(&mut self) {
    let mut count = 0;
    for (addr, msg) in udp_recv(&mut self.socket) {
      //if count < HANDLE_MESSAGE_LIMIT {
      self.handle_message(addr, &msg);
      count = count + 1;
      //}
    }
  }

  fn receive_request(&mut self) {
    if let Ok(request) = self.receiver.try_recv() {
      self.handle_request(request);
    }
  }

  pub fn get_block_info(&self, hash: &U256) -> Option<BlockInfo> {
    let block = self.block.get(hash)?;
    let height = self.height.get(hash).expect("Missing block height.");
    let height: u64 = (*height).try_into().expect("Block height is too big.");
    let results = self.results.get(hash).map(|r| r.clone());
    let transactions = extract_transactions(&block.body);
    let content = transactions.iter().filter_map(Transaction::to_statement).collect();
    let info = BlockInfo {
      block: block.clone(),
      hash: *hash,
      height,
      content,
      results,
    };
    Some(info)
  }

  pub fn get_func_info(&self, fid: u128) -> Option<FuncInfo> {
    let comp_func = self.runtime.read_file(fid)?;
    let func = comp_func.func;
    Some(FuncInfo { func })
  }

  pub fn handle_request(&mut self, request: Request) {
    // TODO: handle unwraps
    match request {
      Request::GetTick { tx: answer } => {
        answer.send(self.runtime.get_tick()).unwrap();
      }
      Request::GetBlocks { range, tx: answer } => {
        let (start, end) = range;
        debug_assert!(start <= end);
        debug_assert!(end == -1);
        let num = (end - start + 1) as usize;
        let hashes = self.get_longest_chain(Some(num));
        let infos = hashes.iter()
          .map(|h| 
            self.get_block_info(h).expect("Missing block.")
          ).collect();
        answer.send(infos).unwrap();
      },
      Request::GetBlock { hash, tx: answer } => {
        // TODO: actual indexing
        let info = self.get_block_info(&hash);
        answer.send(info).unwrap();
      },
      Request::GetFunctions { tx } => {
        let mut funcs: HashSet<u128> = HashSet::new();
        self.runtime.reduce_with(&mut funcs, |acc, heap| {
          for func in heap.disk.links.keys() {
            acc.insert(*func);
          }
        });
        tx.send(funcs).unwrap();
      },
      Request::GetFunction { name, tx: answer } =>  {
        let info = self.get_func_info(name);
        answer.send(info).unwrap();
      },
      Request::GetState { name, tx: answer } => {
        let state = self.runtime.read_disk_as_term(name);
        answer.send(state).unwrap();
      },
      Request::TestCode { code, tx: answer } => {
        let result = self.runtime.test_statements_from_code(&code);
        answer.send(result).unwrap();
      },
      Request::PostCode { code, tx: answer } => {
        let statements = 
          hvm::read_statements(&code)
            .map_err(|err| err.erro)
            .map(|(_, s)| s);
      
        let res = match statements {
          Err(err) => {
            Err(err)
          }
          Ok(statements) => {
            statements
              .iter()
              .map(
                |s| 
                  Transaction::new(bitvec_to_bytes(&serialized_statement(s)))
              )
              .into_iter()
              .for_each(|t| {
                let hash = t.hash.low_u64();
                self.pool.push(t, hash);
              });
            Ok(())
          }
        };
      
        answer.send(res).unwrap();
      }
    }
  }

  // Sends a block to a target address; also share some random peers
  // FIXME: instead of sharing random peers, share recently active peers
  pub fn send_blocks_to(&mut self, addrs: Vec<Address>, gossip: bool, blocks: Vec<Block>, share_peers: u128) {
    //print_with_timestamp!("- sending block: {:?}", block);
    let peers = self.peers.get_random_active(share_peers);
    let msg = Message::NoticeTheseBlocks { gossip, blocks, peers };
    // print_with_timestamp!("- sending block: {:?}", msg);
    udp_send(&mut self.socket, addrs, &msg);
  }

  // Returns the block inclusion state
  pub fn inclusion_state(&self, bhash: &U256) -> InclusionState {
    if self.block.contains_key(bhash) {
      return InclusionState::INCLUDED;
    }
    if self.pending.contains_key(bhash) {
      return InclusionState::PENDING;
    }
    if self.wait_list.contains_key(bhash) {
      return InclusionState::MISSING;
    }
    return InclusionState::UNSEEN;
  }

  // Finds the most recent missing ancestor of a pending block
  pub fn find_missing_ancestor(&mut self, bhash: &U256) -> Option<U256> {
    if self.inclusion_state(bhash) == InclusionState::PENDING {
      let ancestor = {
        let mut ancestor = bhash;
        while self.inclusion_state(ancestor) == InclusionState::PENDING {
          if let Some(jump_to) = self.ancestor.get(&ancestor) {
            ancestor = jump_to;
          }
          if let Some(ancestor_block) = self.pending.get(&ancestor) {
            ancestor = &ancestor_block.prev;
          }
        }
        *ancestor
      };
      self.ancestor.insert(*bhash, ancestor);
      return Some(ancestor);
    }
    // This should NOT happen, since we only call find_missing_ancestor after add_block.
    debug_assert!(self.inclusion_state(bhash) != InclusionState::MISSING);
    debug_assert!(self.inclusion_state(bhash) != InclusionState::UNSEEN);
    return None;
  }

  // Requests the most recent missing ancestor
  pub fn request_missing_ancestor(&mut self, addr: Address, bhash: &U256) {
    if let Some(missing_ancestor) = self.find_missing_ancestor(bhash) {
      udp_send(&mut self.socket, vec![addr], &Message::GiveMeThatBlock { bhash: missing_ancestor })
    }
  }

  pub fn handle_message(&mut self, addr: Address, msg: &Message) {
    if addr != (Address::IPv4 { val0: 127, val1: 0, val2: 0, val3: 1, port: self.port }) {
      // print_with_timestamp!("- received message from {:?}: {:?}", addr, msg);
      self.peers.see_peer(Peer { address: addr, seen_at: get_time() });
      match msg {
        // Someone asked a block
        Message::GiveMeThatBlock { bhash } => {
          // Sends the requested block, plus some of its ancestors
          let mut bhash = bhash;
          let mut chunk = vec![];
          let mut tsize = 0; // total size of the corresponding "NoticeTheseBlocks" message
          loop {
            if !self.block.contains_key(&bhash) { break; }
            if *bhash == ZERO_HASH() { break; }
            let block = &self.block[bhash];
            let bsize = serialized_block_size(block) as usize;
            if tsize + bsize > MAX_UDP_SIZE_SLOW { break }
            chunk.push(block.clone());
            tsize += bsize;
            bhash = &block.prev;
          }
          self.send_blocks_to(vec![addr], false, chunk, 0);
        }
        // Someone sent us some blocks
        Message::NoticeTheseBlocks { gossip, blocks, peers } => {
          // TODO: validate if blocks are sorted by age?

          // Notice received peers
          for peer in peers {
            self.peers.see_peer(*peer);
          }

          // Adds the block to the database
          for block in blocks {
            self.add_block(block);
          }

          // Requests missing ancestors
          if *gossip && blocks.len() > 0 {
            self.request_missing_ancestor(addr, &blocks[0].hash);
          }
        }
        // Someone sent us a transaction to mine
        Message::PleaseMineThisTransaction { trans } => {
          //print_with_timestamp!("- Transaction added to pool:");
          //print_with_timestamp!("-- {:?}", trans.data);
          //print_with_timestamp!("-- {}", if let Some(st) = trans.to_statement() { view_statement(&st) } else { String::new() });
          if self.pool.get(&trans).is_none() {
            self.pool.push(trans.clone(), trans.hash.low_u64());
            self.gossip(5, msg);
          }
        }
      }
    }
  }

  pub fn gossip(&mut self, peer_count: u128, message: &Message) {
    let addrs = self.peers.get_random_active(peer_count).iter().map(|x| x.address).collect();
    udp_send(&mut self.socket, addrs, message);
  }

  pub fn get_blocks_path(&self) -> PathBuf {
    self.path.join("state").join("blocks")
  }

  fn broadcast_tip_block(&mut self) {
    let addrs  = self.peers.get_all_active().iter().map(|x| x.address).collect();
    let blocks = vec![self.block[&self.tip].clone()];
    self.send_blocks_to(addrs, true, blocks, 3);
  }

  fn gossip_tip_block(&mut self, peer_count: u128) {
    let addrs  = self.peers.get_random_active(peer_count).iter().map(|x| x.address).collect();
    let blocks = vec![self.block[&self.tip].clone()];
    self.send_blocks_to(addrs, true, blocks, 3);
  }

  fn load_blocks(&mut self) {
    let blocks_dir = self.get_blocks_path();
    std::fs::create_dir_all(&blocks_dir).ok();
    let mut file_paths : Vec<PathBuf> = vec![];
    for entry in std::fs::read_dir(&blocks_dir).unwrap() {
      file_paths.push(entry.unwrap().path());
    }
    file_paths.sort();
    // print_with_timestamp!("Loading {} blocks from disk...", file_paths.len());
    for file_path in file_paths {
      let buffer = std::fs::read(file_path.clone()).unwrap();
      let block = deserialized_block(&bytes_to_bitvec(&buffer)).unwrap();
      self.add_block(&block);
    }
  }

  fn ask_mine(&self, miner_communication: &mut MinerCommunication, body: Body) {
    //print_with_timestamp!("Asking miner to mine:");
    //for transaction in extract_transactions(&body) {
      //print_with_timestamp!("- statement: {}", view_statement(&transaction.to_statement().unwrap()));
    //}
    miner_communication.write(MinerMessage::Request {
      prev: self.tip,
      body,
      targ: self.get_tip_target(),
    });
  }

  fn add_mined_block(&mut self, miner_communication: &MinerCommunication) {
    if let MinerMessage::Answer { block } = miner_communication.read() {
      self.add_block(&block);
      self.broadcast_tip_block();
    }
  }

  // Builds the body to be mined.
  // To convert back to a vector of transactions, use `extract_transactions()`.
  pub fn build_body(&self) -> Body {
    let mut body_vec = vec![0]; 
    let mut tx_count = 0;
    for (transaction, score) in self.pool.iter() {
      let tx_len = transaction.data.len();
      if tx_len == 0 { continue; }
      let len_info = transaction.encode_length(); // number we will store as the length
      if body_vec.len() + 2 + tx_len > MAX_BODY_SIZE { break; }
      if tx_count + 1 > 255 { break; }
      body_vec.push(len_info.0);
      body_vec.push(len_info.1);
      body_vec.extend_from_slice(&transaction.data);
      tx_count += 1;
    }
    body_vec[0] = tx_count as u8;
    return Body { data: body_vec };
  }

  fn log_heartbeat(&self) {
    let tip = self.tip;
    let tip_height = *self.height.get(&tip).unwrap() as u64;

    let tip_target = *self.target.get(&tip).unwrap();
    let difficulty = target_to_difficulty(tip_target);
    let hash_rate = difficulty * u256(1000) / u256(TIME_PER_BLOCK);

    // Counts missing, pending and included blocks
    let included_count = self.block.keys().count();
    let mut missing_count: u64 = 0;
    let mut pending_count: u64 = 0;
    for (bhash, _) in self.wait_list.iter() {
      if self.pending.get(bhash).is_some() {
        pending_count += 1;
      }
      missing_count += 1;
    }

    let mana_cur = self.runtime.get_mana() as i64;
    let mana_lim = self.runtime.get_mana_limit() as i64;
    let size_cur = self.runtime.get_size() as i64;
    let size_lim = self.runtime.get_size_limit() as i64;
    let mana_avail = mana_lim - mana_cur;
    let size_avail = size_lim - size_cur;
    debug_assert!(size_avail >= 0);
    debug_assert!(mana_avail >= 0);

    let peers_num = self.peers.get_all_active().len();

    let log = object!{
      event: "heartbeat",
      peers: { num: peers_num },
      tip: {
        height: tip_height,
        // target: u256_to_hex(tip_target),
        difficulty: difficulty.low_u64(),
        hashrate: hash_rate.low_u64(),
      },
      blocks: {
        missing: missing_count,
        pending: pending_count,
        included: included_count,
      },
      runtime: {
        mana: {
          current: mana_cur.to_string(),
          limit: mana_lim.to_string(),
          available: mana_avail.to_string(),
        },
        size: {
          current: size_cur,
          limit: size_lim,
          available: size_avail,
        }
      }
    };

    // print_with_timestamp!("{}", log);
  }

  pub fn main(mut self, kindelia_path: PathBuf, mut miner_communication: MinerCommunication, mine: bool) -> ! {

    print_with_timestamp!("Port: {}", self.port);
    print_with_timestamp!("Initial peers: ");
    for peer in self.peers.get_all_active() {
      print_with_timestamp!("- {}", peer.address);
    }

    // Loads all stored blocks. FIXME: remove the if (used for debugging)
    if self.port == UDP_PORT {
      self.load_blocks();
    }

   // A task that is executed continuously on the main loop
    struct Task {
      pub delay : u128,
      pub action : fn (&mut Node, &mut MinerCommunication) -> (),
    }

    // The vector of tasks
    let mut tasks = vec![
      // Gossips the tip block
      Task {
        delay: 20,
        action: |node, mc| { node.gossip_tip_block(8); },
      },
      // Receives and handles incoming network messages
      Task {
        delay: HANDLE_MESSAGE_DELAY,
        action: |node, mc| { node.receive_message(); },
      },
      // Receives and handles incoming API requests
      Task {
        delay: HANDLE_REQUEST_DELAY,
        action: |node, mc| { node.receive_request(); },
      },
      // Forgets inactive peers
      Task {
        delay: 5_000,
        action: |node, mc| { node.peers.timeout(); },
      },
      // Prints stats
      Task {
        delay: 1_000,
        action: |node, mc| { node.log_heartbeat(); },
      },
    ];

    if mine {
      let miner_tasks = vec![
        // Asks the miner thread to mine a block
        Task {
          delay: 1000,
          action: |node, mc| { node.ask_mine(mc, node.build_body()); },
        },
        // If the miner mined a block, adds it
        Task {
          delay: 5,
          action: |node, mc| { node.add_mined_block(mc); },
        },
      ];
      tasks.extend(miner_tasks);
    }

    // Last time a task was executed
    let mut last_tick_time: Vec<u128> = vec![0; tasks.len()];

    loop {
      let now = std::time::Instant::now();
      let system_time = get_time(); // Measured in milliseconds
      for (i, task) in tasks.iter().enumerate() {
        if last_tick_time[i] + task.delay <= system_time {
          (task.action)(&mut self, &mut miner_communication);
          last_tick_time[i] = system_time;
        }
      }
      let elapsed = now.elapsed();
      let extra = std::time::Duration::from_millis(1).checked_sub(elapsed);
      // If the elapsed time is less than 1ms, sleep for the remaining time
      if let Some(extra) = extra {
        std::thread::sleep(extra);
      }
    }
  }
}

impl std::fmt::Display for Address {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Address::IPv4 { val0, val1, val2, val3, port } => {
        f.write_fmt(format_args!("{}.{}.{}.{}:{}", val0, val1, val2, val3, port))
      },
    }
  }
}
