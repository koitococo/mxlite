use std::{
    collections::{BTreeMap, btree_map::Entry},
    sync::Arc,
};

use tokio::sync::Mutex;

pub trait StateStorage<Id, State> {
    fn new() -> Self;
    fn set(&self, id: Id, item: State) -> impl std::future::Future<Output = ()>;
    fn get(&self, id: &Id) -> impl std::future::Future<Output = Option<Arc<State>>>;
    fn del(&self, id: &Id) -> impl std::future::Future<Output = ()>;
    fn list(&self) -> impl std::future::Future<Output = Vec<Id>>;
    fn has(&self, id: &Id) -> impl std::future::Future<Output = bool>;
    fn add(&self, id: Id, item: State) -> impl std::future::Future<Output = bool>;
    fn replace(&self, id: Id, item: State) -> impl std::future::Future<Output = bool>;
}

#[derive(Clone)]
pub struct AtomticStateStorage<Id, T> {
    _inner: Arc<Mutex<BTreeMap<Id, Arc<T>>>>,
}

impl<Id: Ord + Clone, T> StateStorage<Id, T> for AtomticStateStorage<Id, T> {
    fn new() -> Self {
        AtomticStateStorage {
            _inner: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    async fn set(&self, id: Id, item: T) {
        let mut guard = self._inner.lock().await;
        guard.insert(id, Arc::new(item));
    }

    async fn get(&self, id: &Id) -> Option<Arc<T>> {
        let guard = self._inner.lock().await;
        guard.get(id).cloned()
    }

    async fn del(&self, id: &Id) {
        let mut guard = self._inner.lock().await;
        guard.remove(id);
    }

    async fn list(&self) -> Vec<Id> {
        let guard = self._inner.lock().await;
        guard.keys().cloned().collect()
    }

    async fn has(&self, id: &Id) -> bool {
        let guard = self._inner.lock().await;
        guard.contains_key(id)
    }

    async fn add(&self, id: Id, item: T) -> bool {
        let mut guard = self._inner.lock().await;
        if guard.contains_key(&id) {
            return false;
        }
        guard.insert(id, Arc::new(item));
        true
    }

    async fn replace(&self, id: Id, item: T) -> bool {
        let mut guard = self._inner.lock().await;
        if let Entry::Occupied(mut e) = guard.entry(id) {
            e.insert(Arc::new(item));
            true
        } else {
            false
        }
    }
}
