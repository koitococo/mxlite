use std::{
  error::Error,
  fmt::{Debug, Display},
  sync::Arc,
};

use tokio::{
  select,
  sync::{mpsc, oneshot},
};

use super::{
  bus::{Bus, SimpleBus},
  queue::{Queue, SimpleQueue},
};

pub trait Transport {
  type Item;
  type Error: Error;
  fn send(&self, data: &Self::Item) -> impl Future<Output = Result<(), Self::Error>>;
  fn recv(&self) -> impl Future<Output = Result<Option<Self::Item>, Self::Error>>;
}

pub enum MuxError<K, D, T: Transport<Item = (K, D)>> {
  TransportError(T::Error),
  SendCallbackError(oneshot::error::RecvError),
  SendError(mpsc::error::SendError<(K, D, oneshot::Sender<Result<(), T::Error>>)>),
  ClosedTransport,
}
impl<K, D, T: Transport<Item = (K, D)>> Display for MuxError<K, D, T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      MuxError::TransportError(e) => write!(f, "Transport error: {}", e),
      MuxError::SendCallbackError(e) => write!(f, "Send callback error: {}", e),
      MuxError::SendError(e) => write!(f, "Send error: {}", e),
      MuxError::ClosedTransport => write!(f, "Closed transport"),
    }
  }
}
impl<K, D, T: Transport<Item = (K, D)>> Debug for MuxError<K, D, T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::TransportError(arg0) => f.debug_tuple("TransportError").field(arg0).finish(),
      Self::SendCallbackError(arg0) => f.debug_tuple("SendCallbackError").field(arg0).finish(),
      Self::SendError(arg0) => f.debug_tuple("SendError").field(arg0).finish(),
      Self::ClosedTransport => write!(f, "ClosedTransport"),
    }
  }
}
impl<K, D, T: Transport<Item = (K, D)>> Error for MuxError<K, D, T> {}

pub struct Mux<K, D, T: Transport<Item = (K, D)>> {
  transport: T,
  tx_queue: SimpleQueue<(K, D, oneshot::Sender<Result<(), T::Error>>)>,
  _rx: SimpleBus<K, D>,
}

impl<K, D, T: Transport<Item = (K, D)>> Mux<K, D, T>
where K: Ord
{
  pub fn new(transport: T) -> Self {
    Mux {
      transport,
      tx_queue: SimpleQueue::new(),
      _rx: SimpleBus::new(),
    }
  }

  pub async fn worker(&self) {
    loop {
      select! {
          v = self._recv() => {
              if !self._recv2(v).await {
                  break;
              }
          },
          v = self._send() => {
              self._send2(v).await;
          }
      }
    }
  }

  async fn _recv(&self) -> Option<(K, D)> { self.transport.recv().await.unwrap_or(None) }

  async fn _recv2(&self, v: Option<(K, D)>) -> bool {
    if let Some((k, d)) = v {
      self._rx.dispatch(k, d).await.is_ok()
    } else {
      false
    }
  }

  async fn _send(&self) -> Option<(K, D, oneshot::Sender<Result<(), T::Error>>)> { self.tx_queue.pop().await }

  async fn _send2(&self, v: Option<(K, D, oneshot::Sender<Result<(), T::Error>>)>) {
    if let Some((k, d, tx)) = v {
      let r = self.transport.send(&(k, d)).await;
      tx.send(r).unwrap_or(());
    }
  }

  pub async fn send(&self, alt: K, data: D) -> Result<(), MuxError<K, D, T>> {
    let (tx, rx) = oneshot::channel::<Result<(), T::Error>>();
    self.tx_queue.push((alt, data, tx)).await.map_err(MuxError::<K, D, T>::SendError)?;
    let _ = rx.await.map_err(MuxError::SendCallbackError)?;
    Ok(())
  }

  pub async fn recv(&self, alt: K) -> Option<D> { self._rx.listen(alt).await }
}

pub struct MuxTransport<K, D, T: Transport<Item = (K, D)>> {
  mux: Arc<Mux<K, D, T>>,
  alt: K,
}

impl<K: Clone + Ord, D: Clone, T: Transport<Item = (K, D)>> Transport for MuxTransport<K, D, T> {
  type Error = MuxError<K, D, T>;
  type Item = D;

  async fn send(&self, data: &D) -> Result<(), Self::Error> { self.mux.send(self.alt.clone(), data.clone()).await }

  async fn recv(&self) -> Result<Option<D>, Self::Error> { Ok(self.mux.recv(self.alt.clone()).await) }
}

impl<K: Clone + Ord, D: Clone, T: Transport<Item = (K, D)>> MuxTransport<K, D, T> {
  pub fn new(mux: Arc<Mux<K, D, T>>, alt: K) -> Self { MuxTransport { mux, alt } }
}
