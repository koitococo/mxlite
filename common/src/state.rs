use std::{
    collections::{BTreeMap, btree_map::Entry},
    sync::Arc,
};

use tokio::sync::RwLock;

pub trait StateStorage<Key, State> {
    fn new() -> Self;
    fn set(&self, key: Key, item: State) -> impl std::future::Future<Output = ()>;
    fn get(&self, key: &Key) -> impl std::future::Future<Output = Option<Arc<State>>>;
    fn remove(&self, key: &Key) -> impl std::future::Future<Output = ()>;
    fn list(&self) -> impl std::future::Future<Output = Vec<Key>>;
    fn has(&self, key: &Key) -> impl std::future::Future<Output = bool>;
    fn add(&self, key: Key, item: State) -> impl std::future::Future<Output = bool>;
    fn replace(&self, key: Key, item: State) -> impl std::future::Future<Output = bool>;
}

#[derive(Clone)]
pub struct AtomticStateStorage<Key, T> {
    _inner: Arc<RwLock<BTreeMap<Key, Arc<T>>>>,
}

impl<Key: Ord + Clone, T> StateStorage<Key, T> for AtomticStateStorage<Key, T> {
    fn new() -> Self {
        AtomticStateStorage {
            _inner: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    async fn set(&self, key: Key, item: T) {
        let mut guard = self._inner.write().await;
        guard.insert(key, Arc::new(item));
    }

    async fn get(&self, key: &Key) -> Option<Arc<T>> {
        let guard = self._inner.read().await;
        guard.get(key).cloned()
    }

    async fn remove(&self, key: &Key) {
        let mut guard = self._inner.write().await;
        guard.remove(key);
    }

    async fn list(&self) -> Vec<Key> {
        let guard = self._inner.read().await;
        guard.keys().cloned().collect()
    }

    async fn has(&self, key: &Key) -> bool {
        let guard = self._inner.read().await;
        guard.contains_key(key)
    }

    async fn add(&self, key: Key, item: T) -> bool {
        let mut guard = self._inner.write().await;
        if guard.contains_key(&key) {
            return false;
        }
        guard.insert(key, Arc::new(item));
        true
    }

    async fn replace(&self, key: Key, item: T) -> bool {
        let mut guard = self._inner.write().await;
        if let Entry::Occupied(mut e) = guard.entry(key) {
            e.insert(Arc::new(item));
            true
        } else {
            false
        }
    }
}

impl<Key: Ord + Clone, T> AtomticStateStorage<Key, T> {
    pub async fn map<F>(&self, key: Key, f: F) -> bool
    where
        F: FnOnce(&T) -> Option<T>,
    {
        let mut guard = self._inner.write().await;
        if let Entry::Occupied(mut e) = guard.entry(key) {
            if let Some(new_val) = f(e.get()) {
                e.insert(Arc::new(new_val));
            }
            true
        } else {
            false
        }
    }

    pub async fn map_async<F>(&self, key: Key, f: F) -> bool
    where
        F: AsyncFnOnce(&T) -> Option<T>,
    {
        let mut guard = self._inner.write().await;
        if let Entry::Occupied(mut e) = guard.entry(key) {
            if let Some(new_val) = f(e.get()).await {
                e.insert(Arc::new(new_val));
            }
            true
        } else {
            false
        }
    }
}
