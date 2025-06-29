use std::{
  collections::{BTreeMap, btree_map::Entry},
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

impl<Key, State> Default for StateMap<Key, State> {
  fn default() -> Self { Self::new() }
}

impl<Key, State> StateMap<Key, State> {
  pub fn new() -> Self {
    StateMap {
      _inner: Arc::new(RwLock::new(BTreeMap::new())),
    }
  }
}

impl<Key: Ord + Clone, State> States<Key, State> for StateMap<Key, State> {
  fn insert(&self, key: Key, item: State) -> bool {
    let Ok(mut guard) = self._inner.write() else {
      return false;
    };
    guard.insert(key, Arc::new(item));
    true
  }

  fn get_arc(&self, key: &Key) -> Option<Arc<State>> {
    let Ok(guard) = self._inner.read() else {
      return None;
    };
    guard.get(key).cloned()
  }

  fn remove(&self, key: &Key) {
    let Ok(mut guard) = self._inner.write() else {
      return;
    };
    guard.remove(key);
  }

  fn list(&self) -> Vec<Key> {
    let Ok(guard) = self._inner.read() else {
      return Vec::with_capacity(0);
    };
    guard.keys().cloned().collect()
  }
}

impl<Key: Ord + Clone, State> StateMap<Key, State> {
  pub fn try_insert_deferred_returning<F>(&self, key: Key, f: F) -> Option<Arc<State>>
  where F: FnOnce() -> State {
    let Ok(mut guard) = self._inner.write() else {
      return None;
    };
    match guard.entry(key) {
      Entry::Vacant(vacant_entry) => {
        let val = Arc::new(f());
        vacant_entry.insert(val.clone());
        Some(val)
      }
      Entry::Occupied(occupied_entry) => Some(occupied_entry.get().clone()),
    }
  }

  pub fn take_if(&self, key: Key, predicate: impl FnOnce(&State) -> bool) -> Option<Arc<State>> {
    let v = self.get_arc(&key);
    if let Some(v) = v {
      if predicate(&v) {
        self.remove(&key);
      }
      return Some(v);
    }
    None
  }
}
