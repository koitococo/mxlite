use std::{
  collections::{BTreeMap, btree_map::Entry},
  fmt::Debug,
  sync::{Arc, RwLock},
};

pub trait StateStorage<Key, State> {
  fn new() -> Self;
  fn insert(&self, key: Key, item: State);
  fn get(&self, key: &Key) -> Option<Arc<State>>;
  fn remove(&self, key: &Key);
  fn list(&self) -> Vec<Key>;
  fn contains(&self, key: &Key) -> bool;
  fn map<F>(&self, key: Key, f: F) -> bool
  where F: FnOnce(&State) -> Option<State>;
}

#[derive(Clone)]
pub struct AtomticStateStorage<Key, State> {
  _inner: Arc<RwLock<BTreeMap<Key, Arc<State>>>>,
}

impl<Key: Ord + Clone + Debug, State> StateStorage<Key, State> for AtomticStateStorage<Key, State> {
  fn new() -> Self {
    AtomticStateStorage {
      _inner: Arc::new(RwLock::new(BTreeMap::new())),
    }
  }

  fn insert(&self, key: Key, item: State) {
    let guard = self._inner.write();
    if guard.is_err() {
      return;
    }
    let mut guard = guard.unwrap();
    guard.insert(key, Arc::new(item));
  }

  fn get(&self, key: &Key) -> Option<Arc<State>> {
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

  fn contains(&self, key: &Key) -> bool {
    let guard = self._inner.read();
    if guard.is_err() {
      return false;
    }
    let guard = guard.unwrap();
    guard.contains_key(key)
  }

  fn map<F>(&self, key: Key, f: F) -> bool
  where F: FnOnce(&State) -> Option<State> {
    if let Ok(mut guard) = self._inner.write() {
      if let Entry::Occupied(mut e) = guard.entry(key) {
        if let Some(new_val) = f(e.get()) {
          e.insert(Arc::new(new_val));
        }
        return true;
      }
    }
    false
  }
}

impl<Key: Ord + Clone, State> AtomticStateStorage<Key, State> {
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
}
