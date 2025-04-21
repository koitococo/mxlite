use std::{
  collections::{BTreeMap, VecDeque},
  sync::{Arc, RwLock},
};

use log::error;

pub trait Mailbox<Tag, Msg> {
  fn send(&self, tag: Tag, msg: Msg) -> bool;
  fn receive(&self, tag: &Tag) -> Option<Arc<Msg>>;
  fn clear(&self);
  fn list(&self) -> Vec<Tag>;
}

struct SimpleMailboxInner<Tag, Msg> {
  storage: BTreeMap<Tag, (bool, Arc<Msg>)>,
  clear_queue: VecDeque<Tag>,
}

#[derive(Clone)]
pub struct SimpleMailbox<Tag, Msg> {
  _inner: Arc<RwLock<SimpleMailboxInner<Tag, Msg>>>,
  capacity: usize,
}

impl<Tag: Ord + Clone, Msg> SimpleMailbox<Tag, Msg> {
  pub fn new(capacity: usize) -> Self {
    SimpleMailbox {
      _inner: Arc::new(RwLock::new(SimpleMailboxInner {
        storage: BTreeMap::new(),
        clear_queue: VecDeque::new(),
      })),
      capacity,
    }
  }

  pub fn capacity(&self) -> usize { self.capacity }
}

impl<Tag: Ord + Clone, Msg> SimpleMailboxInner<Tag, Msg> {
  fn remove_first_read(&mut self) {
    if let Some(tag) = self.clear_queue.pop_front() {
      self.storage.remove(&tag);
    }
  }

  fn read(&mut self, tag: &Tag) -> Option<Arc<Msg>> {
    if let Some((read, msg)) = self.storage.get_mut(tag) {
      *read = true;
      self.clear_queue.push_back(tag.clone());
      Some(msg.clone())
    } else {
      None
    }
  }

  fn insert(&mut self, tag: Tag, msg: Msg, limit: usize) -> bool {
    while !self.clear_queue.is_empty() && self.storage.len() >= limit {
      self.remove_first_read();
    }
    if self.storage.len() >= limit {
      return false;
    }
    self.storage.insert(tag, (false, Arc::new(msg)));
    true
  }
}

impl<Tag: Ord + Clone, Msg> Mailbox<Tag, Msg> for SimpleMailbox<Tag, Msg> {
  fn send(&self, tag: Tag, msg: Msg) -> bool {
    let guard = self._inner.write();
    if let Err(e) = guard {
      error!("Failed to acquire write lock: {}", e);
      return false;
    }
    let mut guard = guard.unwrap();
    guard.insert(tag, msg, self.capacity)
  }

  fn receive(&self, tag: &Tag) -> Option<Arc<Msg>> {
    let guard = self._inner.write();
    if let Err(e) = guard {
      error!("Failed to acquire write lock: {}", e);
      return None;
    }
    let mut guard = guard.unwrap();
    guard.read(tag)
  }

  fn clear(&self) {
    let guard = self._inner.write();
    if let Err(e) = guard {
      error!("Failed to acquire write lock: {}", e);
      return;
    }
    let mut guard = guard.unwrap();
    guard.storage.clear();
    guard.clear_queue.clear();
  }

  fn list(&self) -> Vec<Tag> {
    let guard = self._inner.read();
    if let Err(e) = guard {
      error!("Failed to acquire write lock: {}", e);
      return Vec::with_capacity(0);
    }
    let guard = guard.unwrap();
    guard.storage.keys().cloned().collect()
  }
}
