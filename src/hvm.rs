// Welcome to Kindelia's High-order Virtual Machine!
// =================================================
//
// This file is a modification of the project hosted on github.com/kindelia/hvm, and it makes a
// series of changes with the goal of serving the requirements of a peer-to-peer computer.
//
// Kindelia-HVM's memory model
// ---------------------------
// 
// The runtime memory consists of just a vector of u128 pointers. That is:
//
//   Mem ::= Vec<Ptr>
// 
// A pointer has 3 parts:
//
//   Ptr ::= TT AAAAAAAAAAAAAAAAAA BBBBBBBBBBBB
//
// Where:
//
//   T : u8  is the pointer tag 
//   A : u72 is the 1st value
//   B : u48 is the 2nd value
//
// There are 12 possible tags:
//
//   Tag | Val | Meaning  
//   ----| --- | -------------------------------
//   DP0 |   0 | a variable, bound to the 1st argument of a duplication
//   DP1 |   1 | a variable, bound to the 2nd argument of a duplication
//   VAR |   2 | a variable, bound to the one argument of a lambda
//   ARG |   3 | an used argument of a lambda or duplication
//   ERA |   4 | an erased argument of a lambda or duplication
//   LAM |   5 | a lambda
//   APP |   6 | an application
//   SUP |   7 | a superposition
//   CTR |   8 | a constructor
//   FUN |   9 | a function
//   OP2 |  10 | a numeric operation
//   NUM |  11 | a 120-bit number
//
// The semantics of the 1st and 2nd values depend on the pointer tag. 
//
//   Tag | 1st ptr value                | 2nd ptr value
//   --- | ---------------------------- | ---------------------------------
//   DP0 | the duplication label        | points to the duplication node
//   DP1 | the duplication label        | points to the duplication node
//   VAR | not used                     | points to the lambda node
//   ARG | not used                     | points to the variable occurrence
//   ERA | not used                     | not used
//   LAM | not used                     | points to the lambda node
//   APP | not used                     | points to the application node
//   SUP | the duplication label        | points to the superposition node
//   CTR | the constructor name         | points to the constructor node
//   FUN | the function name            | points to the function node
//   OP2 | the operation name           | points to the operation node
//   NUM | the most significant 72 bits | the least significant 48 bits
//
// Notes:
//
//   1. The duplication label is an internal value used on the DUP-SUP rule.
//   2. The operation name only uses 4 of the 72 bits, as there are only 16 ops.
//   3. NUM pointers don't point anywhere, they just store the number directly.
//
// A node is a tuple of N pointers stored on sequential memory indices.
// The meaning of each index depends on the node. There are 7 types:
//
//   Duplication Node:
//   - [0] => either an ERA or an ARG pointing to the 1st variable location
//   - [1] => either an ERA or an ARG pointing to the 2nd variable location
//   - [2] => pointer to the duplicated expression
//
//   Lambda Node:
//   - [0] => either and ERA or an ERA pointing to the variable location
//   - [1] => pointer to the lambda's body
//   
//   Application Node:
//   - [0] => pointer to the lambda
//   - [1] => pointer to the argument
//
//   Superposition Node:
//   - [0] => pointer to the 1st superposed value
//   - [1] => pointer to the 2sd superposed value
//
//   Constructor Node:
//   - [0] => pointer to the 1st field
//   - [1] => pointer to the 2nd field
//   - ... => ...
//   - [N] => pointer to the Nth field
//
//   Function Node:
//   - [0] => pointer to the 1st argument
//   - [1] => pointer to the 2nd argument
//   - ... => ...
//   - [N] => pointer to the Nth argument
//
//   Operation Node:
//   - [0] => pointer to the 1st operand
//   - [1] => pointer to the 2nd operand
//
// Notes:
//
//   1. Duplication nodes DON'T have a body. They "float" on the global scope.
//   2. Lambdas and Duplications point to their variables, and vice-versa.
//   3. ARG pointers can only show up inside Lambdas and Duplications.
//   4. Nums and vars don't require a node type, because they're unboxed.
//   5. Function and Constructor arities depends on the user-provided definition.
//
// Example 0:
// 
//   Term:
//
//    {T2 #7 #8}
//
//   Memory:
//
//     Root : Ptr(CTR, 0x0000000007b9d30a43, 0x000000000000)
//     0x00 | Ptr(NUM, 0x000000000000000000, 0x000000000007) // the tuple's 1st field
//     0x01 | Ptr(NUM, 0x000000000000000000, 0x000000000008) // the tuple's 2nd field
//
//   Notes:
//     
//     1. This is just a pair with two numbers.
//     2. The root pointer is not stored on memory.
//     3. The '0x0000000007b9d30a43' constant encodes the 'T2' name.
//     4. Since nums are unboxed, a 2-tuple uses 2 memory slots, or 32 bytes.
//
// Example 1:
//
//   Term:
//
//     λ~ λb b
//
//   Memory:
//
//     Root : Ptr(LAM, 0x000000000000000000, 0x000000000000)
//     0x00 | Ptr(ERA, 0x000000000000000000, 0x000000000000) // 1st lambda's argument
//     0x01 | Ptr(LAM, 0x000000000000000000, 0x000000000002) // 1st lambda's body
//     0x02 | Ptr(ARG, 0x000000000000000000, 0x000000000003) // 2nd lambda's argument
//     0x03 | Ptr(VAR, 0x000000000000000000, 0x000000000002) // 2nd lambda's body
//
//   Notes:
//
//     1. This is a λ-term that discards the 1st argument and returns the 2nd.
//     2. The 1st lambda's argument not used, thus, an ERA pointer.
//     3. The 2nd lambda's argument points to its variable, and vice-versa.
//     4. Each lambda uses 2 memory slots. This term uses 64 bytes in total.
//     
// Example 2:
//
//   Term:
//     
//     λx dup x0 x1 = x; (* x0 x1)
//
//   Memory:
//
//     Root : Ptr(LAM, 0x000000000000000000, 0x000000000000)
//     0x00 | Ptr(ARG, 0x000000000000000000, 0x000000000004) // the lambda's argument
//     0x01 | Ptr(OP2, 0x000000000000000002, 0x000000000005) // the lambda's body
//     0x02 | Ptr(ARG, 0x000000000000000000, 0x000000000005) // the duplication's 1st argument
//     0x03 | Ptr(ARG, 0x000000000000000000, 0x000000000006) // the duplication's 2nd argument
//     0x04 | Ptr(VAR, 0x000000000000000000, 0x000000000000) // the duplicated expression
//     0x05 | Ptr(DP0, 0x7b93e8d2b9ba31fb21, 0x000000000002) // the operator's 1st operand
//     0x06 | Ptr(DP1, 0x7b93e8d2b9ba31fb21, 0x000000000002) // the operator's 2st operand
//
//   Notes:
//     
//     1. This is a lambda function that squares a number.
//     2. Notice how every ARGs point to a VAR/DP0/DP1, that points back its source node.
//     3. DP1 does not point to its ARG. It points to the duplication node, which is at 0x02.
//     4. The lambda's body does not point to the dup node, but to the operator. Dup nodes float.
//     5. 0x7b93e8d2b9ba31fb21 is a globally unique random label assigned to the duplication node.
//     6. That duplication label is stored on the DP0/DP1 that point to the node, not on the node.
//     7. A lambda uses 2 memory slots, a duplication uses 3, an operator uses 2. Total: 112 bytes.
//     8. In-memory size is different to, and larger than, serialization size.
//
// How is Kindelia's HVM different from the conventional HVM?
// ----------------------------------------------------------
//
// First, it is a 128-bit, rather than a 64-bit architecture. It can store 120-bit unboxed
// integers, up from 32-bit unboxed uints stored by the conventional HVM. It allows addressing up
// to 2^72 function names, up from 2^30 allowed by the conventional HVM, which isn't enough for
// Kindelia. This change comes with a cost of about ~30% reduced performance, which is acceptable.
//
// Second, it implements a reversible heap machinery, which allows saving periodic snapshots of
// past heap states, and jump back to them. This is necessary because of decentralized consensus.
// If we couldn't revert to past states, we'd have to recompute the entire history anytime there is
// a block reorg, which isn't practical. On Ethereum, this is achieved by storing the state as a
// Map<U256> using Merkle Trees, which, being an immutable structure, allows non-destructive
// insertions and rollbacks. We could do the same, but we decided to further leverage the HVM by
// saving its whole heap as the network state. In other words, applications are allowed to persist
// arbitrary HVM structures on disk by using the io_save operation. For example:
//
//   (io_save {Cons #1 {Cons #2 {Cons #3 {Nil}}}} ...)
//
// The operation above would persist the [1,2,3] list as the app's state, with no need for
// serialization. As such, when the app stops running, that list will not be freed from memory.
// Instead, the heap will persist between blocks, so the app just needs to store a pointer to
// the list's head, allowing it to retrieve its state later on. This is only possible because the
// HVM is garbage-collection free, otherwise, leaks would overwhelm the memory.
//
// How are reversible heaps stored?
// --------------------------------
//
// Kindelia's heap is set to grow exactly 8 GB per year. In other words, 10 years after the genesis
// block, the heap size will be of exactly 80 GB. But that doesn't mean a full node will be able
// to operate with even that much ram, because Kindelia must also save snapshots. Right now, it
// stores at most 10 snapshots, trying to keep them distributed with exponentially decreasing ages.
// For example, if we're on block 1000, it might store a snapshot of blocks 998, 996, 992, 984,
// 968, 872, 744 and 488, which is compatible with the fact that longer-term rollbacks are
// increasingly unlikely. If there is a rollback to block 990, we just go back to the earliest
// snapshot, 984, and reprocess blocks 985-1000, which is much faster than recomputing the entire
// history.
//
// In order to keep a good set of snapshots, we must be able to create and discard these heaps.
// Obviously, if this operation required copying the entire heap buffer every block, it would
// completely destroy the network's performance. As such, instead, heaps only actually store
// data that changed. So, using the example above, if a list was allocated and persisted on block 980,
// it will actually be stored on the snapshot 984, which is the earliest snapshot after 980. If the
// runtime, now on block 1000, attempts to read the memory where the list is allocated, it will
// actually receive a signal that it is stored on a past heap, and look for it on 996 and 992,
// until it is found on block 984.
//
// To achieve that, hashmaps are used to store defined functions and persistent state pointers. If
// a key isn't present, Kindelia will look for it on past snapshots. As for the runtime's memory,
// where HVM constructors and lambdas are stored, it doesn't use a hashmap. Instead, it uses a
// Nodes type, which stores data in a big pre-allocated u128 buffer, and keeps track of used memory
// slots in a separate buffer. We then reserve a constant, U128_NONE, to signal that an index isn't
// present, and must be found ina  past heap. This is different from 0, which means that this index
// is empty, and can be allocated. This allows for fast write, read, disposal and merging of heaps,
// but comes at the cost of wasting a lot of memory. Because of that, Kindelia's current
// implementation demands up to ~10x more available memory than the current heap size, but that
// could be reduced ten-fold by replacing vectors by a hashmap, or by just saving fewer past heaps.
//
// Other than a 128-bit architecture and reversible heaps, Kindelia's HVM is similar to the
// conventional HVM. This file will be extensively commented, with in-depth explanations of every
// little aspect, from the HVM's memory model to interaction net rewrite rules.

#![allow(clippy::identity_op)]

use std::collections::{hash_map, HashMap, HashSet};
use std::fmt::Write;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::NoHashHasher as NHH;

use crate::bits;
use crate::crypto;
use crate::dbg_println;
use crate::util::U128_SIZE;
use crate::util;

// Types
// -----

// This is the HVM's term type. It is used to represent an expression. It is not used in rewrite
// rules. Instead, it is stored on HVM's heap using its memory model, which will be elaborated
// later on. Below is a description of each variant:
// - Var: variable. Note an u128 is used instead of a string. It stores up to 20 6-bit letters.
// - Dup: a lazy duplication of any other term. Written as: `dup a b = term; body`
// - Lam: an affine lambda. Written as: `λvar body`.
// - App: a lambda application. Written as: `(f x)`.
// - Ctr: a constructor. Written as: `{Ctr val0 val1 ...}`
// - Fun: a function call. Written as: `(Fun arg0 arg1 ...)`
// - Num: an unsigned integer. Note that an u128 is used, but it is actually 120 bits long.
// - Op2: a numeric operation.
/// A native HVM term
#[derive(Clone, Debug, PartialEq)]
pub enum Term {
  Var { name: u128 },
  Dup { nam0: u128, nam1: u128, expr: Box<Term>, body: Box<Term> },
  Lam { name: u128, body: Box<Term> },
  App { func: Box<Term>, argm: Box<Term> },
  Ctr { name: u128, args: Vec<Term> },
  Fun { name: u128, args: Vec<Term> },
  Num { numb: u128 },
  Op2 { oper: u128, val0: Box<Term>, val1: Box<Term> },
}

// A native HVM 120-bit machine integer operation
// - Add: addition
// - Sub: subtraction
// - Mul: multiplication
// - Div: division
// - Mod: modulo
// - And: bitwise and
// - Or : bitwise or
// - Xor: bitwise xor
// - Shl: shift left
// - Shr: shift right
// - Ltn: less than
// - Lte: less than or equal
// - Eql: equal
// - Gte: greater than or equal
// - Gtn: greater than
// - Neq: not equal
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Oper {
  Add, Sub, Mul, Div,
  Mod, And, Or,  Xor,
  Shl, Shr, Ltn, Lte,
  Eql, Gte, Gtn, Neq,
}

// A u64 HashMap
pub type Map<T> = util::U128Map<T>;

/// A rewrite rule, or equation, in the shape of `left_hand_side = right_hand_side`.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
  pub lhs: Term,
  pub rhs: Term,
}

/// A function, which is just a vector of rewrite rules.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Func {
  pub rules: Vec<Rule>,
}

// The types below are used by the runtime to evaluate rewrite rules. They store the same data as
// the type aboves, except in a semi-compiled, digested form, allowing faster computation.

// Compiled information about a left-hand side variable.
#[derive(Clone, Debug, PartialEq)]
pub struct Var {
  pub name : u128,         // this variable's name
  pub param: u128,         // in what parameter is this variable located?
  pub field: Option<u128>, // in what field is this variable located? (if any)
  pub erase: bool,         // should this variable be collected (because it is unused)?
}

// Compiled information about a rewrite rule.
#[derive(Clone, Debug, PartialEq)]
pub struct CompRule {
  pub cond: Vec<Ptr>,          // left-hand side matching conditions
  pub vars: Vec<Var>,          // left-hand side variable locations
  pub eras: Vec<(u128, u128)>, // must-clear locations (argument number and arity)
  pub body: Term,              // right-hand side body of rule
}

// Compiled information about a function.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CompFunc {
  pub func: Func,           // the original function
  pub arity: u128,          // number of arguments
  pub redux: Vec<u128>,     // index of strict arguments
  pub rules: Vec<CompRule>, // vector of rules
}

// A file, which is just a map of `FuncID -> CompFunc`
// It is used to find a function when it is called, in order to apply its rewrite rules.
#[derive(Clone, Debug)]
pub struct Funcs {
  pub funcs: Map<Arc<CompFunc>>,
}

// A map of `FuncID -> Arity`
// It is used in many places to find the arity (argument count) of functions and constructors.
#[derive(Clone, Debug)]
pub struct Arits {
  pub arits: Map<u128>,
}

// A map of `FuncID -> FuncID
// Stores the owner of the 'FuncID' a namespace.
#[derive(Clone, Debug)]
pub struct Ownrs {
  pub ownrs: Map<u128>,
}

// A map of `FuncID -> Ptr`
// It links a function id to its state on the runtime memory.
#[derive(Clone, Debug)]
pub struct Store {
  pub links: Map<Ptr>,
}

/// A global statement that alters the state of the blockchain
#[derive(Debug, PartialEq)]
pub enum Statement {
  Fun { name: u128, args: Vec<u128>, func: Func, init: Term, sign: Option<crypto::Signature> },
  Ctr { name: u128, args: Vec<u128>, sign: Option<crypto::Signature> },
  Run { expr: Term, sign: Option<crypto::Signature> },
  Reg { name: u128, ownr: u128, sign: Option<crypto::Signature> },
}

// An HVM pointer. It can point to an HVM node, a variable, or store an unboxed u120.
pub type Ptr = u128;

// A mergeable vector of u128 values
#[derive(Debug, Clone)]
pub struct Nodes {
  pub nodes: Map<u128>,
}

// HVM's memory state (nodes, functions, metadata, statistics)
#[derive(Debug, Clone)]
pub struct Heap {
  pub uuid: u128,  // unique identifier
  pub memo: Nodes, // memory block holding HVM nodes
  pub disk: Store, // points to stored function states
  pub file: Funcs, // function codes
  pub arit: Arits, // function arities
  pub ownr: Ownrs, // namespace owners
  pub tick: u128,  // tick counter
  pub time: u128,  // block timestamp
  pub meta: u128,  // block metadata
  pub hax0: u128,  // block hash, part 0
  pub hax1: u128,  // block hash, part 1
  pub funs: u128,  // total function count
  pub dups: u128,  // total dups count
  pub rwts: u128,  // total graph rewrites
  pub mana: u128,  // total mana cost
  pub size: i128,  // total used memory (in 64-bit words)
  pub mcap: u128,  // memory capacity (in 64-bit words)
  pub next: u128,  // memory index that *may* be empty
}

#[derive(Debug, Clone)]
// A serialized Heap
pub struct SerializedHeap {
  pub uuid: u128,
  pub memo: Vec<u128>,
  pub disk: Vec<u128>,
  pub file: Vec<u128>,
  pub arit: Vec<u128>,
  pub ownr: Vec<u128>,
  pub nums: Vec<u128>,
  pub stat: Vec<u128>,
}

// A list of past heap states, for block-reorg rollback
// FIXME: this should be replaced by a much simpler index array
#[derive(Debug, Clone)]
pub enum Rollback {
  Cons {
    keep: u64,
    life: u64,
    head: u64,
    tail: Arc<Rollback>,
  },
  Nil,
}

// The current and past states
pub struct Runtime {
  heap: Vec<Heap>,      // heap objects
  draw: u64,            // drawing heap index
  curr: u64,            // current heap index
  nuls: Vec<u64>,       // reuse heap indices
  back: Arc<Rollback>,  // past states
  path: PathBuf,        // where to save runtime state
}

#[derive(Debug, Copy, Clone)]
pub enum RuntimeError {
  NotEnoughMana,
  NotEnoughSpace,
  TypeMismatch,
  EffectFailure,
}

//pub fn heaps_invariant(rt: &Runtime) -> (bool, Vec<u8>, Vec<u64>) {
  //let mut seen = vec![0u8; 10];
  //let mut heaps = vec![0u64; 0];
  //let mut push = |id: u64| {
    //let idx = id as usize;
    //seen[idx] += 1;
    //heaps.push(id);
  //};
  //push(rt.draw);
  //push(rt.curr);
  //for nul in &rt.nuls {
    //push(*nul);
  //}
  //{
    //let mut back = &*rt.back;
    //while let Rollback::Cons { keep, life, head, tail } = back {
      //push(*head);
      //back = &*tail;
    //}
  //}
  //let failed = seen.iter().all(|c| *c == 1);
  //(failed, seen, heaps)
//}

pub type StatementResult = Result<StatementInfo, StatementErr>;

#[derive(Debug, Clone)]
pub enum StatementInfo {
  Ctr { name: u128, args: Vec<u128> },
  Fun { name: u128, args: Vec<u128> },
  Run { done_term: Term, used_mana: u128, size_diff: i128, end_size: u128 },
  Reg { name: u128, ownr: u128 },
}

#[derive(Debug, Clone)]
pub struct StatementErr {
  pub err: String,
}

pub type ParseResult<'a, A> = Result<(&'a str, A), ParseErr>;

#[derive(Debug, Clone)]
pub struct ParseErr {
  pub code: String,
  pub erro: String,
}

// Constants
// ---------

const U128_PER_KB: u128 = (1024 / U128_SIZE) as u128;
const U128_PER_MB: u128 = U128_PER_KB << 10;
const U128_PER_GB: u128 = U128_PER_MB << 10;

// With the constants below, we alloc 4 GB per heap, which holds for the first 6 months. We
// pre-alloc 6 heaps, which is enough for 4 snapshots: 16 seconds old, 4 minutes old, 1 hour old
// and 1 day old, on average. That requires 24 GB RAM. As such, 32 GB RAM is needed to host a full
// node, on the first 6 months. After that, we must increase the HEAP_SIZE to 8 GB, which will
// demand 64 GB RAM, increasing by an additional 64 GB RAM every year. Note that most of this is
// empty space, so, future optimizations should reduce this to closer to the actual 8 GB per year
// that the network actually uses.

//#[cfg(not(debug_assertions))]
//const HEAP_SIZE: u128 = 4096 * U128_PER_MB; // total size per heap, in 128-bit words
const MAX_HEAPS: u64 = 6; // total heaps to pre-alloc (2 are used for draw/curr, rest for rollbacks)
const MAX_ROLLBACK: u64 = MAX_HEAPS - 2; // total heaps to pre-alloc for snapshots

// Use smaller heaps for debug/development builds
//#[cfg(debug_assertions)]
//const HEAP_SIZE: u128 = 64 * U128_PER_MB; // total size per heap, in 128-bit words

pub const MAX_TERM_DEPTH: u128 = 256; // maximum depth of a LHS or RHS term

pub const VAL: u128 = 1 << 0;
pub const EXT: u128 = 1 << 48;
pub const TAG: u128 = 1 << 120;

// FIXME: what were these used for? can this be explained if uncommented?
//pub const VAL_MASK: u128 = EXT - 1;
//pub const EXT_MASK: u128 = (TAG - 1)   ^ VAL_MASK;
//pub const TAG_MASK: u128 = (u128::MAX) ^ EXT_MASK;
//pub const NUM_MASK: u128 = EXT_MASK | VAL_MASK;
pub const NUM_MASK: u128 = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;

pub const DP0: u128 = 0x0;
pub const DP1: u128 = 0x1;
pub const VAR: u128 = 0x2;
pub const ARG: u128 = 0x3;
pub const ERA: u128 = 0x4;
pub const LAM: u128 = 0x5;
pub const APP: u128 = 0x6;
pub const SUP: u128 = 0x7;
pub const CTR: u128 = 0x8;
pub const FUN: u128 = 0x9;
pub const OP2: u128 = 0xA;
pub const NUM: u128 = 0xB;
pub const NIL: u128 = 0xF;

pub const ADD : u128 = 0x0;
pub const SUB : u128 = 0x1;
pub const MUL : u128 = 0x2;
pub const DIV : u128 = 0x3;
pub const MOD : u128 = 0x4;
pub const AND : u128 = 0x5;
pub const OR  : u128 = 0x6;
pub const XOR : u128 = 0x7;
pub const SHL : u128 = 0x8;
pub const SHR : u128 = 0x9;
pub const LTN : u128 = 0xA;
pub const LTE : u128 = 0xB;
pub const EQL : u128 = 0xC;
pub const GTE : u128 = 0xD;
pub const GTN : u128 = 0xE;
pub const NEQ : u128 = 0xF;

pub const VAR_NONE  : u128 = 0x3FFFF;
pub const U128_NONE : u128 = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
pub const I128_NONE : i128 = -0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;

// (IO r:Type) : Type
//   (DONE expr)           : (IO r)
//   (TAKE           then) : (IO r)
//   (SAVE expr      then) : (IO r)
//   (CALL expr args then) : (IO r)
//   (SUBJ           then) : (IO r)
//   (FROM           then) : (IO r)
//   (TICK           then) : (IO r)
//   (TIME           then) : (IO r)
const IO_DONE : u128 = 0x39960f; // name_to_u128("DONE")
const IO_TAKE : u128 = 0x78b54f; // name_to_u128("TAKE")
const IO_SAVE : u128 = 0x74b80f; // name_to_u128("SAVE")
const IO_CALL : u128 = 0x34b596; // name_to_u128("CALL")
const IO_SUBJ : u128 = 0x75f314; // name_to_u128("SUBJ")
const IO_FROM : u128 = 0x41c657; // name_to_u128("FROM")
const IO_LOAD : u128 = 0x5992ce; // name_to_u128("LOAD")
const IO_TICK : u128 = 0x793355; // name_to_u128("TICK")
const IO_TIME : u128 = 0x7935cf; // name_to_u128("TIME")
const IO_META : u128 = 0x5cf78b; // name_to_u128("META")
const IO_HAX0 : u128 = 0x48b881; // name_to_u128("HAX0")
const IO_HAX1 : u128 = 0x48b882; // name_to_u128("HAX1")

// Maximum mana that can be spent in a block
pub const BLOCK_MANA_LIMIT : u128 = 4_000_000;

// Maximum state growth per block, in bits
pub const BLOCK_BITS_LIMIT : i128 = 2048; // 1024 bits per sec = about 8 GB per year

// Mana Table
// ----------

// |-----------|---------------------------------|-------|
// | Opcode    | Effect                          | Mana  |
// |-----------|---------------------------------|-------|
// | APP-LAM   | applies a lambda                | 2     |
// | APP-SUP   | applies a superposition         | 4     |
// | OP2-NUM   | operates on a number            | 2     |
// | OP2-SUP   | operates on a superposition     | 4     |
// | FUN-CTR   | pattern-matches a constructor   | 2 + M |
// | FUN-SUP   | pattern-matches a superposition | 2 + A |
// | DUP-LAM   | clones a lambda                 | 4     |
// | DUP-NUM   | clones a number                 | 2     |
// | DUP-CTR   | clones a constructor            | 2 + A |
// | DUP-SUP-D | clones a superposition          | 4     |
// | DUP-SUP-E | undoes a superposition          | 2     |
// | DUP-ERA   | clones an erasure               | 2     |
// |-----------------------------------------------------|
// | * A is the constructor or function arity            |
// | * M is the alloc count of the right-hand side       |
// |-----------------------------------------------------|


fn AppLamMana() -> u128 {
  return 2;
}

fn AppSupMana() -> u128 {
  return 4;
}

fn Op2NumMana() -> u128 {
  return 2;
}

fn Op2SupMana() -> u128 {
  return 4;
}

fn FunCtrMana(body: &Term) -> u128 {
  return 2 + count_allocs(body);
}

fn FunSupMana(arity: u128) -> u128 {
  return 2 + arity;
}

fn DupLamMana() -> u128 {
  return 4;
}

fn DupNumMana() -> u128 {
  return 2;
}

fn DupCtrMana(arity: u128) -> u128 {
  return 2 + arity;
}

fn DupDupMana() -> u128 {
  return 4;
}

fn DupSupMana() -> u128 {
  return 2;
}

fn DupEraMana() -> u128 {
  return 2;
}

fn count_allocs(body: &Term) -> u128 {
  match body {
    Term::Var { name } => {
      0
    }
    Term::Dup { nam0, nam1, expr, body } => {
      let expr = count_allocs(expr);
      let body = count_allocs(body);
      3 + expr + body
    }
    Term::Lam { name, body } => {
      let body = count_allocs(body);
      2 + body
    }
    Term::App { func, argm } => {
      let func = count_allocs(func);
      let argm = count_allocs(argm);
      2 + func + argm
    }
    Term::Fun { name, args } => {
      let size = args.len() as u128;
      let mut count = 0;
      for (i, arg) in args.iter().enumerate() {
        count += count_allocs(arg);
      }
      size + count
    }
    Term::Ctr { name, args } => {
      let size = args.len() as u128;
      let mut count = 0;
      for (i, arg) in args.iter().enumerate() {
        count += count_allocs(arg);
      }
      size + count
    }
    Term::Num { numb } => {
      0
    }
    Term::Op2 { oper, val0, val1 } => {
      let val0 = count_allocs(val0);
      let val1 = count_allocs(val1);
      2 + val0 + val1
    }
  }
}

const GENESIS : &str = "
// T types
ctr {T0}
ctr {T1 x0}
ctr {T2 x0 x1}
ctr {T3 x0 x1 x2}
ctr {T4 x0 x1 x2 x3}
ctr {T5 x0 x1 x2 x3 x4}
ctr {T6 x0 x1 x2 x3 x4 x5}
ctr {T7 x0 x1 x2 x3 x4 x5 x6}
ctr {T8 x0 x1 x2 x3 x4 x5 x6 x7}
ctr {T9 x0 x1 x2 x3 x4 x5 x6 x7 x8}
ctr {TA x0 x1 x2 x3 x4 x5 x6 x7 x8 x9}
ctr {TB x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 x10}
ctr {TC x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 x10 x11}
ctr {TD x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 x10 x11 x12}
ctr {TE x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 x10 x11 x12 x13}
ctr {TF x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 x10 x11 x12 x13 x14}
ctr {TG x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 x10 x11 x12 x13 x14 x15}

// Used to pretty-print names
ctr {Name name}

// Below, we declare the built-in IO operations

// DONE returns from an IO operation
ctr {DONE expr}
fun (Done expr) {
  (Done expr) = {DONE expr}
}

// TAKE recovers an app's stored state
ctr {TAKE cont}
fun (Take) {
  (Take) = @cont {TAKE cont}
}

// SAVE stores the app's state
ctr {SAVE expr cont}
fun (Save expr) {
  (Save expr) = @cont {SAVE expr cont}
}

// CALL calls another IO operation, assigning
// the caller name to the current subject name
ctr {CALL name args cont}
fun (Call name args) {
  (Call name args) = @cont {CALL name args cont}
}

// SUBJ returns the name of the current subject
ctr {SUBJ cont}
fun (Subj) {
  (Subj) = @cont {SUBJ cont}
}

// FROM returns the name of the current caller
ctr {FROM cont} 
fun (From) {
  (From) = @cont {FROM cont}
}

// TICK returns the current block number
ctr {TICK cont}
fun (Tick) {
  (Tick) = @cont {TICK cont}
}

// TIME returns the current block timestamp
ctr {TIME cont}
fun (Time) {
  (Time) = @cont {TIME cont}
}

// META returns the current block metadata
ctr {META cont}
fun (Meta) {
  (Meta) = @cont {META cont}
}

// HAX0 returns the current block metadata
ctr {HAX0 cont}
fun (Hax0) {
  (Hax0) = @cont {HAX0 cont}
}

// HAX1 returns the current block metadata
ctr {HAX1 cont}
fun (Hax1) {
  (Hax1) = @cont {HAX1 cont}
}

// LOAD works like TAKE, but clones the state
fun (Load) {
  (Load) = @cont {TAKE @x dup x0 x1 = x; {SAVE x0 @~ (cont x1)}}
}

// This is here for debugging. Will be removed.
ctr {Inc}
ctr {Get}
fun (Count action) {
  (Count {Inc}) = {TAKE @x {SAVE (+ x #1) @~ {DONE #0}}}
  (Count {Get}) = ((Load) @x {DONE x})
}

// Registers the empty namespace.
reg {
  #x7e5f4552091a69125d5dfcb7b8c265 // secret_key = 0x1
}
";

// Utils
// -----

pub fn init_map<A>() -> Map<A> {
  HashMap::with_hasher(BuildHasherDefault::default())
}

// Names
// -----

pub fn split_names(name: u128) -> Vec<String> {
  u128_to_name(name).split('.').map(|x| x.to_string()).collect()
}

pub fn get_namespace(name: u128) -> Option<u128> {
  let names = split_names(name);
  if names.len() > 1 {
    return Some(name_to_u128(&names[0 .. names.len() - 1].join(".")));
  } else {
    return None;
  }
}

// Statements
// ----------

// Removes the signature from a statement
pub fn remove_sign(statement: &Statement) -> Statement {
  match statement {
    Statement::Fun { name, args, func, init, sign } => {
      Statement::Fun {
        name: *name,
        args: args.clone(),
        func: func.clone(),
        init: init.clone(),
        sign: None,
      }
    }
    Statement::Ctr { name, args, sign } => {
      Statement::Ctr {
        name: *name,
        args: args.clone(),
        sign: None,
      }
    }
    Statement::Run { expr, sign } => {
      Statement::Run {
        expr: expr.clone(),
        sign: None,
      }
    }
    Statement::Reg { name, ownr, sign } => {
      Statement::Reg {
        name: *name,
        ownr: *ownr,
        sign: None,
      }
    }
  }
}

pub fn set_sign(statement: &Statement, new_sign: crypto::Signature) -> Statement {
  match statement {
    Statement::Fun { name, args, func, init, sign } => {
      Statement::Fun {
        name: *name,
        args: args.clone(),
        func: func.clone(),
        init: init.clone(),
        sign: Some(new_sign),
      }
    }
    Statement::Ctr { name, args, sign } => {
      Statement::Ctr {
        name: *name,
        args: args.clone(),
        sign: Some(new_sign),
      }
    }
    Statement::Run { expr, sign } => {
      Statement::Run {
        expr: expr.clone(),
        sign: Some(new_sign),
      }
    }
    Statement::Reg { name, ownr, sign } => {
      Statement::Reg {
        name: *name,
        ownr: *ownr,
        sign: Some(new_sign),
      }
    }
  }
}

// Rollback
// --------

fn absorb_u128(a: u128, b: u128, overwrite: bool) -> u128 {
  if b == U128_NONE { a } else if overwrite || a == U128_NONE { b } else { a }
}

fn absorb_i128(a: i128, b: i128, overwrite: bool) -> i128 {
  if b == I128_NONE { a } else if overwrite || a == I128_NONE { b } else { a }
}

impl Heap {
  fn write(&mut self, idx: u128, val: u128) {
    return self.memo.write(idx, val);
  }
  fn read(&self, idx: u128) -> u128 {
    return self.memo.read(idx);
  }
  fn write_disk(&mut self, fid: u128, val: Ptr) {
    return self.disk.write(fid, val);
  }
  fn read_disk(&self, fid: u128) -> Option<Ptr> {
    return self.disk.read(fid);
  }
  fn write_file(&mut self, fid: u128, fun: Arc<CompFunc>) {
    return self.file.write(fid, fun);
  }
  fn read_file(&self, fid: u128) -> Option<Arc<CompFunc>> {
    return self.file.read(fid);
  }
  fn write_arit(&mut self, fid: u128, val: u128) {
    return self.arit.write(fid, val);
  }
  fn read_arit(&self, fid: u128) -> Option<u128> {
    return self.arit.read(fid);
  }
  fn write_ownr(&mut self, fid: u128, val: u128) {
    return self.ownr.write(fid, val);
  }
  fn read_ownr(&self, fid: u128) -> Option<u128> {
    return self.ownr.read(fid);
  }
  fn set_tick(&mut self, tick: u128) {
    self.tick = tick;
  }
  fn get_tick(&self) -> u128 {
    return self.tick;
  }
  fn set_time(&mut self, time: u128) {
    self.time = time;
  }
  fn get_time(&self) -> u128 {
    return self.time;
  }
  fn set_meta(&mut self, meta: u128) {
    self.meta = meta;
  }
  fn get_meta(&self) -> u128 {
    return self.meta;
  }
  fn set_hax0(&mut self, meta: u128) {
    self.hax0 = meta;
  }
  fn get_hax0(&self) -> u128 {
    return self.hax0;
  }
  fn set_hax1(&mut self, meta: u128) {
    self.hax1 = meta;
  }
  fn get_hax1(&self) -> u128 {
    return self.hax1;
  }
  fn set_funs(&mut self, funs: u128) {
    self.funs = funs;
  }
  fn get_funs(&self) -> u128 {
    return self.funs;
  }
  fn set_dups(&mut self, dups: u128) {
    self.dups = dups;
  }
  fn get_dups(&self) -> u128 {
    return self.dups;
  }
  fn set_rwts(&mut self, rwts: u128) {
    self.rwts = rwts;
  }
  fn get_rwts(&self) -> u128 {
    return self.rwts;
  }
  fn set_mana(&mut self, mana: u128) { // TODO: do these counters need to be u128? would u64 suffice?
    self.mana = mana;
  }
  fn get_mana(&self) -> u128 {
    return self.mana;
  }
  fn set_size(&mut self, size: i128) {
    self.size = size;
  }
  fn get_size(&self) -> i128 {
    return self.size;
  }
  fn set_mcap(&mut self, mcap: u128) {
    self.mcap = mcap;
  }
  fn get_mcap(&self) -> u128 {
    return self.mcap;
  }
  fn set_next(&mut self, next: u128) {
    self.next = next;
  }
  fn get_next(&self) -> u128 {
    return self.next;
  }
  fn absorb(&mut self, other: &mut Self, overwrite: bool) {
    self.memo.absorb(&mut other.memo, overwrite);
    self.disk.absorb(&mut other.disk, overwrite);
    self.file.absorb(&mut other.file, overwrite);
    self.arit.absorb(&mut other.arit, overwrite);
    self.tick = absorb_u128(self.tick, other.tick, overwrite);
    self.time = absorb_u128(self.time, other.time, overwrite);
    self.meta = absorb_u128(self.meta, other.meta, overwrite);
    self.hax0 = absorb_u128(self.hax0, other.hax0, overwrite);
    self.hax1 = absorb_u128(self.hax1, other.hax1, overwrite);
    self.funs = absorb_u128(self.funs, other.funs, overwrite);
    self.dups = absorb_u128(self.dups, other.dups, overwrite);
    self.rwts = absorb_u128(self.rwts, other.rwts, overwrite);
    self.mana = absorb_u128(self.mana, other.mana, overwrite);
    self.size = absorb_i128(self.size, other.size, overwrite);
    self.mcap = absorb_u128(self.mcap, other.mcap, overwrite);
    self.next = absorb_u128(self.next, other.next, overwrite);
  }
  fn clear(&mut self) {
    self.uuid = fastrand::u128(..);
    self.memo.clear();
    self.disk.clear();
    self.file.clear();
    self.arit.clear();
    self.tick = U128_NONE;
    self.time = U128_NONE;
    self.meta = U128_NONE;
    self.hax0 = U128_NONE;
    self.hax1 = U128_NONE;
    self.funs = U128_NONE;
    self.dups = U128_NONE;
    self.rwts = U128_NONE;
    self.mana = U128_NONE;
    self.size = I128_NONE;
    self.mcap = U128_NONE;
    self.next = U128_NONE;
  }
  pub fn serialize(&self) -> SerializedHeap {
    // Serializes stat and size
    let size = self.size as u128;
    let stat = vec![self.tick, self.time, self.meta, self.hax0, self.hax1, self.funs, self.dups, self.rwts, self.mana, size, self.mcap, self.next];
    // Serializes Nodes
    let mut memo_buff : Vec<u128> = vec![];
    for (idx, val) in &self.memo.nodes {
      memo_buff.push(*idx as u128);
      memo_buff.push(*val as u128);
    }
    // Serializes Store
    let mut disk_buff : Vec<u128> = vec![];
    for (fnid, lnk) in &self.disk.links {
      disk_buff.push(*fnid as u128);
      disk_buff.push(*lnk as u128);
    }
    // Serializes Funcs
    let mut file_buff : Vec<u128> = vec![];
    for (fnid, func) in &self.file.funcs {
      let mut func_buff = util::u8s_to_u128s(&mut bits::serialized_func(&func.func).to_bytes());
      file_buff.push(*fnid as u128);
      file_buff.push(func_buff.len() as u128);
      file_buff.append(&mut func_buff);
    }
    // Serializes Arits
    let mut arit_buff : Vec<u128> = vec![];
    for (fnid, arit) in &self.arit.arits {
      arit_buff.push(*fnid as u128);
      arit_buff.push(*arit);
    }
    // Serializes Ownrs
    let mut ownr_buff : Vec<u128> = vec![];
    for (fnid, ownr) in &self.ownr.ownrs {
      ownr_buff.push(*fnid as u128);
      ownr_buff.push(*ownr);
    }
    // Serializes Nums
    let nums_buff : Vec<u128> = vec![
      self.tick,
      self.time,
      self.meta,
      self.hax0,
      self.hax1,
      self.funs,
      self.dups,
      self.rwts,
      self.mana,
      self.size as u128,
      self.mcap,
      self.next
    ];
    // Returns the serialized heap
    return SerializedHeap {
      uuid: self.uuid,
      memo: memo_buff,
      disk: disk_buff,
      file: file_buff,
      arit: arit_buff,
      ownr: ownr_buff,
      nums: nums_buff,
      stat,
    };
  }
  pub fn deserialize(&mut self, serial: &SerializedHeap) {
    // Deserializes stat and size
    self.tick = serial.nums[0];
    self.time = serial.nums[1];
    self.meta = serial.nums[2];
    self.hax0 = serial.nums[3];
    self.hax1 = serial.nums[4];
    self.funs = serial.nums[5];
    self.dups = serial.nums[6];
    self.rwts = serial.nums[7];
    self.mana = serial.nums[8];
    self.size = serial.nums[9] as i128;
    self.mcap = serial.nums[10];
    self.next = serial.nums[11];

    // Deserializes Nodes
    let mut i = 0;
    while i < serial.memo.len() {
      let idx = serial.memo[i + 0];
      let val = serial.memo[i + 1];
      self.write(idx, val);
      i += 2;
    }
    // Deserializes Store
    let mut i = 0;
    while i < serial.disk.len() {
      let fnid = serial.disk[i + 0];
      let lnk  = serial.disk[i + 1];
      self.write_disk(fnid, lnk);
      i += 2;
    }
    // Deserializes Funcs
    let mut i = 0;
    while i < serial.file.len() {
      let fnid = serial.file[i + 0];
      let size = serial.file[i + 1];
      let buff = &serial.file[i + 2 .. i + 2 + size as usize];
      let func = &bits::deserialized_func(&bit_vec::BitVec::from_bytes(&util::u128s_to_u8s(&buff))).unwrap();
      let func = compile_func(func, false).unwrap();
      self.write_file(fnid, Arc::new(func));
      i = i + 2 + size as usize;
    }
    // Deserializes Arits
    for i in 0 .. serial.arit.len() / 2 {
      let fnid = serial.arit[i * 2 + 0];
      let arit = serial.arit[i * 2 + 1];
      self.write_arit(fnid, arit);
    }
    // Deserializes Ownrs
    for i in 0 .. serial.ownr.len() / 2 {
      let fnid = serial.ownr[i * 2 + 0];
      let ownr = serial.ownr[i * 2 + 1];
      self.write_ownr(fnid, ownr);
    }
  }
  fn buffer_file_path(&self, uuid: u128, buffer_name: &str, path: &PathBuf) -> PathBuf {
    path.join(format!("{:0>32x}.{}.bin", uuid, buffer_name))
  }
  fn write_buffer(&self, uuid: u128, buffer_name: &str, buffer: &[u128], append: bool, path: &PathBuf) -> std::io::Result<()> {
    use std::io::Write;
    std::fs::OpenOptions::new()
      .write(true)
      .append(append)
      .create(true)
      .open(self.buffer_file_path(self.uuid, buffer_name, path))?
      .write_all(&util::u128s_to_u8s(buffer))?;
    return Ok(());
  }
  fn read_buffer(&self, uuid: u128, buffer_name: &str, path: &PathBuf) -> std::io::Result<Vec<u128>> {
    std::fs::read(self.buffer_file_path(uuid, buffer_name, path)).map(|x| util::u8s_to_u128s(&x))
  }
  fn delete_buffer(&self, uuid: u128, buffer_name: &str, path: &PathBuf) -> std::io::Result<()> {
    std::fs::remove_file(self.buffer_file_path(uuid, buffer_name, path))
  }
  pub fn save_buffers(&self, path: &PathBuf) -> std::io::Result<()> {
    self.append_buffers(self.uuid, path)
  }
  fn append_buffers(&self, uuid: u128, path: &PathBuf) -> std::io::Result<()> {
    let serial = self.serialize();
    self.write_buffer(serial.uuid, "memo", &serial.memo, true, path)?;
    self.write_buffer(serial.uuid, "disk", &serial.disk, true, path)?;
    self.write_buffer(serial.uuid, "file", &serial.file, true, path)?;
    self.write_buffer(serial.uuid, "arit", &serial.arit, true, path)?;
    self.write_buffer(serial.uuid, "ownr", &serial.ownr, true, path)?;
    self.write_buffer(serial.uuid, "nums", &serial.nums, true, path)?;
    self.write_buffer(serial.uuid, "stat", &serial.stat, false, path)?;
    return Ok(());
  }
  pub fn load_buffers(&mut self, uuid: u128, path: &PathBuf) -> std::io::Result<()> {
    let memo = self.read_buffer(uuid, "memo", path)?;
    let disk = self.read_buffer(uuid, "disk", path)?;
    let file = self.read_buffer(uuid, "file", path)?;
    let arit = self.read_buffer(uuid, "arit", path)?;
    let ownr = self.read_buffer(uuid, "ownr", path)?;
    let nums = self.read_buffer(uuid, "nums", path)?;
    let stat = self.read_buffer(uuid, "stat", path)?;
    self.deserialize(&SerializedHeap { uuid, memo, disk, file, arit, ownr, nums, stat });
    return Ok(());
  }
  fn delete_buffers(&mut self, path: &PathBuf) -> std::io::Result<()> {
    self.delete_buffer(self.uuid, "memo", path)?;
    self.delete_buffer(self.uuid, "disk", path)?;
    self.delete_buffer(self.uuid, "file", path)?;
    self.delete_buffer(self.uuid, "arit", path)?;
    self.delete_buffer(self.uuid, "ownr", path)?;
    self.delete_buffer(self.uuid, "nums", path)?;
    self.delete_buffer(self.uuid, "stat", path)?;
    return Ok(());
  }
}

pub fn init_heap() -> Heap {
  Heap {
    uuid: fastrand::u128(..),
    memo: Nodes { nodes: init_map() },
    disk: Store { links: init_map() },
    file: Funcs { funcs: init_map() },
    arit: Arits { arits: init_map() },
    ownr: Ownrs { ownrs: init_map() },
    tick: U128_NONE,
    time: U128_NONE,
    meta: U128_NONE,
    hax0: U128_NONE,
    hax1: U128_NONE,
    funs: U128_NONE,
    dups: U128_NONE,
    rwts: U128_NONE,
    mana: U128_NONE,
    size: I128_NONE,
    mcap: U128_NONE,
    next: U128_NONE,
  }
}

impl Nodes {
  fn write(&mut self, idx: u128, val: u128) {
    self.nodes.insert(idx, val);
  }
  fn read(&self, idx: u128) -> u128 {
    return self.nodes.get(&idx).map(|x| *x).unwrap_or(U128_NONE);
  }
  fn clear(&mut self) {
    self.nodes.clear();
  }
  fn absorb(&mut self, other: &mut Self, overwrite: bool) {
    for (idx, ownr) in other.nodes.drain() {
      if overwrite || !self.nodes.contains_key(&idx) {
        self.nodes.insert(idx, ownr);
      }
    }
    other.clear();
  }
}

impl Store {
  fn write(&mut self, fid: u128, val: Ptr) {
    self.links.insert(fid, val);
  }
  fn read(&self, fid: u128) -> Option<Ptr> {
    self.links.get(&fid).map(|x| *x)
  }
  fn clear(&mut self) {
    self.links.clear();
  }
  fn absorb(&mut self, other: &mut Self, overwrite: bool) {
    for (fid, func) in other.links.drain() {
      if overwrite || !self.links.contains_key(&fid) {
        self.write(fid as u128, func);
      }
    }
  }
}

impl Funcs {
  fn write(&mut self, fid: u128, val: Arc<CompFunc>) {
    self.funcs.entry(fid).or_insert(val);
  }
  fn read(&self, fid: u128) -> Option<Arc<CompFunc>> {
    return self.funcs.get(&fid).map(|x| x.clone());
  }
  fn clear(&mut self) {
    self.funcs.clear();
  }
  fn absorb(&mut self, other: &mut Self, overwrite: bool) {
    for (fid, func) in other.funcs.drain() {
      if overwrite || !self.funcs.contains_key(&fid) {
        self.write(fid as u128, func.clone());
      }
    }
  }
}

impl Arits {
  fn write(&mut self, fid: u128, val: u128) {
    self.arits.entry(fid).or_insert(val);
  }
  fn read(&self, fid: u128) -> Option<u128> {
    return self.arits.get(&fid).map(|x| *x);
  }
  fn clear(&mut self) {
    self.arits.clear();
  }
  fn absorb(&mut self, other: &mut Self, overwrite: bool) {
    for (fid, arit) in other.arits.drain() {
      if overwrite || !self.arits.contains_key(&fid) {
        self.arits.insert(fid, arit);
      }
    }
  }
}

impl Ownrs {
  fn write(&mut self, fid: u128, val: u128) {
    self.ownrs.entry(fid).or_insert(val);
  }
  fn read(&self, fid: u128) -> Option<u128> {
    return self.ownrs.get(&fid).map(|x| *x);
  }
  fn clear(&mut self) {
    self.ownrs.clear();
  }
  fn absorb(&mut self, other: &mut Self, overwrite: bool) {
    for (fid, ownr) in other.ownrs.drain() {
      if overwrite || !self.ownrs.contains_key(&fid) {
        self.ownrs.insert(fid, ownr);
      }
    }
  }
}

pub fn init_runtime(path: Option<&PathBuf>) -> Runtime {
  // Default runtime store path
  let dflt = dirs::home_dir().unwrap().join(".kindelia").join("state").join("heaps");
  let path = path.unwrap_or_else(|| &dflt);
  std::fs::create_dir_all(&path).unwrap(); // TODO remove unwrap?
  let mut heap = Vec::new();
  for i in 0 .. MAX_HEAPS {
    heap.push(init_heap());
  }
  let mut rt = Runtime {
    heap,
    draw: 0,
    curr: 1,
    nuls: (2 .. MAX_HEAPS).collect(),
    back: Arc::new(Rollback::Nil),
    path: path.clone(),
  };
  rt.run_statements_from_code(GENESIS, true);
  
  rt.snapshot();
  return rt;
}

impl Runtime {

  // API
  // ---

  pub fn define_function(&mut self, fid: u128, func: CompFunc) {
    self.get_heap_mut(self.draw).write_arit(fid, func.arity);
    self.get_heap_mut(self.draw).write_file(fid, Arc::new(func));
  }

  pub fn define_constructor(&mut self, cid: u128, arity: u128) {
    self.get_heap_mut(self.draw).write_arit(cid, arity);
  }

  // pub fn define_function_from_code(&mut self, name: &str, code: &str) {
  //   self.define_function(name_to_u128(name), read_func(code).1);
  // }

  pub fn create_term(&mut self, term: &Term, loc: u128, vars_data: &mut Map<u128>) -> Ptr {
    return create_term(self, term, loc, vars_data);
  }

  pub fn alloc_term(&mut self, term: &Term) -> u128 {
    let loc = alloc(self, 1);
    let lnk = create_term(self, term, loc, &mut init_map());
    self.write(loc, lnk);
    return loc;
  }

  pub fn alloc_term_from_code(&mut self, code: &str) -> u128 {
    let term = read_term(code);
    match term {
      Ok((.., term)) => {
        return self.alloc_term(&term);
      }
      Err(err) => 0 // TODO: what should we do here?
    }
  }

  pub fn collect(&mut self, term: Ptr) {
    collect(self, term)
  }

  pub fn collect_at(&mut self, loc: u128) {
    collect(self, self.read(loc))
  }

  //fn run_io_term(&mut self, subject: u128, caller: u128, term: &Term) -> Option<Ptr> {
    //let main = self.alloc_term(term);
    //let done = self.run_io(subject, caller, main);
    //return done;
  //}

  //fn run_io_from_code(&mut self, code: &str) -> Option<Ptr> {
    //return self.run_io_term(0, 0, &read_term(code).1);
  //}

  pub fn run_statements(&mut self, statements: &[Statement], silent: bool) -> Vec<StatementResult> {
    statements.iter().map(
      |s| { 
        let res = self.run_statement(s, silent);
        if let Ok(..) = res {
          self.draw();
        }
        res
      }
    ).collect()
  }

  pub fn run_statements_from_code(&mut self, code: &str, silent: bool) -> Vec<StatementResult> {
    let stataments = read_statements(code);
    match stataments {
      Ok((.., statements)) => self.run_statements(&statements, silent),
      Err(ParseErr { erro , .. }) => {
        return vec![Err(StatementErr { err: erro })];
      }
    }
  }

  pub fn test_statements(&mut self, statements: &[Statement]) -> Vec<StatementResult> {
    let mut results = vec![];
    for (idx, statement) in statements.iter().enumerate() {
      let res = self.run_statement(statement, true);
      match res {
        Ok(..) => {
          results.push(res);
        }
        Err(..) => {
          results.push(res);
          break
        }
      }
    }
    self.undo();
    results
  }

  pub fn test_statements_from_code(&mut self, code: &str) -> Vec<StatementResult> {
    let stataments = read_statements(code);
    match stataments {
      Ok((.., statements)) => self.test_statements(&statements),
      Err(ParseErr { erro , .. }) => {
        return vec![Err(StatementErr { err: erro })];
      }
    }
  }

  pub fn compute_at(&mut self, loc: u128, mana: u128) -> Result<Ptr, RuntimeError> {
    compute_at(self, loc, mana)
  }

  pub fn compute(&mut self, lnk: Ptr, mana: u128) -> Result<Ptr, RuntimeError> {
    let host = alloc_lnk(self, lnk);
    let done = self.compute_at(host, mana)?;
    clear(self, host, 1);
    return Ok(done);
  }

  pub fn show_term(&self, lnk: Ptr) -> String {
    return show_term(self, lnk, None);
  }

  pub fn show_term_at(&self, loc: u128) -> String {
    return show_term(self, self.read(loc), None);
  }

  // Heaps
  // -----

  pub fn get_heap(&self, index: u64) -> &Heap {
    return &self.heap[index as usize];
  }

  pub fn get_heap_mut(&mut self, index: u64) -> &mut Heap {
    return &mut self.heap[index as usize];
  }

  // Copies the contents of the absorbed heap into the absorber heap
  fn absorb_heap(&mut self, absorber: u64, absorbed: u64, overwrite: bool) {
    // FIXME: can we satisfy the borrow checker without using unsafe pointers?
    unsafe {
      let a_arr = &mut self.heap as *mut Vec<Heap>;
      let a_ref = &mut *(&mut (*a_arr)[absorber as usize] as *mut Heap);
      let b_ref = &mut *(&mut (*a_arr)[absorbed as usize] as *mut Heap);
      a_ref.absorb(b_ref, overwrite);
    }
  }

  fn clear_heap(&mut self, index: u64) {
    self.heap[index as usize].clear();
  }

  fn undo(&mut self) {
    self.clear_heap(self.draw);
  }

  fn draw(&mut self) {
    self.absorb_heap(self.curr, self.draw, true);
    self.clear_heap(self.draw);
  }

  // IO
  // --

  pub fn run_io(&mut self, subject: u128, caller: u128, host: u128, mana: u128) -> Result<Ptr, RuntimeError> {
    let term = reduce(self, host, mana)?;
    // eprintln!("-- {}", show_term(self, term));
    match get_tag(term) {
      CTR => {
        match get_ext(term) {
          IO_DONE => {
            let retr = ask_arg(self, term, 0);
            clear(self, host, 1);
            clear(self, get_loc(term, 0), 1);
            return Ok(retr);
          }
          IO_TAKE => {
            //println!("- IO_TAKE subject is {} {}", u128_to_name(subject), subject);
            let cont = ask_arg(self, term, 0);
            if let Some(state) = self.read_disk(subject) {
              if state != 0 {
                self.write_disk(subject, 0);
                let cont = alloc_app(self, cont, state);
                let done = self.run_io(subject, subject, cont, mana);
                clear(self, host, 1);
                clear(self, get_loc(term, 0), 1);
                return done;
              }
            }
            return Err(RuntimeError::EffectFailure);
          }
          IO_SAVE => {
            //println!("- IO_SAVE subject is {} {}", u128_to_name(subject), subject);
            let expr = ask_arg(self, term, 0);
            let save = self.compute(expr, mana)?;
            self.write_disk(subject, save);
            let cont = ask_arg(self, term, 1);
            let cont = alloc_app(self, cont, Num(0));
            let done = self.run_io(subject, subject, cont, mana);
            clear(self, host, 1);
            clear(self, get_loc(term, 0), 2);
            return done;
          }
          IO_CALL => {
            let fnid = ask_arg(self, term, 0);
            let tupl = ask_arg(self, term, 1);
            let cont = ask_arg(self, term, 2);
            // Builds the argument vector
            let arit = self.get_arity(get_ext(tupl));
            let mut args = Vec::new();
            for i in 0 .. arit {
              args.push(ask_arg(self, tupl, i));
            }
            // Calls called function IO, changing the subject
            // TODO: this should not alloc a Fun as it's limited to 72-bit names
            let ioxp = alloc_fun(self, get_num(fnid), &args);
            let retr = self.run_io(get_num(fnid), subject, ioxp, mana)?;
            // Calls the continuation with the value returned
            let cont = alloc_app(self, cont, retr);
            let done = self.run_io(subject, caller, cont, mana);
            // Clears memory
            clear(self, host, 1);
            clear(self, get_loc(tupl, 0), arit);
            clear(self, get_loc(term, 0), 3);
            return done;
          }
          IO_SUBJ => {
            let cont = ask_arg(self, term, 0);
            let cont = alloc_app(self, cont, Num(subject));
            let done = self.run_io(subject, caller, cont, mana);
            clear(self, host, 1);
            clear(self, get_loc(term, 0), 1);
            return done;
          }
          IO_FROM => {
            let cont = ask_arg(self, term, 0);
            let cont = alloc_app(self, cont, Num(subject));
            let done = self.run_io(subject, caller, cont, mana);
            clear(self, host, 1);
            clear(self, get_loc(term, 0), 1);
            return done;
          }
          IO_TICK => {
            let cont = ask_arg(self, term, 0);
            let cont = alloc_app(self, cont, Num(subject));
            let done = self.run_io(subject, self.get_tick(), cont, mana);
            clear(self, host, 1);
            clear(self, get_loc(term, 0), 1);
            return done;
          }
          IO_TIME => {
            let cont = ask_arg(self, term, 0);
            let cont = alloc_app(self, cont, Num(subject));
            let done = self.run_io(subject, self.get_time(), cont, mana);
            clear(self, host, 1);
            clear(self, get_loc(term, 0), 1);
            return done;
          }
          IO_META => {
            let cont = ask_arg(self, term, 0);
            let cont = alloc_app(self, cont, Num(subject));
            let done = self.run_io(subject, self.get_meta(), cont, mana);
            clear(self, host, 1);
            clear(self, get_loc(term, 0), 1);
            return done;
          }
          IO_HAX0 => {
            let cont = ask_arg(self, term, 0);
            let cont = alloc_app(self, cont, Num(subject));
            let done = self.run_io(subject, self.get_hax0(), cont, mana);
            clear(self, host, 1);
            clear(self, get_loc(term, 0), 1);
            return done;
          }
          IO_HAX1 => {
            let cont = ask_arg(self, term, 0);
            let cont = alloc_app(self, cont, Num(subject));
            let done = self.run_io(subject, self.get_hax1(), cont, mana);
            clear(self, host, 1);
            clear(self, get_loc(term, 0), 1);
            return done;
          }
          _ => {
            return Err(RuntimeError::EffectFailure);
          }
        }
      }
      _ => {
        return Err(RuntimeError::EffectFailure);
      }
    }
  }

  // Gets the subject of a signature
  pub fn get_subject(&mut self, sign: &Option<crypto::Signature>, hash: crypto::Hash) -> u128 {
    match sign {
      None       => 0,
      Some(sign) => sign.signer_name(&hash).map(|x| x.0).unwrap_or(1),
    }
  }

  // Can this subject deploy this name?
  pub fn can_deploy(&mut self, subj: u128, name: u128) -> bool {
    if name == 0 {
      // nobody can deploy the empty name
      return false;
    } else {
      match get_namespace(name) {
        None => {
          // anyone can deploy a namespace-less name
          return true;
        }
        Some(namespace) => {
          // only owner can deploy on its namespace
          return subj == self.get_owner(namespace);
        }
      }
    }
  }

  // Can this subject register this namespace?
  pub fn can_register(&mut self, subj: u128, name: u128) -> bool {
    if name == 0 {
      // anyone can register the empty namespace (should happen on Genesis Block)
      return true;
    } else {
      // only namespace owner can register a sub-namespace
      return subj == self.get_owner(get_namespace(name).unwrap_or(0));
    }
  }

  /// Run statement in the `draw` heap.
  /// 
  /// It doesn't alter `curr` heap.
  #[allow(clippy::useless_format)]
  pub fn run_statement(&mut self, statement: &Statement, silent: bool) -> StatementResult {
    fn error(rt: &mut Runtime, tag: &str, err: String) -> StatementResult {
      rt.undo();
      println!("[{}] Error. {}", tag, err);
      return Err(StatementErr { err });
    }
    let hash = hash_statement(statement);
    match statement {
      Statement::Fun { name, args, func, init, sign } => {
        if self.exists(*name) {
          return error(self, "fun", format!("Can't redefine '{}'.", u128_to_name(*name)));
        }
        let subj = self.get_subject(&sign, hash);
        if !self.can_deploy(subj, *name) {
          return error(self, "fun", format!("Subject '#x{:0>30x}' not allowed to deploy '{}'.", subj, u128_to_name(*name)));
        }
        if !self.check_func(&func) {
          return error(self, "fun", format!("Invalid function {}.", u128_to_name(*name)));
        }
        let func = compile_func(func, true);
        if func.is_none() {
          return error(self, "fun", format!("Invalid function {}.", u128_to_name(*name)));
        }
        let func = func.unwrap();
        if !silent {
          println!("[fun] {}", u128_to_name(*name));
        }
        self.set_arity(*name, args.len() as u128);
        self.define_function(*name, func);
        let state = self.create_term(init, 0, &mut init_map());
        self.write_disk(*name, state);
        Ok(StatementInfo::Fun { name: *name, args: args.clone() })
      }
      Statement::Ctr { name, args, sign } => {
        if self.exists(*name) {
          return error(self, "ctr", format!("Can't redefine '{}'.", u128_to_name(*name)));
        }
        let subj = self.get_subject(&sign, hash);
        if !self.can_deploy(subj, *name) {
          return error(self, "ctr", format!("Subject '#x{:0>30x}' not allowed to deploy '{}'.", subj, u128_to_name(*name)));
        }
        if args.len() > 16 {
          return error(self, "ctr", format!("Can't define contructor with arity larger than 16."));
        }
        if !silent {
          println!("[ctr] {}", u128_to_name(*name));
        }
        self.set_arity(*name, args.len() as u128);
        Ok(StatementInfo::Ctr { name: *name, args: args.clone() })
      }
      Statement::Run { expr, sign } => {
        let mana_ini = self.get_mana(); 
        let mana_lim = self.get_mana_limit();
        let size_ini = self.get_size();
        let size_lim = self.get_size_limit(); 
        if !self.check_term(expr) {
          return error(self, "run", format!("Invalid term."));
        }
        let subj = self.get_subject(&sign, hash);
        let host = self.alloc_term(expr);
        let done = self.run_io(subj, 0, host, mana_lim);
        if let Err(err) = done {
          return error(self, "run", show_runtime_error(err));
        }
        let done = done.unwrap();
        let done = self.compute(done, mana_lim);
        if let Err(err) = done {
          return error(self, "run", show_runtime_error(err));
        }
        let done = done.unwrap();
        let term = readback_linear_term(self, done);
        self.collect(done);
        let size_end = self.get_size();
        let mana_dif = self.get_mana() - mana_ini;
        let size_dif = size_end - size_ini;
        if size_end > size_lim {
          return error(self, "run", format!("Not enough space."));
        }
        if !silent {
          println!("[run] {} \x1b[2m[{} mana | {} size]\x1b[0m", view_term(&term), mana_dif, size_dif);
        }
        Ok(StatementInfo::Run {
          done_term: term,
          used_mana: mana_dif,
          size_diff: size_dif,
          end_size: size_end as u128, // TODO: rename to done_size for consistency?
        })
      }
      Statement::Reg { name, ownr, sign } => {
        if self.exists(*name) {
          return error(self, "run", format!("Can't redefine '{}'.", u128_to_name(*name)));
        }
        let subj = self.get_subject(sign, hash);
        if !self.can_register(subj, *name) {
          return error(self, "run", format!("Subject '#x{:0>30x}' not allowed to register '{}'.", subj, u128_to_name(*name)));
        }
        self.set_owner(*name, *ownr);
        if !silent {
          println!("[reg] #x{:0>30x} {}", ownr, u128_to_name(*name));
        }
        Ok(StatementInfo::Reg {
          name: *name,
          ownr: *ownr,
        })
      }
    }
  }

  pub fn check_term(&self, term: &Term) -> bool {
    return self.check_term_depth(term, 0) && is_linear(term); // && self.check_term_arities(term)
  }

  pub fn check_func(&self, func: &Func) -> bool {
    for rule in &func.rules {
      if !self.check_term(&rule.lhs) || !self.check_term(&rule.rhs) {
        return false;
      }
    }
    return true;
  }

  pub fn check_term_depth(&self, term: &Term, depth: u128) -> bool {
    if depth > MAX_TERM_DEPTH {
      return false;
    } else {
      match term {
        Term::Var { name } => {
          return true;
        },
        Term::Dup { nam0, nam1, expr, body } => {
          let expr_check = self.check_term_depth(expr, depth + 1);
          let body_check = self.check_term_depth(body, depth + 1);
          return expr_check && body_check;
        }
        Term::Lam { name, body } => {
          let body_check = self.check_term_depth(body, depth + 1);
          return body_check;
        }
        Term::App { func, argm } => {
          let func_check = self.check_term_depth(func, depth + 1);
          let argm_check = self.check_term_depth(argm, depth + 1);
          return func_check && argm_check;
        }
        Term::Ctr { name, args } => {
          for arg in args {
            if !self.check_term_depth(arg, depth + 1) {
              return false;
            }
          }
          return true;
        }
        Term::Fun { name, args } => {
          for arg in args {
            if !self.check_term_depth(arg, depth + 1) {
              return false;
            }
          }
          return true;
        }
        Term::Num { numb } => {
          return true;
        }
        Term::Op2 { oper, val0, val1 } => {
          let val0_check = self.check_term_depth(val0, depth + 1);
          let val1_check = self.check_term_depth(val1, depth + 1);
          return val0_check && val1_check;
        }
      }
    }
  }

  // Maximum mana = 42m * block_number
  pub fn get_mana_limit(&self) -> u128 {
    (self.get_tick() + 1) * BLOCK_MANA_LIMIT
  }

  // Maximum size = 2048 * block_number
  pub fn get_size_limit(&self) -> i128 {
    (self.get_tick() as i128 + 1) * (BLOCK_BITS_LIMIT / 128)
  }

  // Rollback
  // --------

  // Returns a clone of a reference to the current rollback state.
  pub fn get_back(&self) -> Arc<Rollback> {
    return self.back.clone();
  }

  // Advances the heap time counter, saving past states for rollback.
  pub fn tick(&mut self) {
    self.set_tick(self.get_tick() + 1);
    self.draw();
    self.snapshot();
  }

  pub fn snapshot(&mut self) {
    //println!("tick self.curr={}", self.curr);
    let (included, absorber, deleted, rollback) = rollback_push(self.curr, self.back.clone(), 0);
    // println!("- tick={} self.curr={}, included={:?} absorber={:?} deleted={:?} rollback={}", self.get_tick(), self.curr, included, absorber, deleted, view_rollback(&self.back));
    self.back = rollback;
    // println!(" - back {}", view_rollback(&self.back));
    if included {
      self.save_state_metadata().expect("Error saving state metadata.");
      let path = &self.get_dir_path();
      self.heap[self.curr as usize].save_buffers(path).expect("Error saving buffers."); // TODO: persistence-WIP
      if let Some(deleted) = deleted {
        if let Some(absorber) = absorber {
          self.absorb_heap(absorber, deleted, false);
          self.heap[absorber as usize].append_buffers(self.heap[deleted as usize].uuid, path).expect("Couldn't append buffers."); // TODO: persistence-WIP
        }
        self.heap[deleted as usize].delete_buffers(path).expect("Couldn't delete buffers.");
        self.clear_heap(deleted);
        self.curr = deleted;
      } else if let Some(empty) = self.nuls.pop() {
        self.curr = empty;
      } else {
        //println!("- {} {} {:?} {}", self.draw, self.curr, self.nuls, view_rollback(&self.back));
        panic!("Not enough heaps.");
      }
    }
  }

  // Rolls back to the earliest state before or equal `tick`
  pub fn rollback(&mut self, tick: u128) {
    // If target tick is older than current tick
    if tick < self.get_tick() {
      //println!("- rolling back from {} to {}", self.get_tick(), tick);
      self.clear_heap(self.curr);
      self.nuls.push(self.curr);
      let mut cuts = 0;
      let path = self.get_dir_path();
      // Removes heaps until the runtime's tick is larger than, or equal to, the target tick
      while tick < self.get_tick() {
        if let Rollback::Cons { keep, life, head, tail } = &*self.back.clone() {
          self.heap[*head as usize].delete_buffers(&path).expect("Couldn't delete buffers.");
          self.clear_heap(*head);
          self.nuls.push(*head);
          self.back = tail.clone();
          cuts += 1 + life;
        }
      }
      if let Rollback::Cons { keep, life, head, tail } = &*self.back {
        self.back = Arc::new(Rollback::Cons { keep: 0, life: *life + cuts, head: *head, tail: tail.clone() });
      }
      self.curr = self.nuls.pop().expect("No heap available!");
    }
    // println!("- rolled back to {}", self.get_tick());
  }

  // Persistence
  // -----------

  pub fn get_dir_path(&self) -> PathBuf {
    return self.path.clone();
  }

  // Persists the current state. Since heaps are automatically saved to disk, function only saves
  // their uuids. Note that this will NOT save the current heap, nor anything after the last heap
  // included on the Rollback list. In other words, it forgets up to ~16 recent blocks. This
  // function is used to avoid re-processing the entire block history on node startup.
  pub fn save_state_metadata(&self) -> std::io::Result<()> {
    fn build_persistence_buffers(rt: &Runtime, rollback: &Rollback, keeps: &mut Vec<u128>, lifes: &mut Vec<u128>, uuids: &mut Vec<u128>) {
      match rollback {
        Rollback::Cons { keep, life, head, tail } => {
          keeps.push(*keep as u128);
          lifes.push(*life as u128);
          uuids.push(rt.heap[*head as usize].uuid);
          build_persistence_buffers(rt, tail, keeps, lifes, uuids);
        }
        Rollback::Nil => {}
      }
    }
    let mut keeps : Vec<u128> = vec![];
    let mut lifes : Vec<u128> = vec![];
    let mut uuids : Vec<u128> = vec![];
    build_persistence_buffers(self, &self.back,  &mut keeps, &mut lifes, &mut uuids);
    std::fs::write(self.path.join("_keeps_"), &util::u128s_to_u8s(&keeps))?;
    std::fs::write(self.path.join("_lifes_"), &util::u128s_to_u8s(&lifes))?;
    std::fs::write(self.path.join("_uuids_"), &util::u128s_to_u8s(&uuids))?;
    return Ok(());
  }

  // Restores the saved state. This loads the persisted Rollback list and its heaps.
  pub fn restore_state(&mut self) -> std::io::Result<()> {
    for i in 0 .. MAX_HEAPS {
      self.heap[i as usize].clear();
    }
    self.nuls = (2 .. MAX_HEAPS).collect();
    // for i in 0 .. std::cmp::max(uuids.len(), 8) {
    //   self.heap[i + 2].load_buffers(uuids[i])?;
    // }
    let mut keeps = util::u8s_to_u128s(&std::fs::read(self.path.join("_keeps_"))?);
    let mut lifes = util::u8s_to_u128s(&std::fs::read(self.path.join("_lifes_"))?);
    let mut uuids = util::u8s_to_u128s(&std::fs::read(self.path.join("_uuids_"))?);
    fn load_heaps(rt: &mut Runtime, keeps: &mut Vec<u128>, lifes: &mut Vec<u128>, uuids: &mut Vec<u128>, index: u64, back: Arc<Rollback>) -> std::io::Result<Arc<Rollback>> {
      let keep = keeps.pop();
      let life = lifes.pop();
      let uuid = uuids.pop();
      match (keep, life, uuid) {
        (Some(keep), Some(life), Some(uuid)) => {
          let next = rt.nuls.pop();
          match next {
            Some(next) => {
              let path = rt.get_dir_path();
              rt.heap[index as usize].load_buffers(uuid, &path)?;
              rt.curr = index;
              return load_heaps(rt, keeps, lifes, uuids, next, Arc::new(Rollback::Cons { keep: keep as u64, life: life as u64, head: index, tail: back }));
            }
            None => {
              panic!("Not enough heaps.");
            }
          }
        }
        (None, None, None) => {
          return Ok(back);
        }
        _ => {
          panic!("Corrupted persistence files.");
        }
      }
    }
    self.draw = 0;
    self.curr = 1;
    self.back = load_heaps(self, &mut keeps, &mut lifes, &mut uuids, self.curr, Arc::new(Rollback::Nil))?;
    self.curr = self.nuls.pop().expect("No heap available!");
    return Ok(());
  }

  // Reverts until the last 
  pub fn clear_current_heap(&mut self) {
    self.heap[self.curr as usize].clear();
  }

  // Heap writers and readers
  // ------------------------

  // Attempts to read data from the latest heap.
  // If not present, looks for it on past states.
  pub fn get_with<A: std::cmp::PartialEq>(&self, zero: A, none: A, get: impl Fn(&Heap) -> A) -> A {
    let got = get(&self.get_heap(self.draw));
    if none != got {
      return got;
    }
    let got = get(&self.get_heap(self.curr));
    if none != got {
      return got;
    }
    let mut back = &self.back;
    loop {
      match &**back {
        Rollback::Cons { keep, life, head, tail } => {
          let val = get(self.get_heap(*head));
          if val != none {
            return val;
          }
          back = &*tail;
        }
        Rollback::Nil => {
          return zero;
        }
      }
    }
  }

  // Same as get_with, but gets a function
  // FIXME: can get_with be generalized for this case too?
  pub fn get_func(&self, fid: u128) -> Option<Arc<CompFunc>> {
    let got = self.get_heap(self.draw).read_file(fid);
    if let Some(func) = got {
      return Some(func);
    }
    let got = self.get_heap(self.curr).read_file(fid);
    if let Some(func) = got {
      return Some(func);
    }
    let mut back = &self.back;
    loop {
      match &**back {
        Rollback::Cons { keep, life, head, tail } => {
          let got = self.get_heap(*head).file.read(fid);
          if let Some(func) = got {
            return Some(func);
          }
          back = &*tail;
        }
        Rollback::Nil => {
          return None;
        }
      }
    }
  }

  pub fn reduce_with<A>(&self, acc: &mut A, reduce: impl Fn(&mut A, &Heap)) {
    reduce(acc, &self.get_heap(self.draw));
    reduce(acc, &self.get_heap(self.curr));
    let mut back = &self.back;
    while let Rollback::Cons { keep: _, life, head, tail } = &**back {
      reduce(acc, self.get_heap(*head));
      back = &*tail;
    }
  }

  pub fn write(&mut self, idx: u128, val: u128) {
    return self.get_heap_mut(self.draw).write(idx, val);
  }

  pub fn read(&self, idx: u128) -> u128 {
    return self.get_with(0, U128_NONE, |heap| heap.read(idx));
  }

  pub fn write_disk(&mut self, fid: u128, val: Ptr) {
    return self.get_heap_mut(self.draw).write_disk(fid, val);
  }

  pub fn read_disk(&self, fid: u128) -> Option<Ptr> {
    return self.get_with(Some(0), None, |heap| heap.read_disk(fid));
  }

  pub fn read_disk_as_term(&mut self, fid: u128) -> Option<Term> {
    let host = self.read_disk(fid)?;
    let term = readback_linear_term(self, host);
    Some(term)
  }

  pub fn read_file(&self, fid: u128) -> Option<CompFunc> {
    self.get_with(None, None, |heap| heap.read_file(fid)).map(|func| (*func).clone())
  }

  pub fn get_arity(&self, fid: u128) -> u128 {
    if let Some(arity) = self.get_with(None, None, |heap| heap.read_arit(fid)) {
      return arity;
    } else {
      return U128_NONE;
    }
  }

  pub fn set_arity(&mut self, fid: u128, arity: u128) {
    self.get_heap_mut(self.draw).write_arit(fid, arity);
  }

  pub fn get_owner(&self, name: u128) -> u128 {
    if let Some(owner) = self.get_with(None, None, |heap| heap.read_ownr(name)) {
      return owner;
    } else {
      return U128_NONE;
    }
  }

  pub fn set_owner(&mut self, name: u128, owner: u128) {
    self.get_heap_mut(self.draw).write_ownr(name, owner);
  }

  pub fn exists(&self, fid: u128) -> bool {
    if let Some(arity) = self.get_with(None, None, |heap| heap.read_arit(fid)) {
      return true;
    } else if let Some(owner) = self.get_with(None, None, |heap| heap.read_ownr(fid)) {
      return true;
    } else {
      return false;
    }
  }

  pub fn get_dups(&self) -> u128 {
    return self.get_with(0, U128_NONE, |heap| heap.get_dups());
  }

  pub fn set_rwts(&mut self, rwts: u128) {
    self.get_heap_mut(self.draw).set_rwts(rwts);
  }

  pub fn get_rwts(&self) -> u128 {
    return self.get_with(0, U128_NONE, |heap| heap.rwts);
  }

  pub fn set_mana(&mut self, mana: u128) {
    self.get_heap_mut(self.draw).set_mana(mana);
  }

  pub fn get_mana(&self) -> u128 {
    return self.get_with(0, U128_NONE, |heap| heap.mana);
  }

  pub fn set_tick(&mut self, tick: u128) {
    self.get_heap_mut(self.draw).set_tick(tick);
  }

  pub fn get_tick(&self) -> u128 {
    return self.get_with(0, U128_NONE, |heap| heap.tick);
  }

  pub fn set_time(&mut self, time: u128) {
    self.get_heap_mut(self.draw).set_time(time);
  }

  pub fn get_time(&self) -> u128 {
    return self.get_with(0, U128_NONE, |heap| heap.time);
  }

  pub fn set_meta(&mut self, meta: u128) {
    self.get_heap_mut(self.draw).set_meta(meta);
  }

  pub fn get_meta(&self) -> u128 {
    return self.get_with(0, U128_NONE, |heap| heap.meta);
  }

  pub fn set_hax0(&mut self, hax0: u128) {
    self.get_heap_mut(self.draw).set_hax0(hax0);
  }

  pub fn get_hax0(&self) -> u128 {
    return self.get_with(0, U128_NONE, |heap| heap.hax0);
  }

  pub fn set_hax1(&mut self, hax1: u128) {
    self.get_heap_mut(self.draw).set_hax1(hax1);
  }

  pub fn get_hax1(&self) -> u128 {
    return self.get_with(0, U128_NONE, |heap| heap.hax1);
  }

  pub fn set_size(&mut self, size: i128) {
    self.get_heap_mut(self.draw).size = size;
  }

  pub fn get_size(&self) -> i128 {
    return self.get_with(0, I128_NONE, |heap| heap.size);
  }

  pub fn set_mcap(&mut self, mcap: u128) {
    self.get_heap_mut(self.draw).mcap = mcap;
  }

  pub fn get_mcap(&self) -> u128 {
    return self.get_with(32, U128_NONE, |heap| heap.mcap);
  }

  pub fn set_next(&mut self, next: u128) {
    self.get_heap_mut(self.draw).next = next;
  }

  pub fn get_next(&self) -> u128 {
    return self.get_with(0, U128_NONE, |heap| heap.next);
  }

  pub fn fresh_dups(&mut self) -> u128 {
    let dups = self.get_dups();
    self.get_heap_mut(self.draw).set_dups(dups + 1);
    return dups & 0x3FFFFFFF;
  }
}

// Attempts to include a heap state on the list of past heap states. It only keeps at most
// `log_16(tick)` heaps in memory, rejecting heaps that it doesn't need to store. It returns:
// - included : Bool             = true if the heap was included, false if it was rejected
// - absorber : Option<Box<u64>> = the index of the dropped heap absorber (if any)
// - deleted  : Option<Box<u64>> = the index of the dropped heap (if any)
// - rollback : Rollback         = the updated rollback object
pub fn rollback_push(elem: u64, back: Arc<Rollback>, depth: u64) -> (bool, Option<u64>, Option<u64>, Arc<Rollback>) {
  if depth >= MAX_ROLLBACK {
    return (false, None, Some(elem), Arc::new(Rollback::Nil));
  } else {
    match &*back {
      Rollback::Nil => {
        let rollback = Arc::new(Rollback::Cons { keep: 0, life: 0, head: elem, tail: Arc::new(Rollback::Nil) });
        return (true, None, None, rollback);
      }
      Rollback::Cons { keep, life, head, tail } => {
        if *keep == 0xF {
          if *life > 0 {
            let tail = Arc::new(Rollback::Cons { keep: 0, life: life - 1, head: *head, tail: tail.clone() });
            let back = Arc::new(Rollback::Cons { keep: 0, life: 0, head: elem, tail });
            return (true, None, None, back);
          } else {
            let (included, absorber, deleted, tail) = rollback_push(*head, tail.clone(), depth + 1);
            let absorber = if !included { Some(elem) } else { absorber };
            let rollback = Arc::new(Rollback::Cons { keep: 0, life: *life, head: elem, tail });
            return (true, absorber, deleted, rollback);
          }
        } else {
          let rollback = Arc::new(Rollback::Cons { keep: keep + 1, life: *life, head: *head, tail: tail.clone() });
          return (false, None, Some(elem), rollback);
        }
      }
    }
  }
}

pub fn view_rollback(back: &Arc<Rollback>) -> String {
  match &**back {
    Rollback::Nil => {
      return String::new();
    }
    Rollback::Cons { keep, life, head, tail } => {
      return format!("[{:x} {}] {}", keep, head, view_rollback(tail));
    }
  }
}


// Constructors
// ------------

pub fn Var(pos: u128) -> Ptr {
  (VAR * TAG) | pos
}

pub fn Dp0(col: u128, pos: u128) -> Ptr {
  (DP0 * TAG) | (col * EXT) | pos
}

pub fn Dp1(col: u128, pos: u128) -> Ptr {
  (DP1 * TAG) | (col * EXT) | pos
}

pub fn Arg(pos: u128) -> Ptr {
  (ARG * TAG) | pos
}

pub fn Era() -> Ptr {
  ERA * TAG
}

pub fn Lam(pos: u128) -> Ptr {
  (LAM * TAG) | pos
}

pub fn App(pos: u128) -> Ptr {
  (APP * TAG) | pos
}

pub fn Par(col: u128, pos: u128) -> Ptr {
  (SUP * TAG) | (col * EXT) | pos
}

pub fn Op2(ope: u128, pos: u128) -> Ptr {
  (OP2 * TAG) | (ope * EXT) | pos
}

pub fn Num(val: u128) -> Ptr {
  debug_assert!((!NUM_MASK & val) == 0, "Num overflow: `{}`.", val);
  (NUM * TAG) | (val & NUM_MASK)
}

pub fn Ctr(fun: u128, pos: u128) -> Ptr {
  debug_assert!(fun < 1 << 72, "Directly calling constructor with too long name: `{}`.", u128_to_name(fun));
  (CTR * TAG) | (fun * EXT) | pos
}

pub fn Fun(fun: u128, pos: u128) -> Ptr {
  debug_assert!(fun < 1 << 72, "Directly calling function with too long name: `{}`.", u128_to_name(fun));
  (FUN * TAG) | (fun * EXT) | pos
}

// Getters
// -------

pub fn get_tag(lnk: Ptr) -> u128 {
  lnk / TAG
}

pub fn get_ext(lnk: Ptr) -> u128 {
  (lnk / EXT) & 0xFF_FFFF_FFFF_FFFF_FFFF
}

pub fn get_val(lnk: Ptr) -> u128 {
  lnk & 0xFFFF_FFFF_FFFF
}

pub fn get_num(lnk: Ptr) -> u128 {
  lnk & 0xFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF
}

//pub fn get_ari(lnk: Ptr) -> u128 {
  //(lnk / ARI) & 0xF
//}

pub fn get_loc(lnk: Ptr, arg: u128) -> u128 {
  get_val(lnk) + arg
}

// Memory
// ------

pub fn ask_lnk(rt: &Runtime, loc: u128) -> Ptr {
  rt.read(loc)
  //unsafe { *rt.heap.get_unchecked(loc as usize) }
}

pub fn ask_arg(rt: &Runtime, term: Ptr, arg: u128) -> Ptr {
  ask_lnk(rt, get_loc(term, arg))
}

pub fn link(rt: &mut Runtime, loc: u128, lnk: Ptr) -> Ptr {
  rt.write(loc, lnk);
  if get_tag(lnk) <= VAR {
    let pos = get_loc(lnk, get_tag(lnk) & 0x01);
    rt.write(pos, Arg(loc));
  }
  lnk
}

pub fn alloc(rt: &mut Runtime, arity: u128) -> u128 {
  if arity == 0 {
    return 0;
  } else {
    loop {
      // Attempts to allocate enough space, starting from the last index
      // where we previously found free space, and moving rightwards
      let mcap = rt.get_mcap();
      let index = rt.get_next();
      if index <= mcap - arity {
        let mut has_space = true;
        for i in 0 .. arity {
          if rt.read(index + i) != 0 {
            has_space = false;
            break;
          }
        }
        // If we managed to find enough free space somewhere, return that index
        if has_space {
          rt.set_next(rt.get_next() + arity);
          rt.set_size(rt.get_size() + arity as i128);
          //println!("{}", show_memo(rt));
          for i in 0 .. arity {
            rt.write(index + i, NIL); // millions perished for forgetting this line
          }
          return index;
        }
      }
      // If we couldn't allocate space...
      // - If less than 50% of the memory is used, jump to a random index and try again
      // - If more than 50% of the memory is used, double the maximum cap and try again
      if rt.get_size() * 2 < (mcap as i128) {
        rt.set_next((fastrand::u64(..) % mcap as u64) as u128);
      } else {
        rt.set_mcap(mcap * 2);
      }
    }
  }
}

pub fn clear(rt: &mut Runtime, loc: u128, size: u128) {
  for i in 0 .. size {
    if rt.read(loc + i) == 0 {
      panic!("Cleared twice: {}", loc);
    }
    rt.write(loc + i, 0);
  }
  rt.set_size(rt.get_size() - size as i128);
  //rt.free[size as usize].push(loc);
}

pub fn collect(rt: &mut Runtime, term: Ptr) {
  let mut stack : Vec<Ptr> = Vec::new();
  let mut next = term;
  let mut dups : Vec<u128> = Vec::new();
  loop {
    let term = next;
    match get_tag(term) {
      DP0 => {
        link(rt, get_loc(term, 0), Era());
        dups.push(term);
      }
      DP1 => {
        link(rt, get_loc(term, 1), Era());
        dups.push(term);
      }
      VAR => {
        link(rt, get_loc(term, 0), Era());
      }
      LAM => {
        if get_tag(ask_arg(rt, term, 0)) != ERA {
          link(rt, get_loc(ask_arg(rt, term, 0), 0), Era());
        }
        next = ask_arg(rt, term, 1);
        clear(rt, get_loc(term, 0), 2);
        continue;
      }
      APP => {
        stack.push(ask_arg(rt, term, 0));
        next = ask_arg(rt, term, 1);
        clear(rt, get_loc(term, 0), 2);
        continue;
      }
      SUP => {
        stack.push(ask_arg(rt, term, 0));
        next = ask_arg(rt, term, 1);
        clear(rt, get_loc(term, 0), 2);
        continue;
      }
      OP2 => {
        stack.push(ask_arg(rt, term, 0));
        next = ask_arg(rt, term, 1);
        clear(rt, get_loc(term, 0), 2);
        continue;
      }
      NUM => {}
      CTR | FUN => {
        let arity = rt.get_arity(get_ext(term));
        for i in 0 .. arity {
          if i < arity - 1 {
            stack.push(ask_arg(rt, term, i));
          } else {
            next = ask_arg(rt, term, i);
          }
        }
        clear(rt, get_loc(term, 0), arity);
        if arity > 0 {
          continue;
        }
      }
      _ => {}
    }
    if let Some(got) = stack.pop() {
      next = got;
    } else {
      break;
    }
  }
  for dup in dups {
    let fst = ask_arg(rt, dup, 0);
    let snd = ask_arg(rt, dup, 1);
    if get_tag(fst) == ERA && get_tag(snd) == ERA {
      collect(rt, ask_arg(rt, dup, 2));
      clear(rt, get_loc(dup, 0), 3);
    }
  }
}

// Term
// ----

// Counts how many times the free variable 'name' appears inside Term
fn count_uses(term: &Term, name: u128) -> u128 {
  match term {
    Term::Var { name: var_name } => {
      return if name == *var_name { 1 } else { 0 };
    }
    Term::Dup { nam0, nam1, expr, body } => {
      let expr_uses = count_uses(expr, name);
      let body_uses = if name == *nam0 || name == *nam1 { 0 } else { count_uses(body, name) };
      return expr_uses + body_uses;
    }
    Term::Lam { name: lam_name, body } => {
      return if name == *lam_name { 0 } else { count_uses(body, name) };
    }
    Term::App { func, argm } => {
      let func_uses = count_uses(func, name);
      let argm_uses = count_uses(argm, name);
      return func_uses + argm_uses;
    }
    Term::Ctr { name: ctr_name, args } => {
      let mut uses = 0;
      for arg in args {
        uses += count_uses(arg, name);
      }
      return uses;
    }
    Term::Fun { name: fun_name, args } => {
      let mut uses = 0;
      for arg in args {
        uses += count_uses(arg, name);
      }
      return uses;
    }
    Term::Num { numb } => {
      return 0;
    }
    Term::Op2 { oper, val0, val1 } => {
      let val0_uses = count_uses(val0, name);
      let val1_uses = count_uses(val1, name);
      return val0_uses + val1_uses;
    }
  }
}

// Checks if:
// - Every non-erased variable is used exactly once
// - Every erased variable is never used
pub fn is_linear(term: &Term) -> bool {
  // println!("{}", view_term(term));
  let res = match term {
    Term::Var { name: var_name } => {
      true
    }
    Term::Dup { nam0, nam1, expr, body } => {
      let expr_linear = is_linear(expr);
      let body_linear
        =  (*nam0 == VAR_NONE || count_uses(body, *nam0) == 1)
        && (*nam1 == VAR_NONE || count_uses(body, *nam1) == 1)
        && is_linear(body);
      expr_linear && body_linear
    }
    Term::Lam { name, body } => {
      let body_linear
        =  (*name == VAR_NONE || count_uses(body, *name) == 1)
        && is_linear(body);
      body_linear
    }
    Term::App { func, argm } => {
      let func_linear = is_linear(func);
      let argm_linear = is_linear(argm);
      func_linear && argm_linear
    }
    Term::Ctr { name: ctr_name, args } => {
      let mut linear = true;
      for arg in args {
        linear = linear && is_linear(arg);
      }
      linear
    }
    Term::Fun { name: fun_name, args } => {
      let mut linear = true;
      for arg in args {
        linear = linear && is_linear(arg);
      }
      linear
    }
    Term::Num { numb } => {
      true
    }
    Term::Op2 { oper, val0, val1 } => {
      let val0_linear = is_linear(val0);
      let val1_linear = is_linear(val1);
      val0_linear && val1_linear
    }
  };

  // println!("{}", res);
  res
}

// Writes a Term represented as a Rust enum on the Runtime's rt.
pub fn create_term(rt: &mut Runtime, term: &Term, loc: u128, vars_data: &mut Map<u128>) -> Ptr {
  fn bind(rt: &mut Runtime, loc: u128, name: u128, lnk: Ptr, vars_data: &mut Map<u128>) {
    //println!("~~ bind {} {}", u128_to_name(name), show_lnk(lnk));
    if name == VAR_NONE {
      link(rt, loc, Era());
    } else {
      let got = vars_data.get(&name).map(|x| *x);
      match got {
        Some(got) => {
          vars_data.remove(&name);
          link(rt, got, lnk);
        }
        None => {
          vars_data.insert(name, lnk);
          link(rt, loc, Era());
        }
      }
    }
  }
  match term {
    Term::Var { name } => {
      //println!("~~ var {} {}", u128_to_name(*name), vars_data.len());
      let got = vars_data.get(name).map(|x| *x);
      match got {
        Some(got) => {
          vars_data.remove(name);
          return got;
        }
        None => {
          vars_data.insert(*name, loc);
          return Num(0);
        }
      }
    }
    Term::Dup { nam0, nam1, expr, body } => {
      let node = alloc(rt, 3);
      let dupk = rt.fresh_dups();
      bind(rt, node + 0, *nam0, Dp0(dupk, node), vars_data);
      bind(rt, node + 1, *nam1, Dp1(dupk, node), vars_data);
      let expr = create_term(rt, expr, node + 2, vars_data);
      link(rt, node + 2, expr);
      let body = create_term(rt, body, loc, vars_data);
      body
    }
    Term::Lam { name, body } => {
      let node = alloc(rt, 2);
      bind(rt, node + 0, *name, Var(node), vars_data);
      let body = create_term(rt, body, node + 1, vars_data);
      link(rt, node + 1, body);
      Lam(node)
    }
    Term::App { func, argm } => {
      let node = alloc(rt, 2);
      let func = create_term(rt, func, node + 0, vars_data);
      link(rt, node + 0, func);
      let argm = create_term(rt, argm, node + 1, vars_data);
      link(rt, node + 1, argm);
      App(node)
    }
    Term::Fun { name, args } => {
      if args.len() != rt.get_arity(*name) as usize {
        Num(0)
      } else {
        let size = args.len() as u128;
        let node = alloc(rt, size);
        for (i, arg) in args.iter().enumerate() {
          let arg_lnk = create_term(rt, arg, node + i as u128, vars_data);
          link(rt, node + i as u128, arg_lnk);
        }
        Fun(*name, node)
      }
    }
    Term::Ctr { name, args } => {
      if args.len() != rt.get_arity(*name) as usize {
        Num(0)
      } else {
        let size = args.len() as u128;
        let node = alloc(rt, size);
        for (i, arg) in args.iter().enumerate() {
          let arg_lnk = create_term(rt, arg, node + i as u128, vars_data);
          link(rt, node + i as u128, arg_lnk);
        }
        Ctr(*name, node)
      }
    }
    Term::Num { numb } => {
      // TODO: assert numb size
      Num(*numb as u128)
    }
    Term::Op2 { oper, val0, val1 } => {
      let node = alloc(rt, 2);
      let val0 = create_term(rt, val0, node + 0, vars_data);
      link(rt, node + 0, val0);
      let val1 = create_term(rt, val1, node + 1, vars_data);
      link(rt, node + 1, val1);
      Op2(*oper, node)
    }
  }
}

/// Given a Func (a vector of rules, lhs/rhs pairs), builds the CompFunc object
pub fn compile_func(func: &Func, debug: bool) -> Option<CompFunc> {
  let rules = &func.rules;

  // If there are no rules, return none
  if rules.len() == 0 {
    if debug {
      println!("  - failed to build function: no rules");
    }
    return None;
  }

  // Find the function arity
  let arity;
  if let Term::Fun { args, .. } = &rules[0].lhs {
    arity = args.len() as u128;
  } else {
    if debug {
      println!("  - failed to build function: left-hand side must be !(Fun ...)");
    }
    return None;
  }

  // The resulting vector
  let mut comp_rules = Vec::new();

  // A vector with the indices that are strict
  let mut strict = vec![false; arity as usize];

  // For each rule (lhs/rhs pair)
  for rule_index in 0 .. rules.len() {
    let rule = &func.rules[rule_index];

    // Validates that:
    // - the same lhs variable names aren't defined twice or more
    // - lhs variables are used linearly on the rhs
    let mut seen : HashSet<u128> = HashSet::new();
    fn check_var(name: u128, body: &Term, seen: &mut HashSet<u128>) -> bool {
      if seen.contains(&name) {
        return false;
      } else if name == VAR_NONE {
        return true;
      } else {
        seen.insert(name);
        return count_uses(body, name) == 1;
      }
    }

    let mut cond = Vec::new();
    let mut vars = Vec::new();
    let mut eras = Vec::new();

    // If the lhs is a Fun
    if let Term::Fun { ref name, ref args } = rule.lhs {

      // If there is an arity mismatch, return None
      if args.len() as u128 != arity {
        if debug {
          println!("  - failed to build function: arity mismatch on rule {}", rule_index);
        }
        return None;
      }

      // For each lhs argument
      for i in 0 .. args.len() as u128 {
        
        match &args[i as usize] {
          // If it is a constructor...
          Term::Ctr { name: arg_name, args: arg_args } => {
            strict[i as usize] = true;
            cond.push(Ctr(*arg_name, 0)); // adds its matching condition
            eras.push((i, arg_args.len() as u128)); // marks its index and arity for freeing
            // For each of its fields...
            for j in 0 .. arg_args.len() as u128 {
              // If it is a variable...
              if let Term::Var { name } = arg_args[j as usize] {
                if !check_var(name, &rule.rhs, &mut seen) {
                  if debug {
                    println!("  - failed to build function: non-linear variable '{}', on rule {}, argument {}:\n    {} = {}", u128_to_name(name), rule_index, i, view_term(&rule.lhs), view_term(&rule.rhs));
                  }
                  return None;
                } else {
                  vars.push(Var { name, param: i, field: Some(j), erase: name == VAR_NONE }); // add its location
                }
              // Otherwise..
              } else {
                if debug {
                  println!("  - failed to build function: nested match on rule {}, argument {}:\n    {} = {}", rule_index, i, view_term(&rule.lhs), view_term(&rule.rhs));
                }
                return None; // return none, because we don't allow nested matches
              }
            }
          }
          // If it is a number...
          Term::Num { numb: arg_numb } => {
            strict[i as usize] = true;
            cond.push(Num(*arg_numb as u128)); // adds its matching condition
          }
          // If it is a variable...
          Term::Var { name: arg_name } => {
            if !check_var(*arg_name, &rule.rhs, &mut seen) {
              if debug {
                println!("  - failed to build function: non-linear variable '{}', on rule {}, argument {}:\n    {} = {}", u128_to_name(*arg_name), rule_index, i, view_term(&rule.lhs), view_term(&rule.rhs));
              }
              return None;
            } else {
              vars.push(Var { name: *arg_name, param: i, field: None, erase: *arg_name == VAR_NONE }); // add its location
              cond.push(Var(0)); // it has no matching condition
            }
          }
          _ => {
            if debug {
              println!("  - failed to build function: unsupported match on rule {}, argument {}:\n    {} = {}", rule_index, i, view_term(&rule.lhs), view_term(&rule.rhs));
            }
            return None;
          }
        }
      }

    // If lhs isn't a Ctr, return None
    } else {
      if debug {
        println!("  - failed to build function: left-hand side isn't a constructor, on rule {}:\n    {} = {}", rule_index, view_term(&rule.lhs), view_term(&rule.rhs));
      }
      return None;
    }

    // Creates the rhs body
    let body = rule.rhs.clone();

    // Adds the rule to the result vector
    comp_rules.push(CompRule { cond, vars, eras, body });
  }

  // Builds the redux object, with the index of strict arguments
  let mut redux = Vec::new();
  for i in 0 .. strict.len() {
    if strict[i] {
      redux.push(i as u128);
    }
  }

  return Some(CompFunc {
    func: func.clone(),
    arity,
    redux,
    rules: comp_rules,
  });
}

pub fn create_app(rt: &mut Runtime, func: Ptr, argm: Ptr) -> Ptr {
  let node = alloc(rt, 2);
  link(rt, node + 0, func);
  link(rt, node + 1, argm);
  App(node)
}

pub fn create_fun(rt: &mut Runtime, fun: u128, args: &[Ptr]) -> Ptr {
  let node = alloc(rt, args.len() as u128);
  for i in 0 .. args.len() {
    link(rt, node + i as u128, args[i]);
  }
  Fun(fun, node)
}

pub fn alloc_lnk(rt: &mut Runtime, term: Ptr) -> u128 {
  let loc = alloc(rt, 1);
  link(rt, loc, term);
  return loc;
}

pub fn alloc_app(rt: &mut Runtime, func: Ptr, argm: Ptr) -> u128 {
  let app = create_app(rt, func, argm);
  return alloc_lnk(rt, app);
}

pub fn alloc_fun(rt: &mut Runtime, fun: u128, args: &[Ptr]) -> u128 {
  let fun = create_fun(rt, fun, args);
  return alloc_lnk(rt, fun);
}

// Reduction
// ---------

pub fn subst(rt: &mut Runtime, lnk: Ptr, val: Ptr) {
  if get_tag(lnk) != ERA {
    link(rt, get_loc(lnk, 0), val);
  } else {
    collect(rt, val);
  }
}

pub fn reduce(rt: &mut Runtime, root: u128, mana: u128) -> Result<Ptr, RuntimeError> {
  let mut vars_data: Map<u128> = init_map();

  let mut stack: Vec<u128> = Vec::new();

  let mut init = 1;
  let mut host = root;

  let mut func_val : Option<CompFunc>;
  let mut func_ref : Option<&mut CompFunc>;

  loop {
    let term = ask_lnk(rt, host);

    if rt.get_mana() > mana {
      return Err(RuntimeError::NotEnoughMana);
    }

    // if true {
    //   println!("----------------------");
    //   println!("{}", show_term(rt, ask_lnk(rt, root), Some(term)));
    // }

    if init == 1 {
      match get_tag(term) {
        APP => {
          stack.push(host);
          init = 1;
          host = get_loc(term, 0);
          continue;
        }
        DP0 | DP1 => {
          stack.push(host);
          host = get_loc(term, 2);
          continue;
        }
        OP2 => {
          stack.push(host);
          stack.push(get_loc(term, 1) | 0x1_0000_0000_0000);
          host = get_loc(term, 0);
          continue;
        }
        FUN => {
          let fun = get_ext(term);
          let ari = rt.get_arity(fun);
          if let Some(func) = &rt.get_func(fun) {
            if ari == func.arity {
              if func.redux.len() == 0 {
                init = 0;
              } else {
                stack.push(host);
                for (i, redux) in func.redux.iter().enumerate() {
                  if i < func.redux.len() - 1 {
                    stack.push(get_loc(term, *redux) | 0x1_0000_0000_0000);
                  } else {
                    host = get_loc(term, *redux);
                  }
                }
              }
              continue;
            }
          }
        }
        _ => {}
      }
    } else {
      match get_tag(term) {
        APP => {
          let arg0 = ask_arg(rt, term, 0);
          // (@x(body) a)
          // ------------ APP-LAM
          // x <- a
          // body
          if get_tag(arg0) == LAM {
            //println!("app-lam");
            rt.set_mana(rt.get_mana() + AppLamMana());
            rt.set_rwts(rt.get_rwts() + 1);
            subst(rt, ask_arg(rt, arg0, 0), ask_arg(rt, term, 1));
            let _done = link(rt, host, ask_arg(rt, arg0, 1));
            clear(rt, get_loc(term, 0), 2);
            clear(rt, get_loc(arg0, 0), 2);
            init = 1;
            continue;
          // ({a b} c)
          // ----------------- APP-SUP
          // dup x0 x1 = c
          // {(a x0) (b x1)}
          } else if get_tag(arg0) == SUP {
            //println!("app-sup");
            rt.set_mana(rt.get_mana() + AppSupMana());
            rt.set_rwts(rt.get_rwts() + 1);
            let app0 = get_loc(term, 0);
            let app1 = get_loc(arg0, 0);
            let let0 = alloc(rt, 3);
            let par0 = alloc(rt, 2);
            link(rt, let0 + 2, ask_arg(rt, term, 1));
            link(rt, app0 + 1, Dp0(get_ext(arg0), let0));
            link(rt, app0 + 0, ask_arg(rt, arg0, 0));
            link(rt, app1 + 0, ask_arg(rt, arg0, 1));
            link(rt, app1 + 1, Dp1(get_ext(arg0), let0));
            link(rt, par0 + 0, App(app0));
            link(rt, par0 + 1, App(app1));
            let done = Par(get_ext(arg0), par0);
            link(rt, host, done);
          } else {
            return Err(RuntimeError::TypeMismatch);
          }
        }
        DP0 | DP1 => {
          let arg0 = ask_arg(rt, term, 2);
          // dup r s = @x(f)
          // --------------- DUP-LAM
          // dup f0 f1 = f
          // r <- @x0(f0)
          // s <- @x1(f1)
          // x <- {x0 x1}
          if get_tag(arg0) == LAM {
            //println!("dup-lam");
            rt.set_mana(rt.get_mana() + DupLamMana());
            rt.set_rwts(rt.get_rwts() + 1);
            let let0 = get_loc(term, 0);
            let par0 = get_loc(arg0, 0);
            let lam0 = alloc(rt, 2);
            let lam1 = alloc(rt, 2);
            link(rt, let0 + 2, ask_arg(rt, arg0, 1));
            link(rt, par0 + 1, Var(lam1));
            let arg0_arg_0 = ask_arg(rt, arg0, 0);
            link(rt, par0 + 0, Var(lam0));
            subst(rt, arg0_arg_0, Par(get_ext(term), par0));
            let term_arg_0 = ask_arg(rt, term, 0);
            link(rt, lam0 + 1, Dp0(get_ext(term), let0));
            subst(rt, term_arg_0, Lam(lam0));
            let term_arg_1 = ask_arg(rt, term, 1);
            link(rt, lam1 + 1, Dp1(get_ext(term), let0));
            subst(rt, term_arg_1, Lam(lam1));
            let done = Lam(if get_tag(term) == DP0 { lam0 } else { lam1 });
            link(rt, host, done);
            init = 1;
            continue;
          // dup x y = {a b}
          // --------------- DUP-SUP-E
          // x <- a
          // y <- b
          } else if get_tag(arg0) == SUP {
            if get_ext(term) == get_ext(arg0) {
              //println!("dup-sup-e");
              rt.set_mana(rt.get_mana() + DupSupMana());
              rt.set_rwts(rt.get_rwts() + 1);
              subst(rt, ask_arg(rt, term, 0), ask_arg(rt, arg0, 0));
              subst(rt, ask_arg(rt, term, 1), ask_arg(rt, arg0, 1));
              let _done = link(rt, host, ask_arg(rt, arg0, if get_tag(term) == DP0 { 0 } else { 1 }));
              clear(rt, get_loc(term, 0), 3);
              clear(rt, get_loc(arg0, 0), 2);
              init = 1;
              continue;
            // dup x y = {a b}
            // ----------------- DUP-SUP-D
            // x <- {xA xB}
            // y <- {yA yB}
            // dup xA yA = a
            // dup xB yB = b
            } else {
              //println!("dup-sup-d");
              rt.set_mana(rt.get_mana() + DupDupMana());
              rt.set_rwts(rt.get_rwts() + 1);
              let par0 = alloc(rt, 2);
              let let0 = get_loc(term, 0);
              let par1 = get_loc(arg0, 0);
              let let1 = alloc(rt, 3);
              link(rt, let0 + 2, ask_arg(rt, arg0, 0));
              link(rt, let1 + 2, ask_arg(rt, arg0, 1));
              let term_arg_0 = ask_arg(rt, term, 0);
              let term_arg_1 = ask_arg(rt, term, 1);
              link(rt, par1 + 0, Dp1(get_ext(term), let0));
              link(rt, par1 + 1, Dp1(get_ext(term), let1));
              link(rt, par0 + 0, Dp0(get_ext(term), let0));
              link(rt, par0 + 1, Dp0(get_ext(term), let1));
              subst(rt, term_arg_0, Par(get_ext(arg0), par0));
              subst(rt, term_arg_1, Par(get_ext(arg0), par1));
              let done = Par(get_ext(arg0), if get_tag(term) == DP0 { par0 } else { par1 });
              link(rt, host, done);
            }
          // dup x y = N
          // ----------- DUP-NUM
          // x <- N
          // y <- N
          // ~
          } else if get_tag(arg0) == NUM {
            //println!("dup-num");
            rt.set_mana(rt.get_mana() + DupNumMana());
            rt.set_rwts(rt.get_rwts() + 1);
            subst(rt, ask_arg(rt, term, 0), arg0);
            subst(rt, ask_arg(rt, term, 1), arg0);
            clear(rt, get_loc(term, 0), 3);
            let _done = arg0;
            link(rt, host, arg0);
          // dup x y = (K a b c ...)
          // ----------------------- DUP-CTR
          // dup a0 a1 = a
          // dup b0 b1 = b
          // dup c0 c1 = c
          // ...
          // x <- (K a0 b0 c0 ...)
          // y <- (K a1 b1 c1 ...)
          } else if get_tag(arg0) == CTR {
            //println!("dup-ctr");
            let func = get_ext(arg0);
            let arit = rt.get_arity(func);
            rt.set_mana(rt.get_mana() + DupCtrMana(arit));
            rt.set_rwts(rt.get_rwts() + 1);
            if arit == 0 {
              subst(rt, ask_arg(rt, term, 0), Ctr(func, 0));
              subst(rt, ask_arg(rt, term, 1), Ctr(func, 0));
              clear(rt, get_loc(term, 0), 3);
              let _done = link(rt, host, Ctr(func, 0));
            } else {
              let ctr0 = get_loc(arg0, 0);
              let ctr1 = alloc(rt, arit);
              for i in 0..arit - 1 {
                let leti = alloc(rt, 3);
                link(rt, leti + 2, ask_arg(rt, arg0, i));
                link(rt, ctr0 + i, Dp0(get_ext(term), leti));
                link(rt, ctr1 + i, Dp1(get_ext(term), leti));
              }
              let leti = get_loc(term, 0);
              link(rt, leti + 2, ask_arg(rt, arg0, arit - 1));
              let term_arg_0 = ask_arg(rt, term, 0);
              link(rt, ctr0 + arit - 1, Dp0(get_ext(term), leti));
              subst(rt, term_arg_0, Ctr(func, ctr0));
              let term_arg_1 = ask_arg(rt, term, 1);
              link(rt, ctr1 + arit - 1, Dp1(get_ext(term), leti));
              subst(rt, term_arg_1, Ctr(func, ctr1));
              let done = Ctr(func, if get_tag(term) == DP0 { ctr0 } else { ctr1 });
              link(rt, host, done);
            }
          // dup x y = *
          // ----------- DUP-ERA
          // x <- *
          // y <- *
          } else if get_tag(arg0) == ERA {
            //println!("dup-era");
            rt.set_mana(rt.get_mana() + DupEraMana());
            rt.set_rwts(rt.get_rwts() + 1);
            subst(rt, ask_arg(rt, term, 0), Era());
            subst(rt, ask_arg(rt, term, 1), Era());
            link(rt, host, Era());
            clear(rt, get_loc(term, 0), 3);
            init = 1;
            continue;
          } else {
            return Err(RuntimeError::TypeMismatch);
          }
        }
        OP2 => {
          let arg0 = ask_arg(rt, term, 0);
          let arg1 = ask_arg(rt, term, 1);
          // (+ a b)
          // --------- OP2-NUM
          // add(a, b)
          if get_tag(arg0) == NUM && get_tag(arg1) == NUM {
            //eprintln!("op2-num");
            rt.set_mana(rt.get_mana() + Op2NumMana());
            let op  = get_ext(term);
            let a_u = get_num(arg0);
            let b_u = get_num(arg1);
            let res = match op {
              // U120
              ADD => a_u.wrapping_add(b_u) & NUM_MASK,
              SUB => a_u.wrapping_sub(b_u) & NUM_MASK,
              MUL => a_u.wrapping_mul(b_u) & NUM_MASK,
              DIV => a_u.wrapping_div(b_u) & NUM_MASK,
              MOD => a_u.wrapping_rem(b_u) & NUM_MASK,
              AND => (a_u &  b_u) & NUM_MASK,
              OR  => (a_u |  b_u) & NUM_MASK,
              XOR => (a_u ^  b_u) & NUM_MASK,
              SHL => a_u.wrapping_shl(b_u as u32) & NUM_MASK,
              SHR => a_u.wrapping_shr(b_u as u32) & NUM_MASK,
              LTN => u128::from(a_u <  b_u),
              LTE => u128::from(a_u <= b_u),
              EQL => u128::from(a_u == b_u),
              GTE => u128::from(a_u >= b_u),
              GTN => u128::from(a_u >  b_u),
              NEQ => u128::from(a_u != b_u),
              _ => panic!("Invalid operation!"),
            };
            let done = Num(res);
            clear(rt, get_loc(term, 0), 2);
            link(rt, host, done);
          // (+ {a0 a1} b)
          // --------------------- OP2-SUP-0
          // let b0 b1 = b
          // {(+ a0 b0) (+ a1 b1)}
          } else if get_tag(arg0) == SUP {
            //println!("op2-sup-0");
            rt.set_mana(rt.get_mana() + Op2SupMana());
            rt.set_rwts(rt.get_rwts() + 1);
            let op20 = get_loc(term, 0);
            let op21 = get_loc(arg0, 0);
            let let0 = alloc(rt, 3);
            let par0 = alloc(rt, 2);
            link(rt, let0 + 2, arg1);
            link(rt, op20 + 1, Dp0(get_ext(arg0), let0));
            link(rt, op20 + 0, ask_arg(rt, arg0, 0));
            link(rt, op21 + 0, ask_arg(rt, arg0, 1));
            link(rt, op21 + 1, Dp1(get_ext(arg0), let0));
            link(rt, par0 + 0, Op2(get_ext(term), op20));
            link(rt, par0 + 1, Op2(get_ext(term), op21));
            let done = Par(get_ext(arg0), par0);
            link(rt, host, done);
          // (+ a {b0 b1})
          // --------------- OP2-SUP-1
          // dup a0 a1 = a
          // {(+ a0 b0) (+ a1 b1)}
          } else if get_tag(arg1) == SUP {
            //println!("op2-sup-1");
            rt.set_mana(rt.get_mana() + Op2SupMana());
            rt.set_rwts(rt.get_rwts() + 1);
            let op20 = get_loc(term, 0);
            let op21 = get_loc(arg1, 0);
            let let0 = alloc(rt, 3);
            let par0 = alloc(rt, 2);
            link(rt, let0 + 2, arg0);
            link(rt, op20 + 0, Dp0(get_ext(arg1), let0));
            link(rt, op20 + 1, ask_arg(rt, arg1, 0));
            link(rt, op21 + 1, ask_arg(rt, arg1, 1));
            link(rt, op21 + 0, Dp1(get_ext(arg1), let0));
            link(rt, par0 + 0, Op2(get_ext(term), op20));
            link(rt, par0 + 1, Op2(get_ext(term), op21));
            let done = Par(get_ext(arg1), par0);
            link(rt, host, done);
          } else {
            return Err(RuntimeError::TypeMismatch);
          }
        }
        FUN => {

          fn call_function(rt: &mut Runtime, func: Arc<CompFunc>, host: u128, term: Ptr, mana: u128, vars_data: &mut Map<u128>) -> bool {
            // For each argument, if it is a redex and a SUP, apply the cal_par rule
            for idx in &func.redux {
              // (F {a0 a1} b c ...)
              // ------------------- FUN-SUP
              // dup b0 b1 = b
              // dup c0 c1 = c
              // ...
              // {(F a0 b0 c0 ...) (F a1 b1 c1 ...)}
              if get_tag(ask_arg(rt, term, *idx)) == SUP {
                //println!("fun-sup");
                let funx = get_ext(term);
                let arit = rt.get_arity(funx);
                rt.set_mana(rt.get_mana() + FunSupMana(arit));
                rt.set_rwts(rt.get_rwts() + 1);
                let argn = ask_arg(rt, term, *idx);
                let fun0 = get_loc(term, 0);
                let fun1 = alloc(rt, arit);
                let par0 = get_loc(argn, 0);
                for i in 0..arit {
                  if i != *idx {
                    let leti = alloc(rt, 3);
                    let argi = ask_arg(rt, term, i);
                    link(rt, fun0 + i, Dp0(get_ext(argn), leti));
                    link(rt, fun1 + i, Dp1(get_ext(argn), leti));
                    link(rt, leti + 2, argi);
                  } else {
                    link(rt, fun0 + i, ask_arg(rt, argn, 0));
                    link(rt, fun1 + i, ask_arg(rt, argn, 1));
                  }
                }
                link(rt, par0 + 0, Fun(funx, fun0));
                link(rt, par0 + 1, Fun(funx, fun1));
                let done = Par(get_ext(argn), par0);
                link(rt, host, done);
                return true;
              }
            }
            // For each rule condition vector
            for rule in &func.rules {
              // Check if the rule matches
              let mut matched = true;
              //println!("- matching rule");
              // Tests each rule condition (ex: `get_tag(args[0]) == SUCC`)
              for i in 0 .. rule.cond.len() as u128 {
                let cond = rule.cond[i as usize];
                match get_tag(cond) {
                  NUM => {
                    //println!("Didn't match because of NUM. i={} {} {}", i, get_val(ask_arg(rt, term, i)), get_val(cond));
                    let same_tag = get_tag(ask_arg(rt, term, i)) == NUM;
                    let same_val = get_val(ask_arg(rt, term, i)) == get_val(cond);
                    matched = matched && same_tag && same_val;
                  }
                  CTR => {
                    //println!("Didn't match because of CTR. i={} {} {}", i, get_tag(ask_arg(rt, term, i)), get_val(cond));
                    let same_tag = get_tag(ask_arg(rt, term, i)) == CTR;
                    let same_ext = get_ext(ask_arg(rt, term, i)) == get_ext(cond);
                    matched = matched && same_tag && same_ext;
                  }
                  VAR => {
                    if func.redux.contains(&i) {
                      let not_var = get_tag(ask_arg(rt, term, i)) > VAR;
                      matched = matched && not_var;
                    }
                  }
                  _ => {}
                }
              }
              // (user-defined)
              // -------------- FUN-CTR
              // (user-defined)
              // If all conditions are satisfied, the rule matched, so we must apply it
              if matched {
                //println!("fun-ctr");
                //println!("- matched");
                // Increments the gas count
                rt.set_mana(rt.get_mana() + FunCtrMana(&rule.body));
                rt.set_rwts(rt.get_rwts() + 1);
                // Gathers matched variables
                //let mut vars = vec![None; 16]; // FIXME: pre-alloc statically
                for (i, rule_var) in rule.vars.iter().enumerate() {
                  let mut var = term;
                  var = ask_arg(rt, var, rule_var.param);
                  if let Some(field) = rule_var.field {
                    var = ask_arg(rt, var, field);
                  }
                  //eprintln!("~~ set {} {}", u128_to_name(rule_var.name), show_lnk(var));
                  if !rule_var.erase {
                    vars_data.insert(rule_var.name, var);
                  } else {
                    // Collects unused argument
                    collect(rt, var);
                  }
                }
                // Builds the right-hand side term (ex: `(Succ (Add a b))`)
                //println!("-- vars: {:?}", vars);
                let done = create_term(rt, &rule.body, host, vars_data);
                // Links the host location to it
                link(rt, host, done);
                // Clears the matched ctrs (the `(Succ ...)` and the `(Add ...)` ctrs)
                for (eras_index, eras_arity) in &rule.eras {
                  clear(rt, get_loc(ask_arg(rt, term, *eras_index), 0), *eras_arity);
                }
                clear(rt, get_loc(term, 0), func.arity);
                // // Collects unused variables (none in this example)
                // for i in 0 .. rule.vars.len() {
                //   if rule.vars[i].erase {
                //     if let Some(var) = vars_data.get(&(i as u64)) {
                //       collect(rt, *var, mana)?;
                //     }
                //   }
                // }
                return true;
              }
            }
            return false;
          }

          let fun = get_ext(term);
          if let Some(func) = rt.get_func(fun) {
            if call_function(rt, func, host, term, mana, &mut vars_data) {
              init = 1;
              continue;
            } else {
              return Err(RuntimeError::TypeMismatch);
            }
          }

        }
        _ => {}
      }
    }

    if let Some(item) = stack.pop() {
      init = item >> 48;
      host = item & 0x0_FFFF_FFFF_FFFF;
      continue;
    }

    break;
  }

  // FIXME: remove this when Runtime is split (see above)
  //rt.get_heap_mut(self.curr).file = file;

  return Ok(ask_lnk(rt, root));
}

/// Evaluates redexes iteratively. This is used to save space before storing a term, since,
/// otherwise, chunks would grow indefinitely due to lazy evaluation. It does not reduce the term to
/// normal form, though, since it stops on whnfs. If it did, then storing a state wouldn't be O(1),
/// since it would require passing over the entire state.
pub fn compute_at(rt: &mut Runtime, host: u128, mana: u128) -> Result<Ptr, RuntimeError> {
  enum StackItem {
    LinkResolver(u128),
    Host(u128, u128)
  }
  let mut stack = vec![StackItem::Host(host, mana)];
  let mut output = vec![];
  while !stack.is_empty() {
    let item = stack.pop().unwrap();
    match item {
      StackItem::Host(host, mana) => {
        let term = ask_lnk(rt, host);
        let norm = reduce(rt, host, mana)?;

        if term == norm {
          output.push(Some(term));
        } else {
          match get_tag(norm) {
            LAM => {
              let loc_1 = get_loc(norm, 1);
              stack.push(StackItem::LinkResolver(loc_1));
              stack.push(StackItem::Host(loc_1, mana));
            }
            APP => {
              let loc_0 = get_loc(norm, 0);
              let loc_1 = get_loc(norm, 1);
              stack.push(StackItem::LinkResolver(loc_1));
              stack.push(StackItem::Host(loc_1, mana));
              stack.push(StackItem::LinkResolver(loc_0));
              stack.push(StackItem::Host(loc_0, mana));
            }
            SUP => {
              let loc_0 = get_loc(norm, 0);
              let loc_1 = get_loc(norm, 1);
              stack.push(StackItem::LinkResolver(loc_1));
              stack.push(StackItem::Host(loc_1, mana));
              stack.push(StackItem::LinkResolver(loc_0));
              stack.push(StackItem::Host(loc_0, mana));
            }
            DP0 => {
              let loc_2 = get_loc(norm, 2);
              stack.push(StackItem::LinkResolver(loc_2));
              stack.push(StackItem::Host(loc_2, mana));
            }
            DP1 => {
              let loc_2 = get_loc(norm, 2);
              stack.push(StackItem::LinkResolver(loc_2));
              stack.push(StackItem::Host(loc_2, mana));
            }
            CTR | FUN => {
              for i in (0..rt.get_arity(get_ext(norm))).rev() {
                let loc_i = get_loc(norm, i);
                stack.push(StackItem::LinkResolver(loc_i));
                stack.push(StackItem::Host(loc_i, mana));
              }
            }
            _ => {}
          };
          output.push(Some(norm));
        }
      },
      StackItem::LinkResolver(loc) => {
        match output.pop() {
          Some(lnk) => {
            if let Some(lnk) = lnk {
              link(rt, loc, lnk);
            }
          }
          None => panic!("No term to resolve link"),
        }
      }
    }
  }
  // FIXME: is this always safe? if no, create a runtime error for what could go wrong
  Ok(output.pop().unwrap().unwrap())
}

// Debug
// -----

pub fn show_lnk(x: Ptr) -> String {
  if x == 0 {
    String::from("~")
  } else {
    let tag = get_tag(x);
    let ext = get_ext(x);
    let val = get_val(x);
    let tgs = match tag {
      DP0 => "DP0",
      DP1 => "DP1",
      VAR => "VAR",
      ARG => "ARG",
      ERA => "ERA",
      LAM => "LAM",
      APP => "APP",
      SUP => "SUP",
      CTR => "CTR",
      FUN => "FUN",
      OP2 => "OP2",
      NUM => "NUM",
      _   => "?",
    };
    format!("{}:{}:{:x}", tgs, u128_to_name(ext), val)
  }
}

pub fn show_rt(rt: &Runtime) -> String {
  let mut s: String = String::new();
  for i in 0..32 {
    // pushes to the string
    write!(s, "{:x} | ", i).unwrap();
    s.push_str(&show_lnk(rt.read(i)));
    s.push('\n');
  }
  s
}

fn show_memo(rt: &Runtime) -> String {
  let mut txt = String::new();
  for i in 0 .. rt.get_mcap() {
    txt.push(if rt.read(i) == 0 { '_' } else { 'X' });
  }
  return txt;
}

// TODO: this should be replaced by readback + view_term
pub fn show_term(rt: &Runtime, term: Ptr, focus: Option<u128>) -> String {
  enum StackItem {
    Term(Ptr),
    Str(String),
  }
  let mut names: HashMap<u128, String> = HashMap::new();
  fn find_lets(
    rt: &Runtime,
    term: Ptr,
    names: &mut HashMap<u128, String>,
    focus: Option<u128>
  ) -> String {
    let mut lets: HashMap<u128, u128> = HashMap::new();
    let mut kinds: HashMap<u128, u128> = HashMap::new();
    let mut count: u128 = 0;
    let mut stack = vec![term];
    let mut text = String::new();
    while !stack.is_empty() { 
      let term = stack.pop().unwrap();
      match get_tag(term) {
        LAM => {
          names.insert(get_loc(term, 0), format!("{}", count));
          count += 1;
          stack.push(ask_arg(rt, term, 1));
        }
        APP => {
          stack.push(ask_arg(rt, term, 1));
          stack.push(ask_arg(rt, term, 0));
        }
        SUP => {
          stack.push(ask_arg(rt, term, 1));
          stack.push(ask_arg(rt, term, 0));
        }
        DP0 => {
          if let hash_map::Entry::Vacant(e) = lets.entry(get_loc(term, 0)) {
            names.insert(get_loc(term, 0), format!("{}", count));
            count += 1;
            kinds.insert(get_loc(term, 0), get_ext(term));
            e.insert(get_loc(term, 0));
            stack.push(ask_arg(rt, term, 2));
          }
        }
        DP1 => {
          if let hash_map::Entry::Vacant(e) = lets.entry(get_loc(term, 0)) {
            names.insert(get_loc(term, 0), format!("{}", count));
            count += 1;
            kinds.insert(get_loc(term, 0), get_ext(term));
            e.insert(get_loc(term, 0));
            stack.push(ask_arg(rt, term, 2));
          }
        }
        OP2 => {
          stack.push(ask_arg(rt, term, 1));
          stack.push(ask_arg(rt, term, 0));
        }
        CTR | FUN => {
          let arity = rt.get_arity(get_ext(term));
          for i in (0..arity).rev() {
            stack.push(ask_arg(rt, term, i));
          }
        }
        _ => {}
      }
    }

    for (_key, pos) in lets {
      // todo: reverse
      let what = String::from("?h");
      //let kind = kinds.get(&key).unwrap_or(&0);
      let name = names.get(&pos).unwrap_or(&what);
      let nam0 = if ask_lnk(rt, pos + 0) == Era() { String::from("*") } else { format!("a{}", name) };
      let nam1 = if ask_lnk(rt, pos + 1) == Era() { String::from("*") } else { format!("b{}", name) };
      writeln!(text, "dup {} {} = {};\n", nam0, nam1, go(rt, ask_lnk(rt, pos + 2), &names, focus)).unwrap();
    }
    text
  }

  fn go(rt: &Runtime, term: Ptr, names: &HashMap<u128, String>, focus: Option<u128>) -> String {
    let mut stack = vec![StackItem::Term(term)];
    let mut output = Vec::new();
    while !stack.is_empty() {
      let item = stack.pop().unwrap();
      match item {
        StackItem::Str(txt) => {
          output.push(txt);
        },
        StackItem::Term(term) => {
          if let Some(focus) = focus {
            if focus == term {
              output.push("$".to_string());
            }
          }
          match get_tag(term) {
            DP0 => {
              output.push(format!("a{}", names.get(&get_loc(term, 0)).unwrap_or(&String::from("?a"))));
            }
            DP1 => {
              output.push(format!("b{}", names.get(&get_loc(term, 0)).unwrap_or(&String::from("?b"))));
            }
            VAR => {
              output.push(format!("x{}", names.get(&get_loc(term, 0)).unwrap_or(&String::from("?c"))));
            }
            LAM => {
              let name = format!("x{}", names.get(&get_loc(term, 0)).unwrap_or(&String::from("?")));
              output.push(format!("@{}", name));
              stack.push(StackItem::Term(ask_arg(rt, term, 1)));
            }
            APP => {
              output.push("(".to_string());
              stack.push(StackItem::Str(")".to_string()));
              stack.push(StackItem::Term(ask_arg(rt, term, 1)));
              stack.push(StackItem::Str(" ".to_string()));
              stack.push(StackItem::Term(ask_arg(rt, term, 0)));
            }
            SUP => {
              output.push("{".to_string());
              stack.push(StackItem::Str("}".to_string()));
              //let kind = get_ext(term);
              stack.push(StackItem::Term(ask_arg(rt, term, 1)));
              stack.push(StackItem::Str(" ".to_string()));
              stack.push(StackItem::Term(ask_arg(rt, term, 0)));
            }
            OP2 => {
              let oper = get_ext(term);
              let symb = match oper {
                ADD => "+",
                SUB => "-",
                MUL => "*",
                DIV => "/",
                MOD => "%",
                AND => "&",
                OR  => "|",
                XOR => "^",
                SHL => "<<",
                SHR => ">>",
                LTN => "<",
                LTE => "<=",
                EQL => "=",
                GTE => ">=",
                GTN => ">",
                NEQ => "!=",
                _        => "?",
              };
              output.push(format!("({} ", symb));
              stack.push(StackItem::Str(")".to_string()));
              stack.push(StackItem::Term(ask_arg(rt, term, 1)));
              stack.push(StackItem::Str(" ".to_string()));
              stack.push(StackItem::Term(ask_arg(rt, term, 0)));
            }
            NUM => {
              let numb = get_num(term);
              output.push(format!("#{}", numb));
            }
            CTR => {
              let name = get_ext(term);
              let mut arit = rt.get_arity(name);
              let mut name = view_name(name);
              // Pretty print names
              if name == "Name" && arit == 1 {
                let arg = ask_arg(rt, term, 0);
                if get_tag(arg) == NUM {
                  name = format!("Name '{}'", view_name(get_num(arg)));
                  arit = 0; // erase arit to avoid for
                }
              }
              output.push(format!("{{{}", name));
              stack.push(StackItem::Str("}".to_string()));
              
              for i in (0..arit).rev() {
                stack.push(StackItem::Term(ask_arg(rt, term, i)));
                stack.push(StackItem::Str(" ".to_string()));
      
              }
            }
            FUN => {
              let func = get_ext(term);
              output.push(format!("({}", u128_to_name(func)));
              stack.push(StackItem::Str(")".to_string()));
              let arit = rt.get_arity(func);
              for i in (0..arit).rev() {
                stack.push(StackItem::Term(ask_arg(rt, term, i)));
                stack.push(StackItem::Str(" ".to_string()));
              }
            }
            ERA => {
              output.push(String::from("*"));
            }
            _ => output.push(format!("?g({})", get_tag(term))),
          }
        }
      }
    }

    let res = output.join("");
    return res;

  }

  let mut text = find_lets(rt, term, &mut names, focus);
  text.push_str( &go(rt, term, &names, focus));
  text
}

fn show_runtime_error(err: RuntimeError) -> String {
  (match err {
    RuntimeError::NotEnoughMana => "Not enough mana.",
    RuntimeError::NotEnoughSpace => "Not enough space.",
    RuntimeError::TypeMismatch => "Runtime type mismatch.",
    RuntimeError::EffectFailure => "Runtime effect failure."
  }).to_string()
}

// FIXME: This is NOT the readback function. I didn't notice it before. This is just the debug
// stringification function, converted to return a term instead. There is a crucial difference: the
// proper readback function does NOT return dups, i.e., it resolves pending dups, returning a
// conventional lambda term. Seems like we removed the proper readback function! We must restore
// it, and optimize it to use stacks instead of recursion. The original readback function can be
// found on the HVM repository. 
pub fn readback_linear_term(rt: &Runtime, term: Ptr) -> Term {
  enum StackItem {
    Term(Ptr),
    Resolver(Ptr),
  }

  fn dups(rt: &Runtime, term: Ptr, names: &mut HashMap<u128, String>) -> Term {
    let mut lets: HashMap<u128, u128> = HashMap::new();
    let mut kinds: HashMap<u128, u128> = HashMap::new();
    let mut count: u128 = 0;
    let mut stack = vec![term];
    while !stack.is_empty() {
      let term = stack.pop().unwrap();
      match get_tag(term) {
        LAM => {
          names.insert(get_loc(term, 0), format!("{}", count));
          count += 1;
          stack.push(ask_arg(rt, term, 1));
        }
        APP => {
          stack.push(ask_arg(rt, term, 1));
          stack.push(ask_arg(rt, term, 0));
        }
        SUP => {
          stack.push(ask_arg(rt, term, 1));
          stack.push(ask_arg(rt, term, 0));
        }
        DP0 => {
          if let hash_map::Entry::Vacant(e) = lets.entry(get_loc(term, 0)) {
            names.insert(get_loc(term, 0), format!("{}", count));
            count += 1;
            kinds.insert(get_loc(term, 0), get_ext(term));
            e.insert(get_loc(term, 0));
            stack.push(ask_arg(rt, term, 2));
          }
        }
        DP1 => {
          if let hash_map::Entry::Vacant(e) = lets.entry(get_loc(term, 0)) {
            names.insert(get_loc(term, 0), format!("{}", count));
            count += 1;
            kinds.insert(get_loc(term, 0), get_ext(term));
            e.insert(get_loc(term, 0));
            stack.push(ask_arg(rt, term, 2));
          }
        }
        OP2 => {
          stack.push(ask_arg(rt, term, 1));
          stack.push(ask_arg(rt, term, 0));
        }
        CTR | FUN => {
          let arity = rt.get_arity(get_ext(term));
          for i in (0..arity).rev() {
            stack.push(ask_arg(rt, term, i));
          }
        }
        _ => {}
      }
    }

    let cont = expr(rt, term, &names);
    if lets.is_empty() {
      cont
    } else {
      let mut output = Term::Var { name: 0 };
      for (i, (_key, pos)) in lets.iter().enumerate() {
        // todo: reverse
        let what = String::from("?h");
        let name = names.get(&pos).unwrap_or(&what);
        let nam0 = if ask_lnk(rt, pos + 0) == Era() { String::from("*") } else { format!("a{}", name) };
        let nam1 = if ask_lnk(rt, pos + 1) == Era() { String::from("*") } else { format!("b{}", name) };
        let expr = expr(rt, ask_lnk(rt, pos + 2), &names);
        if i == 0 {
          output = Term::Dup { nam0: name_to_u128(&nam0), nam1: name_to_u128(&nam1), expr: Box::new(expr), body: Box::new(cont.clone()) };
        } else {
          output = Term::Dup { nam0: name_to_u128(&nam0), nam1: name_to_u128(&nam1), expr: Box::new(expr), body: Box::new(output) };
        }
      }
      output
    }
  }

  fn expr(rt: &Runtime, term: Ptr, names: &HashMap<u128, String>) -> Term {
    let mut stack = vec![StackItem::Term(term)];
    let mut output = Vec::new();
    while !stack.is_empty() {
      let item = stack.pop().unwrap();
      match item {
        StackItem::Resolver(term) => {
          match get_tag(term) {
            CTR => {
              let func = get_ext(term);
              let arit = rt.get_arity(func);
              let mut args = Vec::new();
              for i in 0..arit {
                args.push(output.pop().unwrap());
              }
              output.push(Term::Ctr { name: func, args });
            },
            LAM => {
              let name = format!("x{}", names.get(&get_loc(term, 0)).unwrap_or(&String::from("?")));
              let body = Box::new(output.pop().unwrap());
              output.push(Term::Lam { name: name_to_u128(&name), body });
            }
            APP => {
              let argm = Box::new(output.pop().unwrap());
              let func = Box::new(output.pop().unwrap());
              output.push(Term::App { func , argm });
            }
            OP2 => {
              let oper = get_ext(term);
              let val1 = Box::new(output.pop().unwrap());
              let val0 = Box::new(output.pop().unwrap());
              output.push(Term::Op2 { oper, val0, val1 })
            }
            FUN => {
              let func = get_ext(term);
              let arit = rt.get_arity(func);
              let mut args = Vec::new();
              for i in 0..arit {
                args.push(output.pop().unwrap());
              }
              output.push(Term::Fun { name: func, args });
            }
            _ => panic!("Term not valid in readback"),
          }
        },
        StackItem::Term(term) => {
          match get_tag(term) {
            DP0 => {
              let name = format!("a{}", names.get(&get_loc(term, 0)).unwrap_or(&String::from("?a")));
              output.push(Term::Var { name: name_to_u128(&name) });
            }
            DP1 => {
              let name = format!("b{}", names.get(&get_loc(term, 0)).unwrap_or(&String::from("?b")));
              output.push(Term::Var { name: name_to_u128(&name) });
            }
            VAR => {
              let name = format!("x{}", names.get(&get_loc(term, 0)).unwrap_or(&String::from("?x")));
              output.push(Term::Var { name: name_to_u128(&name) });
            }
            LAM => {
              stack.push(StackItem::Resolver(term));
              stack.push(StackItem::Term(ask_arg(rt, term, 1)));
            }
            APP => {
              stack.push(StackItem::Resolver(term));
              stack.push(StackItem::Term(ask_arg(rt, term, 1)));
              stack.push(StackItem::Term(ask_arg(rt, term, 0)));
            }
            SUP => {}
            OP2 => {
              stack.push(StackItem::Resolver(term));
              stack.push(StackItem::Term(ask_arg(rt, term, 1)));
              stack.push(StackItem::Term(ask_arg(rt, term, 0)));
            }
            NUM => {
              let numb = get_num(term);
              output.push(Term::Num { numb });
            }
            CTR => {
              let func = get_ext(term);
              let arit = rt.get_arity(func);
              stack.push(StackItem::Resolver(term));
              for i in 0..arit {
                stack.push(StackItem::Term(ask_arg(rt, term, i)));
              }
            }
            FUN => {
              let func = get_ext(term);
              let arit = rt.get_arity(func);
              stack.push(StackItem::Resolver(term));
              for i in 0..arit {
                stack.push(StackItem::Term(ask_arg(rt, term, i)));
              }
            }
            ERA => {}
            _ => {}
          }
        }
      }
    }
    output.pop().unwrap()
  }

  let mut names: HashMap<u128, String> = HashMap::new();
  dups(rt, term, &mut names)
}

// Parsing
// -------

fn head(code: &str) -> char {
  return code.chars().take(1).last().unwrap_or('\0');
}

fn tail(code: &str) -> &str {
  if code.len() > 0 {
    return &code[head(code).len_utf8()..];
  } else {
    return "";
  }
}

fn drop(code: &str, amount: u128) -> &str {
  let mut code = code;
  for _ in 0 .. amount {
    code = tail(code);
  }
  return code;
}

fn nth(code: &str, index: u128) -> char {
  return head(drop(code, index));
}

fn skip(code: &str) -> &str {
  let mut code = code;
  loop {
    if head(code) == ' ' || head(code) == '\n' {
      while head(code) == ' ' || head(code) == '\n' {
        code = tail(code);
      }
      continue;
    }
    if head(code) == '/' && nth(code,1) == '/' {
      while head(code) != '\n' && head(code) != '\0' {
        code = tail(code);
      }
      continue;
    }
    break;
  }
  return code;
}

fn hash(name: &str) -> u128 {
  let mut hasher = hash_map::DefaultHasher::new();
  name.hash(&mut hasher);
  hasher.finish() as u128
}

fn is_name_char(chr: char) -> bool {
  return chr == '_' || chr == '.'
      || chr >= 'a' && chr <= 'z'
      || chr >= 'A' && chr <= 'Z'
      || chr >= '0' && chr <= '9';
}

pub fn read_char(code: &str, chr: char) -> ParseResult<()> {
  let code = skip(code);
  if head(code) == chr {
    Ok((tail(code), ()))
  } else {
    Err(ParseErr {
      erro: format!("Expected '{}', found '{}'. Context:\n\x1b[2m{}\x1b[0m", chr, head(code), code.chars().take(64).collect::<String>()),
      code: code.to_string(),
    })
  }
}

pub fn read_numb(code: &str) -> ParseResult<u128> {
  let mut code = skip(code);
  if head(code) == 'x' {
    code = tail(code);
    let mut numb = 0;
    let mut code = code;
    loop {
      if head(code) >= '0' && head(code) <= '9' {
        numb = numb * 16 + head(code) as u128 - 0x30;
        code = tail(code);
      } else if head(code) >= 'a' && head(code) <= 'f' {
        numb = numb * 16 + head(code) as u128 - 0x61 + 10;
        code = tail(code);
      } else if head(code) >= 'A' && head(code) <= 'F' {
        numb = numb * 16 + head(code) as u128 - 0x41 + 10;
        code = tail(code);
      } else {
        break;
      }
    }
    return Ok((code, numb));
  } else {
    let mut numb = 0;
    while head(code) >= '0' && head(code) <= '9' {
      numb = numb * 10 + head(code) as u128 - 0x30;
      code = tail(code);
    }
    return Ok((code, numb));
  }
}

pub fn read_name(code: &str) -> ParseResult<u128> {
  let code = skip(code);
  let mut name = String::new();
  if head(code) == '~' {
    return Ok((tail(code), VAR_NONE));
  } else {
    let mut code = code;
    while is_name_char(head(code)) {
      name.push(head(code));
      code = tail(code);
    }
    if name.is_empty() {
      return Err(ParseErr {
        code: code.to_string(),
        erro: format!("Expected identifier, found `{}`.", head(code))
      });
    }
    // TODO: check identifier size and propagate error
    if name.len() > 20 {
      return Err(ParseErr {
        code: code.to_string(),
        erro: format!("Identifier too long: {}", name)
      });
    }
    if ('0'..='9').contains(&name.chars().nth(0).unwrap_or(' ')) {
      // In most cases, an user writing 0-9 probably wants to make a number, not a name, so, to
      // avoid mistakes, we disable this syntax by default. But since names CAN start with 0-9 on
      // Kindelia, we must create an alternative, explicit way to parse "numeric names".
      return Err(ParseErr {
        code: code.to_string(),
        erro: format!("Number must start with #, but '{}' doesn't.", name),
      });
    }
    return Ok((code, name_to_u128(&name)));
  }
}

pub fn read_hex(code: &str) -> ParseResult<Vec<u8>> {
  let mut data : Vec<u8> = Vec::new();
  let mut code = skip(code);
  while nth(code,0).is_ascii_hexdigit() && nth(code,1).is_ascii_hexdigit() {
    data.append(&mut hex::decode(&String::from_iter([nth(code,0),nth(code,1)])).unwrap());
    code = drop(code, 2);
    code = skip(code);
  }
  return Ok((code, data));
}

/// Converts a name to a number, using the following table:
/// ```
/// '.'       =>  0
/// '0' - '9' =>  1 to 10
/// 'A' - 'Z' => 11 to 36
/// 'a' - 'z' => 37 to 62
/// '_'       => 63
/// ```
/// Panics when name length > 20.
pub fn name_to_u128(name: &str) -> u128 {
  let mut num: u128 = 0;
  for (i, chr) in name.chars().enumerate() {
    debug_assert!(i < 20, "Name too big: `{}`.", name);
    num = (num << 6) + char_to_u128(chr);
  }
  return num;
}

pub const fn char_to_u128(chr: char) -> u128 {
  match chr {
    '.'       =>  0,
    '0'..='9' =>  1 + chr as u128 - '0' as u128,
    'A'..='Z' => 11 + chr as u128 - 'A' as u128,
    'a'..='z' => 37 + chr as u128 - 'a' as u128,
    '_'       => 63,
    _         => panic!("Invalid name character."),
  }
}

/// Inverse of `name_to_u128`
pub fn u128_to_name(num: u128) -> String {
  let mut name = String::new();
  let mut num = num;
  while num > 0 {
    let chr = (num % 64) as u8;
    let chr =
        match chr {
            0         => '.',
            1  ..= 10 => (chr -  1 + b'0') as char,
            11 ..= 36 => (chr - 11 + b'A') as char,
            37 ..= 62 => (chr - 37 + b'a') as char,
            63        => '_',
            64 ..     => panic!("impossible character value")
        };
    name.push(chr);
    num = num / 64;
  }
  name.chars().rev().collect()
}

pub fn read_until<A>(code: &str, stop: char, read: fn(&str) -> ParseResult<A>) -> ParseResult<Vec<A>> {
  let mut elems = Vec::new();
  let mut code = code;
  while code.len() > 0 && head(skip(code)) != stop {
    let (new_code, elem) = read(code)?;
    code = new_code;
    elems.push(elem);
  }
  code = tail(skip(code));
  return Ok((code, elems));
}

pub fn read_term(code: &str) -> ParseResult<Term> {
  let code = skip(code);
  match head(code) {
    '@' => {
      let code         = tail(code);
      let (code, name) = read_name(code)?;
      let (code, body) = read_term(code)?;
      return Ok((code, Term::Lam { name, body: Box::new(body) }));
    },
    '(' => {
      let code = skip(tail(code));
      let (code, oper) = read_oper(code);
      if let Some(oper) = oper {
        let (code, val0) = read_term(code)?;
        let (code, val1) = read_term(code)?;
        let (code, unit) = read_char(code, ')')?;
        return Ok((code, Term::Op2 { oper: oper, val0: Box::new(val0), val1: Box::new(val1) }));
      } else if head(code) == '!' {
        let code = tail(code);
        let (code, func) = read_term(code)?;
        let (code, argm) = read_term(code)?;
        let (code, unit) = read_char(code, ')')?;
        return Ok((code, Term::App { func: Box::new(func), argm: Box::new(argm) }));
      } else if ('A'..='Z').contains(&head(code)) {
        let (code, name) = read_name(code)?;
        let (code, args) = read_until(code, ')', read_term)?;
        // TODO: check function name size _on direct calling_, and propagate error
        return Ok((code, Term::Fun { name, args }));
      } else {
        let (code, func) = read_term(code)?;
        let (code, argm) = read_term(code)?;
        let (code, unit) = read_char(code, ')')?;
        return Ok((code, Term::App { func: Box::new(func), argm: Box::new(argm) }));
      }
    },
    '{' => {
      let code = tail(code);
      let (code, name) = read_name(code)?;
      let (code, args) = read_until(code, '}', read_term)?;
      return Ok((code, Term::Ctr { name, args }));
    },
    '[' => {
      let code = tail(code);
      let (code, vals) = read_until(code, ']', read_term)?;
      if vals.len() <= 12 { 
        return Ok((code, Term::Ctr {
          name: name_to_u128(&format!("T{}", vals.len())),
          args: vals
        }));
      } else {
        return Err(ParseErr { code: code.to_string(), erro: "Tuple too long".to_string() });
      }
    },
    '#' => {
      let code = tail(code);
      let (code, numb) = read_numb(code)?;
      return Ok((code, Term::Num { numb }));
    },
    '\'' => {
      let code = tail(code);
      let (code, numb) = read_name(code)?;
      let (code, unit) = read_char(code, '\'')?;
      return Ok((code, Term::Num { numb }));
    },
    _ => {
      if let ('d','u','p',' ') = (nth(code,0), nth(code,1), nth(code,2), nth(code,3)) {
        let code = drop(code,3);
        let (code, nam0) = read_name(code)?;
        let (code, nam1) = read_name(code)?;
        let (code, unit) = read_char(code, '=')?;
        let (code, expr) = read_term(code)?;
        let (code, unit) = read_char(code, ';')?;
        let (code, body) = read_term(code)?;
        return Ok((code, Term::Dup { nam0, nam1, expr: Box::new(expr), body: Box::new(body) }));
      // let x = y; z
      // ------------
      // (@x z y)
      } else if let ('l','e','t',' ') = (nth(code,0), nth(code,1), nth(code,2), nth(code,3)) {
        let code = drop(code,3);
        let (code, name) = read_name(code)?;
        let (code, unit) = read_char(code, '=')?;
        let (code, expr) = read_term(code)?;
        let (code, unit) = read_char(code, ';')?;
        let (code, body) = read_term(code)?;
        return Ok((code, Term::App {
          func: Box::new(Term::Lam { name, body: Box::new(body) }),
          argm: Box::new(expr),
        }));
      // ask x = y; z
      // ------------
      // (y @x z)
      } else if let ('a','s','k',' ') = (nth(code,0), nth(code,1), nth(code,2), nth(code,3)) {
        let code = skip(drop(code,3));
        if nth(code,0) == '(' {
          let (code, expr) = read_term(code)?;
          let (code, unit) = read_char(code, ';')?;
          let (code, body) = read_term(code)?;
          return Ok((code, Term::App {
            func: Box::new(expr),
            argm: Box::new(Term::Lam { name: VAR_NONE, body: Box::new(body) }),
          }));
        } else {
          let (code, name) = read_name(code)?;
          let (code, unit) = read_char(code, '=')?;
          let (code, expr) = read_term(code)?;
          let (code, unit) = read_char(code, ';')?;
          let (code, body) = read_term(code)?;
          return Ok((code, Term::App {
            func: Box::new(expr),
            argm: Box::new(Term::Lam { name, body: Box::new(body) }),
          }));
        }
      } else {
        let (code, name) = read_name(code)?;
        return Ok((code, Term::Var { name }));
      }
    }
  }
}

pub fn read_oper(in_code: &str) -> (&str, Option<u128>) {
  let code = skip(in_code);
  match head(code) {
    // Should not match with `~`
    '+' => (tail(code), Some(ADD)),
    '-' => (tail(code), Some(SUB)),
    '*' => (tail(code), Some(MUL)),
    '/' => (tail(code), Some(DIV)),
    '%' => (tail(code), Some(MOD)),
    '&' => (tail(code), Some(AND)),
    '|' => (tail(code), Some(OR)),
    '^' => (tail(code), Some(XOR)),
    '<' => match head(tail(code)) {
      '=' => (tail(tail(code)), Some(LTE)),
      '<' => (tail(tail(code)), Some(SHL)),
      _   => (tail(code), Some(LTN)),
    },
    '>' => match head(tail(code)) {
      '=' => (tail(tail(code)), Some(GTE)),
      '>' => (tail(tail(code)), Some(SHR)),
      _   => (tail(code), Some(GTN)),
    },
    '=' => match head(tail(code)) {
      '=' => (tail(tail(code)), Some(EQL)),
      _   => (code, None),
    },
    '!' => match head(tail(code)) {
      '=' => (tail(tail(code)), Some(NEQ)),
      _   => (code, None),
    },
    _ => (code, None),
  }
}

pub fn read_rule(code: &str) -> ParseResult<Rule> {
  let (code, lhs) = read_term(code)?;
  let (code, ())  = read_char(code, '=')?;
  let (code, rhs) = read_term(code)?;
  return Ok((code, Rule{lhs, rhs}));
}

pub fn read_rules(code: &str) -> ParseResult<Vec<Rule>> {
  let (code, rules) = read_until(code, '\0', read_rule)?;
  return Ok((code, rules));
}

pub fn read_func(code: &str) -> ParseResult<CompFunc> {
  let (code, rules) = read_until(code, '\0', read_rule)?;
  let func = Func { rules };
  if let Some(func) = compile_func(&func, false) {
    return Ok((code, func));
  } else {
    return Err(ParseErr { 
      code: code.to_string(), 
      erro: "Couldn't parse function".to_string()
    });
  }
}

pub fn read_sign(code: &str) -> ParseResult<Option<crypto::Signature>> {
  let code = skip(code);
  if let ('s','i','g','n') = (nth(code,0), nth(code,1), nth(code,2), nth(code,3)) {
    let code = drop(code,4);
    let (code, unit) = read_char(code, '{')?;
    let (code, sign) = read_hex(code)?;
    let (code, unit) = read_char(code, '}')?;
    if sign.len() == 65 {
      return Ok((code, Some(crypto::Signature(sign.as_slice().try_into().unwrap())))); // TODO: remove unwrap
    } else {
      return Err(ParseErr { 
        code: code.to_string(), 
        erro: "Wrong signature size".to_string()
      });
    }
  }
  return Ok((code, None));
}

pub fn read_statement(code: &str) -> ParseResult<Statement> {
  let code = skip(code);
  match (nth(code,0), nth(code,1), nth(code,2)) {
    ('f','u','n') => {
      let code = drop(code,3);
      let (code, unit) = read_char(code, '(')?;
      let (code, name) = read_name(code)?;
      let (code, args) = read_until(code, ')', read_name)?;
      let (code, unit) = read_char(code, '{')?;
      let (code, ruls) = read_until(code, '}', read_rule)?;
      let code = skip(code);
      let (code, init) = if let ('w','i','t','h') = (nth(code,0), nth(code,1), nth(code,2), nth(code,3)) {
        let code = drop(code,4);
        let (code, unit) = read_char(code, '{')?;
        let (code, init) = read_term(code)?;
        let (code, unit) = read_char(code, '}')?;
        (code, init)
      } else {
        (code, Term::Num { numb: 0 })
      };
      let (code, sign) = read_sign(code)?;
      let func = Func { rules: ruls };
      return Ok((code, Statement::Fun { name, args, func, init, sign }));
    }
    ('c','t','r') => {
      let code = drop(code,3);
      let (code, unit) = read_char(code, '{')?;
      let (code, name) = read_name(code)?;
      let (code, args) = read_until(code, '}', read_name)?;
      let (code, sign) = read_sign(code)?;
      return Ok((code, Statement::Ctr { name, args, sign }));
    }
    ('r','u','n') => {
      let code = drop(code,3);
      let (code, unit) = read_char(code, '{')?;
      let (code, expr) = read_term(code)?;
      let (code, unit) = read_char(code, '}')?;
      let (code, sign) = read_sign(code)?;
      return Ok((code, Statement::Run { expr, sign  }));
    }
    // reg Foo.Bar { #x123456 } sign { signature }
    ('r','e','g') => {
      let code = skip(drop(code, 3));
      let (code, name) = if nth(code,0) == '{' { (code, 0) } else { read_name(code)? };
      let (code, unit) = read_char(code, '{')?;
      let (code, unit) = read_char(code, '#')?;
      let (code, ownr) = read_numb(code)?;
      let (code, unit) = read_char(code, '}')?;
      let (code, sign) = read_sign(code)?;
      return Ok((code, Statement::Reg { name, ownr, sign }));
    }
    _ => {
      return Err(ParseErr { code: code.to_string(),  erro: "Expected statement.".to_string() });
    }
  }
}

pub fn read_statements(code: &str) -> ParseResult<Vec<Statement>> {
  read_until(code, '\0', read_statement)
}

// View
// ----

pub fn view_name(name: u128) -> String {
  if name == VAR_NONE {
    return "~".to_string();
  } else {
    return u128_to_name(name);
  }
}

pub fn view_term(term: &Term) -> String {
  enum StackItem<'a> {
    Term(&'a Term),
    Str(String),
  }

  let mut stack = vec![StackItem::Term(term)];
  let mut output = Vec::new();

  while !stack.is_empty() {
    let item = stack.pop().unwrap();

    match item {
      StackItem::Str(str) => {
        output.push(str);
      }
      StackItem::Term(term) => {  
        match term {
          Term::Var { name } => {
            output.push(view_name(*name));
          }
          Term::Dup { nam0, nam1, expr, body } => {
            output.push("dup ".to_string());
            output.push(view_name(*nam0));
            output.push(" ".to_string());
            output.push(view_name(*nam1));
            output.push(" = ".to_string());
            stack.push(StackItem::Term(&*body));
            stack.push(StackItem::Str("; ".to_string()));
            stack.push(StackItem::Term(&*expr));
          }
          Term::Lam { name, body } => {
            output.push(format!("@{} ", view_name(*name)));
            stack.push(StackItem::Term(&*body));
          }
          Term::App { func, argm } => {
            output.push("(".to_string());
            stack.push(StackItem::Str(")".to_string()));
            stack.push(StackItem::Term(&*argm));
            stack.push(StackItem::Str(" ".to_string()));
            stack.push(StackItem::Term(&*func));
          }
          Term::Ctr { name, args } => {
            let name = view_name(*name);
            // Pretty print names
            if name == "Name" && args.len() == 1 {
              if let Term::Num { numb } = args[0] {
                output.push(format!("{{Name '{}'}}", view_name(numb)));
              }
            } else {
              output.push("{".to_string());
              output.push(name);
              stack.push(StackItem::Str("}".to_string()));
              for arg in args.iter().rev() {
                stack.push(StackItem::Term(arg));
                stack.push(StackItem::Str(" ".to_string()));
              }
            }
          }
          Term::Fun { name, args } => {
            let name = view_name(*name);
            output.push("(".to_string());
            output.push(name);
            stack.push(StackItem::Str(")".to_string()));
            for arg in args.iter().rev() {
              stack.push(StackItem::Term(arg));
              stack.push(StackItem::Str(" ".to_string()));
            }
          }
          Term::Num { numb } => {
            // If it has 26-30 bits, pretty-print as a name
            //if *numb > 0x3FFFFFF && *numb <= 0x3FFFFFFF {
              //return format!("@{}", view_name(*numb));
            //} else {
              output.push(format!("#{}", numb));
            //}
          }
          Term::Op2 { oper, val0, val1 } => {
            let oper = view_oper(oper);
            output.push(format!("({} ", oper));
            stack.push(StackItem::Str(")".to_string()));
            stack.push(StackItem::Term(val1));
            stack.push(StackItem::Str(" ".to_string()));
            stack.push(StackItem::Term(val0));
          }
        }
      }
    }
  }
  let res = output.join("");
  res
}

pub fn view_oper(oper: &u128) -> String {
  match *oper {
    ADD => "+",
    SUB => "-",
    MUL => "*",
    DIV => "/",
    MOD => "%",
    AND => "&",
    OR  => "|",
    XOR => "^",
    SHL => "<<",
    SHR => ">>",
    LTN => "<",
    LTE => "<=",
    EQL => "==",
    GTE => ">=",
    GTN => ">",
    NEQ => "!=",
    _   => "??",
  }.to_string()
}

pub fn view_statement(statement: &Statement) -> String {
  fn view_sign(sign: &Option<crypto::Signature>) -> String {
    fn format_sign(sign: &crypto::Signature) -> String {
      let hex = sign.to_hex();
      let mut text = String::new();
      for i in 0 .. 5 {
        text.push_str("  ");
        text.push_str(&hex[i * 26 .. (i+1) * 26]);
        text.push_str("\n");
      }
      return text;
    }
    match sign {
      None       => String::new(),
      Some(sign) => format!(" sign {{\n{}}}", format_sign(sign)),
    }
  }
  match statement {
    Statement::Fun { name, args, func, init, sign } => {
      let name = u128_to_name(*name);
      let func = func.rules.iter().map(|x| format!("\n  {} = {}", view_term(&x.lhs), view_term(&x.rhs)));
      let func = func.collect::<Vec<String>>().join("");
      let args = args.iter().map(|x| u128_to_name(*x)).collect::<Vec<String>>().join(" ");
      let init = view_term(init);
      let init = format!(" with {{\n  {}\n}}", init);
      let sign = view_sign(sign);
      return format!("fun ({} {}) {{{}\n}}{}{}", name, args, func, init, sign);
    }
    Statement::Ctr { name, args, sign } => {
      // correct:
      let name = u128_to_name(*name);
      let args = args.iter().map(|x| format!(" {}", u128_to_name(*x))).collect::<Vec<String>>().join("");
      let sign = view_sign(sign);
      return format!("ctr {{{}{}}}{}", name, args, sign);
    }
    Statement::Run { expr, sign } => {
      let expr = view_term(expr);
      let sign = view_sign(sign);
      return format!("run {{\n  {}\n}}{}", expr, sign);
    }
    Statement::Reg { name, ownr, sign } => {
      let name = u128_to_name(*name);
      let ownr = format!("#x{:0>30x}", ownr);
      let sign = view_sign(sign);
      return format!("reg {} {{ {} }}{}", name, ownr, sign);
    }
  }
}

pub fn view_statements(statements: &[Statement]) -> String {
  let mut result = String::new();
  for statement in statements {
    result.push_str(&view_statement(statement));
    result.push_str("\n");
  }
  return result;
}

// Hashing
// -------

pub fn hash_term(term: &Term) -> crypto::Hash {
  crypto::keccak256(&util::bitvec_to_bytes(&bits::serialized_term(&term)))
}

pub fn hash_statement(statement: &Statement) -> crypto::Hash {
  crypto::keccak256(&util::bitvec_to_bytes(&bits::serialized_statement(&remove_sign(&statement))))
}

// Tests
// -----

// FIXME: since we don't have a proper macro, we're using this temporarily
pub fn print_io_consts() {
  let names = ["done", "take", "save", "call", "subj", "from", "load"];
  for name in names {
    let name = name.to_uppercase();
    let numb = name_to_u128(&format!("IO_{}", name));
    println!("const IO_{} : u128 = 0x{:x}; // name_to_u128(\"IO_{}\")", name, numb, name);
  }
  for name in names {
    let numb = name_to_u128(&name);
    println!("const MC_{} : u128 = 0x{:x}; // name_to_u128(\"{}\")", name.to_uppercase(), numb, name);
  }
}

// Serializes, deserializes and evaluates statements
pub fn test_statements(statements: &[Statement]) {
  let str_0 = view_statements(statements);
  let str_1 = view_statements(&crate::bits::deserialized_statements(&crate::bits::serialized_statements(&statements)).unwrap());

  println!("Block {}", if str_0 == str_1 { "" } else { "(note: serialization error, please report)" });
  println!("=====");
  println!();

  let mut rt = init_runtime(None);
  let init = Instant::now();
  rt.run_statements(&statements, false);
  println!();

  println!("Stats");
  println!("=====");
  println!();

  println!("[size] {}", rt.get_size());
  println!("[mana] {}", rt.get_mana());
  println!("[rwts] {}", rt.get_rwts());
  println!("[time] {} ms", init.elapsed().as_millis());
}

pub fn test_statements_from_code(code: &str) {
  let statments = read_statements(code);
  match statments {
    Ok((.., statements)) => test_statements(&statements),
    Err(ParseErr { code, erro }) => println!("{}", erro),
  }
}

pub fn test_statements_from_file(file: &str) {
  test_statements_from_code(&std::fs::read_to_string(file).expect("file not found"));
}
