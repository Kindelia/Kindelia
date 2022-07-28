use rstest::fixture;

use crate::hvm::{init_runtime, name_to_u128, show_term, Rollback, Runtime, U128_NONE};
use std::{
  collections::{hash_map::DefaultHasher, HashMap},
  hash::{Hash, Hasher},
  path::PathBuf,
  sync::Arc,
};

// ===========================================================
// Aux functions
pub fn are_all_elemenets_equal<E: PartialEq>(vec: &[E]) -> bool {
  if vec.len() == 0 {
    return true;
  }
  let last_value = &vec[0];
  for value in vec.iter().skip(1) {
    if *value != vec[0] {
      return false;
    }
  }
  true
}

pub fn view_rollback_ticks(rt: &Runtime) -> String {
  fn view_rollback_ticks_go(rt: &Runtime, back: &Arc<Rollback>) -> Vec<Option<u128>> {
    match &**back {
      Rollback::Nil => return Vec::new(),
      Rollback::Cons { keep, head, tail, life } => {
        let mut vec = view_rollback_ticks_go(&rt, tail);
        let tick = rt.get_heap(*head).tick;
        vec.push(Some(tick));
        return vec;
      }
    }
  }

  let back = rt.get_back();
  let ticks = view_rollback_ticks_go(rt, &back);
  let elems = ticks
    .iter()
    .rev()
    .map(|x| {
      if let Some(x) = x {
        format!("{}", if *x != U128_NONE { *x } else { 0 })
      } else {
        "___________".to_string()
      }
    })
    .collect::<Vec<String>>()
    .join(", ");
  return format!("[{}]", elems);
}

// ===========================================================
// HEAP STATE

// Struct used to store interesting parts of runtime state
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct RuntimeStateTest {
  checksum: u64,
  mana: u128,
  size: i128,
}
impl RuntimeStateTest {
  pub fn new(fn_names: &[&str], rt: &mut Runtime) -> RuntimeStateTest {
    RuntimeStateTest {
      checksum: test_heap_checksum(&fn_names, rt),
      mana: rt.get_mana(),
      size: rt.get_size(),
    }
  }
}

// Generate a checksum for a runtime state (for testing)
pub fn test_heap_checksum(fn_names: &[&str], rt: &mut Runtime) -> u64 {
  let fn_ids = fn_names.iter().map(|x| name_to_u128(x)).collect::<Vec<u128>>();
  let mut hasher = DefaultHasher::new();
  for fn_id in fn_ids {
    let term_lnk = rt.read_disk(fn_id);
    if let Some(term_lnk) = term_lnk {
      let term_lnk = show_term(rt, term_lnk, None);

      // dbg!(term_lnk.clone());
      term_lnk.hash(&mut hasher);
    }
  }
  let res = hasher.finish();
  // dbg!(res);
  res
}

// ===========================================================
// RUNTIME ROLLBACK
pub fn rollback(rt: &mut Runtime, tick: u128, pre_code: Option<&str>, code: Option<&str>) {
  debug_assert!(tick < rt.get_tick());
  rt.rollback(tick);
  if rt.get_tick() == 0 {
    if let Some(pre_code) = pre_code {
      rt.run_statements_from_code(pre_code, true);
    }
  }
  let tick_diff = tick - rt.get_tick();
  for _ in 0..tick_diff {
    if let Some(code) = code {
      rt.run_statements_from_code(code, true);
    }
    rt.tick();
  }
  // println!("- final rollback tick {}", rt.get_tick());
}

pub fn advance(rt: &mut Runtime, tick: u128, code: Option<&str>) {
  debug_assert!(tick >= rt.get_tick());
  // println!("- advancing from {} to {}", rt.get_tick(), tick);
  let actual_tick = rt.get_tick();
  for _ in actual_tick..tick {
    if let Some(code) = code {
      rt.run_statements_from_code(code, true);
    }
    rt.tick();
  }
}

/// Tests the rollback of states in the kindelia runtime
///
/// # Arguments
///
/// * `pre_code` - Like a genesis block, use this to deploy functions and contracts
/// * `code` - The code that will be executed `total_tick` times, use this to test the states
/// * `fn_names` - The names of the functions which states will be tested
/// * `total_tick` - The number of times the code will be executed
/// * `rollback_tick` - The tick to rollback to
///
pub fn rollback_simple(
  pre_code: &str,
  code: &str,
  fn_names: &[&str],
  total_tick: u128,
  rollback_tick: u128,
  dir_path: &PathBuf,
) -> bool {
  let mut rt = init_runtime(Some(&dir_path));

  // Calculate all total_tick states and saves old checksum
  let mut old_state = RuntimeStateTest::new(fn_names, &mut rt);
  rt.run_statements_from_code(pre_code, true);
  for _ in 0..total_tick {
    rt.run_statements_from_code(code, true);
    rt.tick();
    // dbg!(test_heap_checksum(&fn_names, &mut rt));
    if rt.get_tick() == rollback_tick {
      old_state = RuntimeStateTest::new(fn_names, &mut rt);
    }
  }
  // Does rollback to nearest rollback_tick saved state
  rt.rollback(rollback_tick);
  if rt.get_tick() == 0 {
    rt.run_statements_from_code(pre_code, true);
  }
  // Run until rollback_tick
  let tick_diff = rollback_tick - rt.get_tick();
  for _ in 0..tick_diff {
    rt.run_statements_from_code(code, true);
    rt.tick();
  }
  // Calculates new checksum, after rollback
  let new_state = RuntimeStateTest::new(fn_names, &mut rt);
  // dbg!(old_state.clone(), new_state.clone());
  // Returns if checksums are equal
  old_state == new_state
}

// Does basically the same of rollback_simple, but with a path
// This path tells kindelia where to go
pub fn rollback_path(
  pre_code: &str,
  code: &str,
  fn_names: &[&str],
  path: &[u128],
  dir_path: &PathBuf,
) -> bool {
  let mut states_store: HashMap<u128, Vec<RuntimeStateTest>> = HashMap::new();
  let mut insert_state = |rt: &mut Runtime| {
    let state = RuntimeStateTest::new(fn_names, rt);
    let vec = states_store.get_mut(&rt.get_tick());
    if let Some(vec) = vec {
      vec.push(state);
    } else {
      states_store.insert(rt.get_tick(), vec![state]);
    }
  };

  let mut rt = init_runtime(Some(dir_path));
  rt.run_statements_from_code(pre_code, true);

  for tick in path {
    let tick = *tick;
    if tick < rt.get_tick() {
      rollback(&mut rt, tick, Some(pre_code), Some(code));
    } else {
      advance(&mut rt, tick, Some(code));
    }
    insert_state(&mut rt);
  }

  // dbg!(states_store.clone());
  // Verify if all values from all vectors from all ticks of interest are equal
  states_store.values().all(|vec| are_all_elemenets_equal(vec))
}

// ===========================================================
// BEFORE EACH

// This struct is created just to wrap Pathbuf and
// be able to remove the dir when it is dropped
pub struct TempDir {
  pub path: PathBuf,
}

impl Drop for TempDir {
  fn drop(&mut self) {
    if let Err(e) = std::fs::remove_dir_all(&self.path) {
      eprintln!("Error removing temp dir: {:?}", e);
    }
  }
}

// fixture from rstest library
// Creates a temporary dir and returns a TempDir struct
// before each test that uses it as a parameter
#[fixture]
pub fn temp_dir() -> TempDir {
  let path = std::env::temp_dir().join(format!("kindelia.{:x}", fastrand::u128(..)));
  let temp_dir = TempDir { path };
  println!("Temp dir: {:?}", temp_dir.path);
  temp_dir
}
