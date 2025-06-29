use std::{
  collections::{btree_map::Entry, BTreeMap},
  fmt::Debug,
  sync::{Arc, RwLock},
};

pub trait States<Key, State> {
  fn insert(&self, key: Key, item: State) -> bool;

  // #[deprecated]
  fn get_arc(&self, key: &Key) -> Option<Arc<State>>;

  fn remove(&self, key: &Key);
  fn list(&self) -> Vec<Key>;
}

#[derive(Clone)]
pub struct StateMap<Key, State> {
  _inner: Arc<RwLock<BTreeMap<Key, Arc<State>>>>,
}

impl<Key, State> StateMap<Key, State> {
  pub fn new() -> Self {
    StateMap {
      _inner: Arc::new(RwLock::new(BTreeMap::new())),
    }
  }
}

impl<Key: Ord + Clone + Debug, State> States<Key, State> for StateMap<Key, State> {
  fn insert(&self, key: Key, item: State) -> bool {
    let guard = self._inner.write();
    if guard.is_err() {
      return false;
    }
    let mut guard = guard.unwrap();
    guard.insert(key, Arc::new(item));
    true
  }

  fn get_arc(&self, key: &Key) -> Option<Arc<State>> {
    let guard = self._inner.read();
    if guard.is_err() {
      return None;
    }
    let guard = guard.unwrap();
    guard.get(key).cloned()
  }

  fn remove(&self, key: &Key) {
    let guard = self._inner.write();
    if guard.is_err() {
      return;
    }
    let mut guard = guard.unwrap();
    guard.remove(key);
  }

  fn list(&self) -> Vec<Key> {
    let guard = self._inner.read();
    if guard.is_err() {
      return Vec::with_capacity(0);
    }
    let guard = guard.unwrap();
    guard.keys().cloned().collect()
  }
}

impl<Key: Ord + Clone, State> StateMap<Key, State> {
  pub fn try_insert_deferred_returning<F>(&self, key: Key, f: F) -> Option<Arc<State>>
  where F: FnOnce() -> State {
    let guard = self._inner.write();
    if guard.is_err() {
      return None;
    }
    let mut guard = guard.unwrap();
    match guard.entry(key) {
      Entry::Vacant(vacant_entry) => {
        let val = Arc::new(f());
        vacant_entry.insert(val.clone());
        Some(val)
      }
      Entry::Occupied(occupied_entry) => Some(occupied_entry.get().clone()),
    }
  }

  pub fn take(&self, key: &Key) -> Option<Arc<State>> {
    let guard = self._inner.write();
    if guard.is_err() {
      return None;
    }
    let mut guard = guard.unwrap();
    guard.remove(key)
  }
}