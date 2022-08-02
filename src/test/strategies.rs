use std::{collections::HashMap, fmt::Debug, sync::Arc};

use crate::{
  crypto,
  hvm::{
    init_map, name_to_u128, Arits, CompFunc, CompRule, Func, Funcs, Heap, Map, Nodes, Ownrs,
    Rollback, Rule, Runtime, SerializedHeap, Statement, Store, Term, Var,
  },
  node::{hash_bytes, Address, Block, Body, Message, Peer, Transaction},
};
use primitive_types::U256;
use proptest::{
  arbitrary::any,
  array,
  collection::{hash_map, vec},
  option, prop_oneof,
  strategy::{Just, Strategy},
};

// generate valid names
pub fn name() -> impl Strategy<Value = u128> {
  // TODO: temporary fix to new limitation due
  // to the fact that is not possible make a name
  // that start with a number anymore
  "[a-z][a-zA-Z0-9_]{1,19}".prop_map(|s| name_to_u128(&s))
}

pub fn fun_name() -> impl Strategy<Value = u128> {
  "[A-Z][a-zA-Z0-9_]{1,19}".prop_map(|s| name_to_u128(&s))
}

// generate valid terms
pub fn term() -> impl Strategy<Value = Term> {
  let leaf = prop_oneof![
    name().prop_map(|n| Term::Var { name: n }),
    name().prop_map(|n| Term::Num { numb: n }),
  ];

  leaf.prop_recursive(
    16,  // 16 levels deep
    256, // Shoot for maximum size of 256 nodes
    10,  // We put up to 10 items per collection
    |inner| {
      prop_oneof![
        (name(), name(), inner.clone(), inner.clone()).prop_map(|(n0, n1, e, b)| {
          Term::Dup { nam0: n0, nam1: n1, expr: Box::new(e), body: Box::new(b) }
        }),
        (name(), inner.clone()).prop_map(|(n, e)| { Term::Lam { name: n, body: Box::new(e) } }),
        (inner.clone(), inner.clone())
          .prop_map(|(f, a)| { Term::App { func: Box::new(f), argm: Box::new(a) } }),
        (fun_name(), vec(inner.clone(), 0..10))
          .prop_map(|(n, v)| { Term::Ctr { name: n, args: v } }),
        (fun_name(), vec(inner.clone(), 0..10))
          .prop_map(|(n, v)| { Term::Fun { name: n, args: v } }),
        (0..15_u128, inner.clone(), inner).prop_map(|(o, v0, v1)| {
          Term::Op2 { oper: o, val0: Box::new(v0), val1: Box::new(v1) }
        }),
      ]
    },
  )
}

fn fun() -> impl Strategy<Value = Term> {
  (fun_name(), vec(term(), 0..32)).prop_map(|(n, b)| Term::Fun { name: n, args: b })
}

// generate rules
pub fn rule() -> impl Strategy<Value = Rule> {
  (fun(), term()).prop_map(|(lhs, rhs)| Rule { lhs, rhs })
}

pub fn func() -> impl Strategy<Value = Func> {
  vec(rule(), 0..10).prop_map(|rules| Func { rules })
}

// generate signatures
pub fn sign() -> impl Strategy<Value = crypto::Signature> {
  (vec(any::<u8>(), 65)).prop_map(|s| crypto::Signature(s.try_into().unwrap()))
}

// generate statements
pub fn statement() -> impl Strategy<Value = Statement> {
  prop_oneof![
    (fun_name(), vec(name(), 0..10), func(), term(), option::of(sign())).prop_map(
      |(name, args, func, init, sign)| { Statement::Fun { name, args, func, init, sign } }
    ),
    (fun_name(), vec(name(), 0..10), option::of(sign()))
      .prop_map(|(name, args, sign)| { Statement::Ctr { name, args, sign } }),
    (term(), option::of(sign())).prop_map(|(t, s)| { Statement::Run { expr: t, sign: s } }),
    (name(), name(), option::of(sign()))
      .prop_map(|(name, ownr, sign)| { Statement::Reg { name, ownr, sign } }),
  ]
}

pub fn nodes() -> impl Strategy<Value = Nodes> {
  (map(any::<u128>())).prop_map(|m| Nodes { nodes: m })
}

pub fn map<A: std::fmt::Debug>(s: impl Strategy<Value = A>) -> impl Strategy<Value = Map<A>> {
  vec((any::<u128>(), s), 0..10).prop_map(|v| {
    let mut m = init_map();
    for (k, v) in v {
      m.insert(k, v);
    }
    m
  })
}

pub fn store() -> impl Strategy<Value = Store> {
  map(any::<u128>()).prop_map(|m| Store { links: m })
}

pub fn arits() -> impl Strategy<Value = Arits> {
  map(any::<u128>()).prop_map(|m| Arits { arits: m })
}

pub fn ownrs() -> impl Strategy<Value = Ownrs> {
  map(any::<u128>()).prop_map(|m| Ownrs { ownrs: m })
}

pub fn var() -> impl Strategy<Value = Var> {
  (name(), any::<u128>(), option::of(any::<u128>()), any::<bool>()).prop_map(|(n, p, f, e)| Var {
    name: n,
    param: p,
    field: f,
    erase: e,
  })
}

pub fn comp_rule() -> impl Strategy<Value = CompRule> {
  (vec(any::<u128>(), 0..32), vec(var(), 0..32), vec((any::<u128>(), any::<u128>()), 0..32), term())
    .prop_map(|(c, v, e, b)| CompRule { cond: c, vars: v, eras: e, body: b })
}

pub fn comp_func() -> impl Strategy<Value = CompFunc> {
  (func(), any::<u128>(), vec(any::<u128>(), 0..32), vec(comp_rule(), 0..32))
    .prop_map(|(f, a, r, s)| CompFunc { func: f, arity: a, redux: r, rules: s })
}

pub fn funcs() -> impl Strategy<Value = Funcs> {
  map(comp_func().prop_map(|cf| Arc::new(cf))).prop_map(|m| Funcs { funcs: m })
}

pub fn heap() -> impl Strategy<Value = Heap> {
  let tuple_strategy =
    (any::<u128>(), any::<u128>(), any::<u128>(), any::<u128>(), any::<u128>(), any::<u128>());

  (tuple_strategy, tuple_strategy, any::<i128>(), nodes(), store(), arits(), ownrs(), funcs())
    .prop_map(
      |(
        (uuid, mcap, tick, funs, dups, rwts),
        (mana, next, meta, hax1, hax0, time),
        size,
        memo,
        disk,
        arit,
        ownr,
        file,
      )| Heap {
        mcap,
        disk,
        arit,
        ownr,
        file: Funcs { funcs: init_map() }, // TODO, fix?
        uuid,
        memo,
        tick,
        funs,
        dups,
        rwts,
        mana,
        size,
        next,
        meta,
        hax0,
        hax1,
        time,
      },
    )
}

pub fn u256() -> impl Strategy<Value = U256> {
  array::uniform32(any::<u8>()).prop_map(|a| U256::from(a))
}

pub fn body() -> impl Strategy<Value = Body> {
  vec(any::<u8>(), 0..128).prop_map(|v| Body { data: v })
}

pub fn block() -> impl Strategy<Value = Block> {
  (any::<u128>(), any::<u128>(), u256(), body())
    .prop_map(|(t, m, p, b)| crate::node::new_block(p, m, t, b))
}

pub fn address() -> impl Strategy<Value = Address> {
  (any::<u8>(), any::<u8>(), any::<u8>(), any::<u8>(), any::<u16>())
    .prop_map(|(a, b, c, d, e)| Address::IPv4 { val0: a, val1: b, val2: c, val3: d, port: e })
}

pub fn peer() -> impl Strategy<Value = Peer> {
  (any::<u32>(), address()).prop_map(|(s, a)| Peer { seen_at: s as u128, address: a })
}

pub fn transaction() -> impl Strategy<Value = Transaction> {
  vec(any::<u8>(), 1..128).prop_map(|d| Transaction::new(d))
}

pub fn message() -> impl Strategy<Value = Message> {
  prop_oneof![
    (any::<bool>(), vec(block(), 0..10), vec(peer(), 0..10))
      .prop_map(|(g, b, p)| Message::NoticeTheseBlocks { gossip: g, blocks: b, peers: p }),
    (u256()).prop_map(|h| Message::GiveMeThatBlock { bhash: h }),
    (transaction()).prop_map(|t| Message::PleaseMineThisTransaction { trans: t })
  ]
}
