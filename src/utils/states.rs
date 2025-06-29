use std::{
  collections::{btree_map::Entry, BTreeMap, VecDeque},
  fmt::Debug,
  sync::{Arc, RwLock},
};

pub trait States<Key, State> {
  fn insert(&self, key: Key, item: State) -> bool;
  fn get(&self, key: &Key) -> Option<Arc<State>>;
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
}

#[derive(Debug, Clone)]
pub struct VecState<Tag, Msg> {
  _inner: Arc<RwLock<VecDeque<(Tag, Arc<Msg>)>>>,
  capacity: usize,
}

impl<Tag, Msg> VecState<Tag, Msg> {
  pub fn new(capacity: usize) -> Self {
    VecState {
      _inner: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
      capacity,
    }
  }

  pub fn capacity(&self) -> usize { self.capacity }
}

impl<Tag: Eq + Clone, Msg> States<Tag, Msg> for VecState<Tag, Msg> {
  fn insert(&self, tag: Tag, msg: Msg) -> bool {
    let mut guard = self._inner.write().unwrap();
    while guard.len() >= self.capacity {
      let _ = guard.pop_front();
    }
    guard.push_back((tag, Arc::new(msg)));
    true
  }

  fn get(&self, tag: &Tag) -> Option<Arc<Msg>> {
    let guard = self._inner.read().unwrap();
    if let Some(pos) = guard.iter().position(|(t, _)| t == tag) {
      let (_, msg) = guard.get(pos)?;
      Some(msg.clone())
    } else {
      None
    }
  }

  fn remove(&self, tag: &Tag) {
    let mut guard = self._inner.write().unwrap();
    if let Some(pos) = guard.iter().position(|(t, _)| t == tag) {
      guard.remove(pos);
    }
  }

  fn list(&self) -> Vec<Tag> {
    let guard = self._inner.read().unwrap();
    guard.iter().map(|(tag, _)| tag.clone()).collect()
  }
}
