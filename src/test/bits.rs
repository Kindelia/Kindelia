use std::collections::HashMap;

use crate::{
  bits::{
    deserialize_fixlen, deserialize_list, deserialize_varlen, deserialized_message,
    deserialized_statements, serialize_fixlen, serialize_list, serialize_varlen,
    serialized_message, serialized_statements,
  },
  hvm::{view_statements, Term},
  node::Message,
  test::strategies::{message, statement, u256 as u256_strategy},
  util::u256,
};
use bit_vec::BitVec;
use proptest::{collection::vec, proptest};

proptest! {
  #[test]
  fn serialize_deserialize_statements(statements in vec(statement(), 0..20)) {
    let s1 = view_statements(&statements);
    let bits = serialized_statements(&statements);
    let statements2 = deserialized_statements(&bits).unwrap();
    let s2 = view_statements(&statements2);
    assert_eq!(s1, s2);
  }

  #[test]
  fn serialize_deserialize_message(message in message()) {
    let bits = serialized_message(&message);
    let message2 = deserialized_message(&bits).unwrap();
    assert_eq!(format!("{:?}", message), format!("{:?}", message2));
  }
}

#[test]
pub fn test_serializer_0() {
  let mut bits = BitVec::new();
  let a = u256(123);
  let b = u256(777);
  let mut names = HashMap::new();
  serialize_fixlen(10, &a, &mut bits, &mut names);
  serialize_fixlen(16, &b, &mut bits, &mut names);
  let mut index = 0;
  let x0 = deserialize_fixlen(10, &bits, &mut index, &mut names).unwrap();
  let x1 = deserialize_fixlen(16, &bits, &mut index, &mut names).unwrap();
  assert_eq!(a, x0);
  assert_eq!(b, x1);
}

#[test]
pub fn test_serializer_1() {
  let mut bits = BitVec::new();
  let a = u256(123);
  let b = u256(777);
  let mut names = HashMap::new();
  serialize_varlen(&a, &mut bits, &mut names);
  serialize_varlen(&b, &mut bits, &mut names);
  let mut index = 0;
  let x0 = deserialize_varlen(&bits, &mut index, &mut names).unwrap();
  let x1 = deserialize_varlen(&bits, &mut index, &mut names).unwrap();
  assert_eq!(a, x0);
  assert_eq!(b, x1);
}

#[test]
pub fn test_serializer_2() {
  let mut bits = BitVec::new();
  let a = u256(123);
  let b = u256(777);
  let c = u256(1000);
  let mut g_names = HashMap::new();
  let vals = vec![a, b, c];
  serialize_list(
    |x, bits, names| serialize_fixlen(10, x, bits, names),
    &vals,
    &mut bits,
    &mut g_names,
  );
  let mut index = 0;
  let gots = deserialize_list(
    |bits, ix, names| deserialize_fixlen(10, bits, ix, names),
    &bits,
    &mut index,
    &mut g_names,
  )
  .unwrap();
  assert_eq!(vals, gots);
}
