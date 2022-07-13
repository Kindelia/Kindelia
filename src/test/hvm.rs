use crate::{
  bits::{deserialized_func, serialized_func},
  hvm::{
    init_map, init_runtime, name_to_u128, read_statements, u128_to_name, view_statements, Rollback,
  },
  test::{
    strategies::{func, heap, name, statement},
    util::{
      advance, rollback, rollback_path, rollback_simple, temp_dir, test_heap_checksum,
      view_rollback_ticks, RuntimeStateTest, TempDir,
    },
  },
};
use proptest::collection::vec;
use proptest::proptest;
use rstest::rstest;

#[rstest]
#[case(&["Count", "Store", "Sub", "Add"], PRE_COUNTER, COUNTER)]
#[case(&["Bank", "Random", "AddAcc", "AddEq", "AddChild"], PRE_BANK, BANK)]
pub fn simple_rollback(
  #[case] fn_names: &[&str],
  #[case] pre_code: &str,
  #[case] code: &str,
  temp_dir: TempDir,
) {
  assert!(rollback_simple(pre_code, code, fn_names, 1000, 1, &temp_dir.path));
}

#[rstest]
#[case(&["Count", "Store", "Sub", "Add"], PRE_COUNTER, COUNTER)]
#[case(&["Bank", "Random", "AddAcc", "AddEq", "AddChild"], PRE_BANK, BANK)]
pub fn advanced_rollback_in_random_state(
  #[case] fn_names: &[&str],
  #[case] pre_code: &str,
  #[case] code: &str,
  temp_dir: TempDir,
) {
  let path = [1000, 12, 1000, 24, 1000, 36];
  assert!(rollback_path(pre_code, code, fn_names, &path, &temp_dir.path));
}

#[rstest]
#[case(&["Count", "Store", "Sub", "Add"], PRE_COUNTER, COUNTER)]
#[case(&["Bank", "Random", "AddAcc", "AddEq", "AddChild"], PRE_BANK, BANK)]
pub fn advanced_rollback_in_saved_state(
  #[case] fn_names: &[&str],
  #[case] pre_code: &str,
  #[case] code: &str,
  temp_dir: TempDir,
) {
  let mut rt = init_runtime(Some(&temp_dir.path));
  rt.run_statements_from_code(pre_code, true);
  advance(&mut rt, 1000, Some(code));
  rt.rollback(900);
  println!(" - tick: {}", rt.get_tick());
  let s1 = RuntimeStateTest::new(&fn_names, &mut rt);

  advance(&mut rt, 1000, Some(code));
  rt.rollback(900);
  println!(" - tick: {}", rt.get_tick());
  let s2 = RuntimeStateTest::new(&fn_names, &mut rt);

  advance(&mut rt, 1000, Some(code));
  rt.rollback(900);
  println!(" - tick: {}", rt.get_tick());
  let s3 = RuntimeStateTest::new(&fn_names, &mut rt);

  assert_eq!(s1, s2);
  assert_eq!(s2, s3);
}

#[rstest]
#[case(&["Count", "Store", "Sub", "Add"], PRE_COUNTER, COUNTER)]
#[case(&["Bank", "Random", "AddAcc", "AddEq", "AddChild"], PRE_BANK, BANK)]
pub fn advanced_rollback_run_fail(
  #[case] fn_names: &[&str],
  #[case] pre_code: &str,
  #[case] code: &str,
  temp_dir: TempDir,
) {
  let path = [2, 1, 2, 1, 2, 1];
  assert!(rollback_path(PRE_COUNTER, COUNTER, &fn_names, &path, &temp_dir.path));
}

#[rstest]
#[case(PRE_COUNTER, COUNTER)]
#[case(PRE_BANK, BANK)]
pub fn stack_overflow(#[case] pre_code: &str, #[case] code: &str, temp_dir: TempDir) {
  // caused by compute_at function
  let mut rt = init_runtime(Some(&temp_dir.path));
  rt.run_statements_from_code(pre_code, true);
  advance(&mut rt, 1000, Some(code));
}

#[rstest]
#[ignore = "fix not done"]
// TODO: fix drop stack overflow
pub fn stack_overflow2(temp_dir: TempDir) {
  // caused by drop of term
  let mut rt = init_runtime(Some(&temp_dir.path));
  rt.run_statements_from_code(PRE_COUNTER, false);
  rt.run_statements_from_code(COUNTER_STACKOVERFLOW, false);
}

#[rstest]
#[case(&["Count", "Store", "Sub", "Add"], PRE_COUNTER, COUNTER)]
#[case(&["Bank", "Random", "AddAcc", "AddEq", "AddChild"], PRE_BANK, BANK)]
pub fn persistence1(
  #[case] fn_names: &[&str],
  #[case] pre_code: &str,
  #[case] code: &str,
  #[values(1000, 1500, 2000)] tick: u128,
  temp_dir: TempDir,
) {
  let mut rt = init_runtime(Some(&temp_dir.path));
  rt.run_statements_from_code(pre_code, true);

  advance(&mut rt, tick, Some(code));
  let s1 = RuntimeStateTest::new(&fn_names, &mut rt);

  let last = {
    if let Rollback::Cons { head, .. } = *rt.get_back() {
      rt.get_heap(head).tick
    } else {
      0
    }
  };

  rt.rollback(last); // rollback for the latest rollback saved
  let s2 = RuntimeStateTest::new(&fn_names, &mut rt);

  advance(&mut rt, tick, Some(code));
  let s3 = RuntimeStateTest::new(&fn_names, &mut rt);

  rt.restore_state().expect("Could not restore state"); // restore last rollback, must be equal to s2
  let s4 = RuntimeStateTest::new(&fn_names, &mut rt);

  advance(&mut rt, tick, Some(code));
  let s5 = RuntimeStateTest::new(&fn_names, &mut rt);

  assert_eq!(s1, s3);
  assert_eq!(s2, s4);
  assert_eq!(s3, s5);
}

#[rstest]
fn one_hundred_snapshots(temp_dir: TempDir) {
  // run this with rollback in each 4th snapshot
  // note: this test has no state
  let mut rt = init_runtime(Some(&temp_dir.path));
  for i in 0..100000 {
    rt.tick();
    println!(" - tick: {}, - rollback: {}", rt.get_tick(), view_rollback_ticks(&rt));
  }
}

proptest! {
  #[test]
  fn name_conversion(name in name()) {
    let a = u128_to_name(name);
    let b = name_to_u128(&a);
    let c = u128_to_name(b);
    assert_eq!(name, b);
    assert_eq!(a, c);
  }

  #[test]
  fn parser(statements in vec(statement(), 0..10)) {
    let str = view_statements(&statements);
    let (.., s1) = read_statements(&str).unwrap();
    assert_eq!(statements, s1);
  }

  #[test]
  #[ignore = "slow"]
  fn serialize_deserialize_heap(heap in heap()) {
    let mut h1 = heap;
    let s1 = format!("{:?}", h1);
    let a = h1.serialize();
    h1.deserialize(&a);
    let s2 = format!("{:?}", h1);
    assert_eq!(s1, s2);
  }
}

// ===========================================================
// Codes
pub const PRE_COUNTER: &'static str = "
  ctr {Succ p}
  ctr {Zero}

  fun (ToSucc n) {
    (ToSucc #0) = {Zero}
    (ToSucc n) = {Succ (ToSucc (- n #1))}
  }

  fun (Add n) {
    (Add n) = {Succ n}
  }

  fun (Sub n) {
    (Sub {Succ p}) = p
    (Sub {Zero}) = {Zero}
  }

  ctr {StoreAdd}
  ctr {StoreSub}
  ctr {StoreGet}

  fun (Store action) {
    (Store {StoreAdd}) =
      ask l = (Take);
      ask (Save (Add l));
      (Done #0)
    (Store {StoreSub}) =
      ask l = (Take);
      ask (Save (Sub l));
      (Done #0)
    (Store {StoreGet}) = 
      ask l = (Load);
      (Done l)
  } with { {Zero} }
";

pub const COUNTER: &'static str = "
  run {
    ask (Call 'Store' [{StoreAdd}]);
    ask count = (Call 'Store' [{StoreGet}]);
    (Done count)
  }

  run {
    ask (Call 'Count' [{Inc}]);
    ask count = (Call 'Count' [{Get}]);
    (Done count)
  }
";

pub const SIMPLE_COUNT: &'static str = "
  run {
    ask (Call 'Count' [{Inc}]);
    ask count = (Call 'Count' [{Get}]);
    (Done count)
  }
";

pub const COUNTER_STACKOVERFLOW: &'static str = "
  run {
    (Done (ToSucc #8000))
  }
";

pub const PRE_BANK: &'static str = "
ctr {Node k v l r}
ctr {Leaf}

fun (AddEq cond key t) {
  (AddEq #1 ~ {Node k v l r}) = {Node k (+ v #1) l r}
  (AddEq #0 key {Node k v l r}) = 
    dup k.0 k.1 = k;
    dup key.0 key.1 = key;
    (AddChild (> key.0 k.0) key.1 {Node k.1 v l r})
} 

fun (AddChild cond key t) {
  (AddChild #1 key {Node k v l r}) = {Node k v l (AddAcc key r)}
  (AddChild #0 key {Node k v l r}) = {Node k v (AddAcc key l) r}
} 

fun (AddAcc key t) {
  (AddAcc key {Leaf}) = {Node key #1 {Leaf} {Leaf}}
  (AddAcc key {Node k v lft rgt}) =
    dup k.0 k.1 = k;
    dup key.0 key.1 = key;
    (AddEq (== k.0 key.0) key.1 {Node k.1 v lft rgt})
}

ctr {Random_Inc}
ctr {Random_Get}

fun (Random action) {
  (Random {Random_Inc}) = 
    !take x
    !save (% (+ (* #25214903917 x) #11) #281474976710656)
    !done #0
  (Random {Random_Get}) = 
    !load x
    !done x
} with {
  #1
}

ctr {Bank_Add acc}
ctr {Bank_Get}

fun (Bank action) {
  (Bank {Bank_Add acc}) = 
    !take t
    !save (AddAcc acc t)
    !done #0
  (Bank {Bank_Get}) = 
    !load x
    !done x
} with {
  {Leaf}
}
";

pub const BANK: &'static str = "
  run {
    !call ~   'Random' [{Random_Inc}]
    !call acc 'Random' [{Random_Get}]
    !call ~   'Bank' [{Bank_Add acc }]
    !call b   'Bank' [{Bank_Get}]
    !done b
    // !done (AddAcc #1 {Leaf})
  }
";
