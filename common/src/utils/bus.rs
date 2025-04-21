use std::{
  collections::{BTreeMap, btree_map::Entry},
  sync::Arc,
};

use tokio::sync::{Mutex, mpsc};

pub trait Bus<Event, Data> {
  type Error;
  fn dispatch(&self, event: Event, data: Data) -> impl Future<Output = Result<(), Self::Error>>;
  fn listen(&self, event: Event) -> impl Future<Output = Option<Data>>;
}

pub struct SimpleBus<Event, Data> {
  _inner: Mutex<Option<BTreeMap<Event, Box<(Option<mpsc::Sender<Data>>, Arc<Mutex<mpsc::Receiver<Data>>>)>>>>,
}

impl<Event, Data> Default for SimpleBus<Event, Data>
where Event: Ord
{
  fn default() -> Self { Self::new() }
}

impl<Event, Data> SimpleBus<Event, Data>
where Event: Ord
{
  pub fn new() -> Self {
    Self {
      _inner: Mutex::new(None),
    }
  }

  async fn get_inner(&self, event: Event) -> Box<(Option<mpsc::Sender<Data>>, Arc<Mutex<mpsc::Receiver<Data>>>)> {
    let mut inner = self._inner.lock().await;
    let map = {
      if let Some(map) = inner.as_mut() {
        map
      } else {
        inner.insert(BTreeMap::new())
      }
    };
    map
      .entry(event)
      .or_insert_with(|| {
        let (tx, rx) = mpsc::channel(32);
        Box::new((Some(tx), Arc::new(Mutex::new(rx))))
      })
      .clone()
  }

  pub async fn reset(&self, event: Event) {
    let mut inner = self._inner.lock().await;
    if let Some(map) = inner.as_mut() {
      if let Entry::Occupied(e) = map.entry(event) {
        let mut v = e.remove();
        let (tx, rx) = v.as_mut();
        drop(tx.take());
        let mut rx = rx.lock().await;
        while rx.recv().await.is_some() {}
      }
    }
  }
}

impl<Event, Data> Bus<Event, Data> for SimpleBus<Event, Data>
where Event: Ord
{
  type Error = Option<mpsc::error::SendError<Data>>;

  async fn dispatch(&self, event: Event, data: Data) -> Result<(), Self::Error> {
    let mut inner = self.get_inner(event).await;
    let (tx, _) = inner.as_mut();
    let tx = tx.as_mut().ok_or(None)?;
    tx.send(data).await.map_err(Some)
  }

  async fn listen(&self, event: Event) -> Option<Data> {
    let inner = self.get_inner(event).await;
    let (_, rx) = inner.as_ref();
    let mut rx = rx.lock().await;
    rx.recv().await
  }
}
