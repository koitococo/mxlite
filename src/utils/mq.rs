use std::{
  collections::VecDeque,
  sync::{Arc, RwLock},
};

pub trait Mq<Tag, Msg> {
  fn send(&self, tag: Tag, msg: Msg) -> bool;
  fn receive(&self, tag: &Tag) -> Option<Arc<Msg>>;
  fn clear(&self);
  fn list(&self) -> Vec<Tag>;
}

#[derive(Debug, Clone)]
pub struct VecMq<Tag, Msg> {
  _inner: Arc<RwLock<VecDeque<(Tag, Msg)>>>,
  capacity: usize,
}

impl<Tag, Msg> VecMq<Tag, Msg> {
  pub fn new(capacity: usize) -> Self {
    VecMq {
      _inner: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
      capacity,
    }
  }

  pub fn capacity(&self) -> usize { self.capacity }
}

impl<Tag: Eq + Clone, Msg> Mq<Tag, Msg> for VecMq<Tag, Msg> {
  fn send(&self, tag: Tag, msg: Msg) -> bool {
    let mut guard = self._inner.write().unwrap();
    guard.push_back((tag, msg));
    true
  }

  fn receive(&self, tag: &Tag) -> Option<Arc<Msg>> {
    let mut guard = self._inner.write().unwrap();
    if let Some(pos) = guard.iter().position(|(t, _)| t == tag) {
      let (_, msg) = guard.remove(pos)?;
      Some(Arc::new(msg))
    } else {
      None
    }
  }

  fn clear(&self) {
    let mut guard = self._inner.write().unwrap();
    guard.clear();
  }

  fn list(&self) -> Vec<Tag> {
    let guard = self._inner.read().unwrap();
    guard.iter().map(|(tag, _)| tag.clone()).collect()
  }
}
