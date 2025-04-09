use std::{
    collections::{BTreeMap, btree_map::Entry},
    sync::{Arc, RwLock},
};

pub trait StateStorage<Key, State> {
    fn new() -> Self;
    fn set(&self, key: Key, item: State);
    fn get(&self, key: &Key) -> Option<Arc<State>>;
    fn remove(&self, key: &Key);
    fn list(&self) -> Vec<Key>;
    fn has(&self, key: &Key) -> bool;
    fn add(&self, key: Key, item: State) -> bool;
    fn replace(&self, key: Key, item: State) -> bool;
    fn map<F>(&self, key: Key, f: F) -> bool
    where
        F: FnOnce(&State) -> Option<State>;
    // fn map_async<F>(&self, key: Key, f: F) -> impl Future<Output = bool>
    // where
    //     F: AsyncFnOnce(&State) -> Option<State>;
}

#[derive(Clone)]
pub struct AtomticStateStorage<Key, State> {
    _inner: Arc<RwLock<BTreeMap<Key, Arc<State>>>>,
}

impl<Key: Ord + Clone, State> StateStorage<Key, State> for AtomticStateStorage<Key, State> {
    fn new() -> Self {
        AtomticStateStorage {
            _inner: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    fn set(&self, key: Key, item: State) {
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

    fn has(&self, key: &Key) -> bool {
        let guard = self._inner.read();
        if guard.is_err() {
            return false;
        }
        let guard = guard.unwrap();
        guard.contains_key(key)
    }

    fn add(&self, key: Key, item: State) -> bool {
        let guard = self._inner.write();
        if guard.is_err() {
            return false;
        }
        let mut guard = guard.unwrap();
        if guard.contains_key(&key) {
            return false;
        }
        guard.insert(key, Arc::new(item));
        true
    }

    fn replace(&self, key: Key, item: State) -> bool {
        let guard = self._inner.write();
        if guard.is_err() {
            return false;
        }
        let mut guard = guard.unwrap();
        if let Entry::Occupied(mut e) = guard.entry(key) {
            e.insert(Arc::new(item));
            true
        } else {
            false
        }
    }

    fn map<F>(&self, key: Key, f: F) -> bool
    where
        F: FnOnce(&State) -> Option<State>,
    {
        let guard = self._inner.write();
        if guard.is_err() {
            return false;
        }
        let mut guard = guard.unwrap();
        if let Entry::Occupied(mut e) = guard.entry(key) {
            if let Some(new_val) = f(e.get()) {
                e.insert(Arc::new(new_val));
            }
            true
        } else {
            false
        }
    }

    // fn map_async<F>(&self, key: Key, f: F) -> bool
    // where
    //     F: AsyncFnOnce(&State) -> Option<State>,
    // {
    //     let mut guard = self._inner.write();
    //     if let Entry::Occupied(mut e) = guard.entry(key) {
    //         if let Some(new_val) = f(e.get()) {
    //             e.insert(Arc::new(new_val));
    //         }
    //         true
    //     } else {
    //         false
    //     }
    // }
}
