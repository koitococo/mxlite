use tokio::sync::{Mutex, mpsc};

pub trait Queue<T> {
  type Error;
  fn push(&self, item: T) -> impl Future<Output = Result<(), Self::Error>>;
  fn pop(&self) -> impl Future<Output = Option<T>>;
}

pub struct SimpleQueue<T> {
  _tx: mpsc::Sender<T>,
  _rx: Mutex<mpsc::Receiver<T>>,
}

impl<T> Queue<T> for SimpleQueue<T> {
  type Error = mpsc::error::SendError<T>;

  async fn pop(&self) -> Option<T> {
    let mut rx = self._rx.lock().await;
    rx.recv().await
  }

  async fn push(&self, item: T) -> Result<(), Self::Error> { self._tx.send(item).await }
}

impl<T> SimpleQueue<T> {
  pub fn new() -> Self {
    let (tx, rx) = mpsc::channel(1024);
    SimpleQueue { _tx: tx, _rx: Mutex::new(rx) }
  }
}
