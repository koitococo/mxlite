use std::{
    collections::{BTreeMap, btree_map::Entry},
    sync::{Arc, RwLock},
};

pub trait Mailbox<Tag, Msg> {
    fn new() -> Self;
    fn send(&self, tag: Tag, msg: Msg) -> bool;
    fn receive(&self, tag: &Tag) -> Option<Arc<Msg>>;
    fn get_read(&self, tag: &Tag) -> Option<bool>;
    fn set_read(&self, tag: &Tag, read: bool) -> bool;
    fn delete(&self, tag: &Tag) -> bool;
    fn clear(&self);
    fn list(&self) -> Vec<Tag>;
    fn gc(&self);
}

#[derive(Clone)]
pub struct SimpleMailbox<Tag, Msg> {
    #[allow(clippy::type_complexity)]
    _inner: Arc<RwLock<BTreeMap<Tag, (bool, Arc<Msg>)>>>,
}

impl<Tag: Ord + Clone, Msg> Mailbox<Tag, Msg> for SimpleMailbox<Tag, Msg> {
    fn new() -> Self {
        SimpleMailbox {
            _inner: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    fn send(&self, tag: Tag, msg: Msg) -> bool {
        let guard = self._inner.write();
        if guard.is_err() {
            return false;
        }
        let mut guard = guard.unwrap();
        match guard.entry(tag) {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                entry.insert((false, Arc::new(msg)));
                true
            }
        }
    }

    fn receive(&self, tag: &Tag) -> Option<Arc<Msg>> {
        let guard = self._inner.write();
        if guard.is_err() {
            return None;
        }
        let mut guard = guard.unwrap();
        if let Some((read, msg)) = guard.get_mut(tag) {
            *read = true;
            Some(msg.clone())
        } else {
            None
        }
    }

    fn get_read(&self, tag: &Tag) -> Option<bool> {
        let guard = self._inner.read();
        if guard.is_err() {
            return None;
        }
        let guard = guard.unwrap();
        if let Some((read, _)) = guard.get(tag) {
            Some(*read)
        } else {
            None
        }
    }

    fn set_read(&self, tag: &Tag, read: bool) -> bool {
        let guard = self._inner.write();
        if guard.is_err() {
            return false;
        }
        let mut guard = guard.unwrap();
        if let Some((r, _)) = guard.get_mut(tag) {
            *r = read;
            true
        } else {
            false
        }
    }

    fn delete(&self, tag: &Tag) -> bool {
        let guard = self._inner.write();
        if guard.is_err() {
            return false;
        }
        let mut guard = guard.unwrap();
        guard.remove(tag).is_some()
    }

    fn clear(&self) {
        let guard = self._inner.write();
        if guard.is_err() {
            return;
        }
        let mut guard = guard.unwrap();
        guard.clear();
    }

    fn list(&self) -> Vec<Tag> {
        let guard = self._inner.read();
        if guard.is_err() {
            return Vec::with_capacity(0);
        }
        let guard = guard.unwrap();
        guard.keys().cloned().collect()
    }

    fn gc(&self) {
        let guard = self._inner.write();
        if guard.is_err() {
            return;
        }
        let mut guard = guard.unwrap();
        guard.retain(|_, (read, _)| !*read);
    }
}
