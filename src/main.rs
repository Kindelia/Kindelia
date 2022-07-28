#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(clippy::style)]

#[cfg(test)]
mod test;
#[cfg(test)]
use rstest_reuse;

mod api;
mod bits;
mod crypto;
mod hvm;
mod node;
mod util;
mod NoHashHasher;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

pub use clap::{Parser, Subcommand};

use crate::api::api_loop;
use crate::bits::*;
use crate::hvm::*;
use crate::node::*;
use crate::util::*;

// Kindelia Foundation nodes
const ENTRY_PEERS : [&str; 4] = [
  "143.244.179.61:42000",
  "164.92.214.78:42000",
  "159.223.39.129:42000",
  "159.65.148.6:42000",
];

// Starts the node process
fn main() -> Result<(), String> {
  //for i in 0 .. 3200 {
    //let mut bits = bit_vec::BitVec::new();
    //serialize_fixlen(16, &u256(i), &mut bits, &mut std::collections::HashMap::new());
    //fn encode_length(len: usize) -> (u8, u8) {
      //let num = (len as u16).reverse_bits();
      //(((num >> 8) & 0xFF) as u8, (num & 0xFF) as u8)
    //}
    //fn decode_length(pair: (u8,u8)) -> u16 {
      //(((pair.0 as u16) << 8) | (pair.1 as u16)).reverse_bits()
    //}
    //println!("{:?}", bits.to_bytes());
    //println!("{:?}", encode_length(i as usize));
    //println!("{} == {}", i, decode_length(encode_length(i as usize)));
  //}
  return run_cli();
  //start_node(dirs::home_dir().unwrap().join(".kindelia"), false);
  //hvm::test_statements_from_file("./example/block_1.kdl");
  //return Ok(());
}

/// Environment variable where Kindelia path should be passed.
const KINDELIA_PATH_ENV_VAR: &str = "KINDELIA_PATH";

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
  /// Path where Kindelia files are stored
  #[clap(long)]
  path: Option<String>,
  #[clap(subcommand)]
  pub command: CliCmd,
}

#[derive(Subcommand)]
pub enum CliCmd {
  /// Starts a Kindelia node
  Start {
    /// Adds testnet nodes as initial peers
    #[clap(long)]
    testnet: bool,
    /// Mine blocks
    #[clap(long)]
    mine: bool,
  },
  /// Runs a Kindelia (.kdl) file
  Run {
    /// Input file
    file: String,
  },
  /// Prints the address and subject of a secret key
  Subject {
    /// File containing the 256-bit secret key, as a hex string
    skey: String,
  },
  /// Prints all statements in a Kindelia (.kdl) file
  Print {
    /// File containing the statements to be printed
    file: String,
  },
  /// Serializes all statements in a Kindelia (.kdl) file
  Serialize {
    /// File containing the statements to be serialized
    file: String,
  },
  /// Deserializes a statement
  Deserialize {
    /// The statement to be deserialized, in hex
    hex: String,
  },
  /// Signs a serialized statement
  Sign {
    /// File containing the 256-bit secret key, as a hex string
    skey: String,
    /// The statement to be signed, in hex
    hex: String,
  },
  /// Posts a serialized statement to the network
  Post {
    /// The statement to be posted, in hex
    hex: String,
    /// IP of the node to submit it to
    addr: Option<String>,
  },
}

/// Gets the path where Kindelia files should be saved.
///
/// Priority is:
/// 1. CLI argument
/// 2. Environment variable
/// 3. Default path (`$HOME/.kindelia`)
fn get_kindelia_path(dir_cli: Option<String>) -> Result<PathBuf, String> {
  let default = || dirs::home_dir().unwrap().join(".kindelia");
  if let Some(dir_cli) = dir_cli {
    return Ok(PathBuf::from(dir_cli));
  }
  match std::env::var(KINDELIA_PATH_ENV_VAR) {
    Ok(dir) => Ok(PathBuf::from(dir)),
    Err(err) => {
      if let std::env::VarError::NotPresent = err {
        Ok(default())
      } else {
        Err(format!("{} environment variable is not valid: '{}'", KINDELIA_PATH_ENV_VAR, err))
      }
    }
  }
}

fn run_cli() -> Result<(), String> {
  let arguments = Cli::parse();
  let kindelia_path = get_kindelia_path(arguments.path)?;
  
  fn get_statement(hex: &str) -> Option<Statement> {
    return deserialized_statement(&bytes_to_bitvec(&hex::decode(hex).expect("hex string")));
  }

  match arguments.command {
    // Starts the node process
    CliCmd::Start { testnet, mine } => {
      eprintln!("Starting Kindelia node. Store path: {:?}", kindelia_path);
      start_node(kindelia_path, testnet, mine);
    }

    // Runs a single block, for testing
    CliCmd::Run { file } => {
      let file = std::fs::read_to_string(file);
      match file {
        Err(err) => {
          return Err(format!("{}", err));
        }
        Ok(code) => {
          // TODO: flag to disable size limit / debug
          hvm::test_statements_from_code(&code);
        }
      }
    }

    // Prints all statements in a file
    CliCmd::Print { file } => {
      if let Ok(code) = std::fs::read_to_string(file) {
        let statements = hvm::read_statements(&code).map_err(|err| err.erro)?.1;
        for statement in statements {
          println!("// {}", hex::encode(serialized_statement(&statement).to_bytes()));
          println!("{}", view_statement(&statement));
          println!("");
        }
      } else {
        println!("Couldn't load file.");
      }
    }

    // Serializes all statements in a file
    CliCmd::Serialize { file } => {
      if let Ok(code) = std::fs::read_to_string(file) {
        let statements = hvm::read_statements(&code).map_err(|err| err.erro)?.1;
        for statement in statements {
          println!("{}", hex::encode(serialized_statement(&statement).to_bytes()));
        }
      } else {
        println!("Couldn't load file.");
      }
    }

    // Deserializes a statement
    CliCmd::Deserialize { hex } => {
      if let Some(statement) = get_statement(&hex) {
        println!("{}", view_statement(&statement));
      }
    }

    // Signs a statement
    CliCmd::Sign { hex, skey: skey_file } => {
      if let Ok(skey) = std::fs::read_to_string(skey_file) {
        if let Some(statement) = get_statement(&hex) {
          let skey = hex::decode(&skey[0..64]).expect("hex string");
          let user = crypto::Account::from_private_key(&skey);
          let hash = hvm::hash_statement(&statement);
          let sign = user.sign(&hash);
          let stat = set_sign(&statement, sign);
          println!("{}", hex::encode(serialized_statement(&stat).to_bytes()));
          return Ok(());
        } else {
          println!("Hex provided isn't a serialized statement.");
        }
      } else {
        println!("Couldn't load term and secret key files.");
      }
    }

    // Posts a run statement
    CliCmd::Post { hex, addr: node_addr } => {
      if let Some(statement) = get_statement(&hex) {
        let tx = Transaction::new(bitvec_to_bytes(&serialized_statement(&statement)));
        let ms = Message::PleaseMineThisTransaction { trans: tx };
        let ports = [UDP_PORT + 100, UDP_PORT + 101, UDP_PORT + 102, UDP_PORT + 103];
        if let Some((mut socket, port)) = udp_init(&ports) {
          let addrs = if let Some(node_addr) = node_addr {
            vec![read_address(&node_addr)]
          } else {
            ENTRY_PEERS.iter().map(|x| read_address(x)).collect()
          };
          udp_send(&mut socket, addrs, &ms);
          println!("Published statement:\n\n{}", view_statement(&statement));
          return Ok(());
        } else {
          panic!("Couldn't open UDP socket on ports: {:?}.", ports);
        }
      } else {
        println!("Hex provided isn't a serialized statement.");
      }
    }

    // Prints the subject
    CliCmd::Subject { skey } => {
      if let Ok(skey) = std::fs::read_to_string(skey) {
        let skey = hex::decode(&skey[0..64]).expect("hex string");
        let acc  = crypto::Account::from_private_key(&skey);
        println!("Ethereum Address: {}", acc.address.show());
        println!("Kindelia Subject: {}", acc.name.show());
      } else {
        println!("Couldn't load term and secret key files.");
      }
    }
  };
  Ok(())
}

fn start_node(kindelia_path: PathBuf, testnet: bool, mine: bool) {
  // TODO: move out to config file
  let testnet_peers: Vec<Address> = ENTRY_PEERS.into_iter().map(node::read_address).collect();
  let init_peers = if testnet { Some(testnet_peers) } else { None };

  // Reads the file contents
  //let file = file.map(|file| std::fs::read_to_string(file).expect("Block file not found."));

  // Node state object
  let (node_query_sender, node) = Node::new(kindelia_path.clone(), &init_peers);

  // Node to Miner communication object
  let miner_comm_0 = MinerCommunication::new();
  let miner_comm_1 = miner_comm_0.clone();

  // API thread channel
  //let (api_send, api_recv) = mpsc::channel();

  // Threads
  let mut threads = vec![];

  // Spawns the node thread
  let node_thread = thread::spawn(move || {
    node.main(kindelia_path.clone(), miner_comm_0, mine);
  });
  threads.push(node_thread);

  // Spawns the miner thread
  if mine {
    let miner_thread = thread::spawn(move || {
      miner_loop(miner_comm_1);
    });
    threads.push(miner_thread);
  }

  // Spawns the API thread
  let api_thread = thread::spawn(move || {
    api_loop(node_query_sender);
  });
  threads.push(api_thread);

  // Joins all threads
  for thread in threads {
    thread.join().unwrap();
  }
}

// Post
// ----

// FIXME: sorry, seems like we changed this function at the same time. Let's discuss in call

// TODO: move out to client.rs?
//type PostResult = Result<(), String>;

//pub fn post_from_code(code: &str, address: &str) -> PostResult {
  //let statements = hvm::read_statements(&code).map_err(|err| err.erro)?;
  //let statements = statements.1;
  //if let Some(last_statement) = &statements.last() {
    //let tx = Transaction::new(bitvec_to_bytes(&serialized_statement(last_statement)));
    //let ms = Message::PleaseMineThisTransaction { trans: tx };
    //let ip = read_address(&address);
    //let ports = [UDP_PORT + 100, UDP_PORT + 101, UDP_PORT + 102, UDP_PORT + 103];
    //if let Some((mut socket, port)) = udp_init(&ports) {
      //udp_send(&mut socket, ip, &ms);
      //println!("Sent statement to {} via UDP:\n\n{}", address, view_statement(last_statement));
      //Ok(())
    //} else {
      //Err(format!("Couldn't open UDP socket on ports: {:?}.", ports))
    //}
  //} else {
    //return Err("No statement found in code.".to_string());
  //}
//}
