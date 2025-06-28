use crate::utils::util::safe_sleep;

pub enum Retry<T> {
  Break,
  Return(T),
  RetryImmediate,
  RetryWithDelay,
}

pub enum RetryResult<T> {
  Break,
  Return(T),
  NoResult,
}

pub async fn async_with_retry<F, R, Fu>(mut f: F, retries: i32) -> RetryResult<R>
where
  F: FnMut() -> Fu,
  Fu: Future<Output = Retry<R>>,
{
  for i in 0..retries {
    match f().await {
      Retry::Break => return RetryResult::Break,
      Retry::Return(result) => return RetryResult::Return(result),
      Retry::RetryImmediate => continue,
      Retry::RetryWithDelay => {
        if safe_sleep(((1.5f32).powi(i) * 3000f32 + 2000f32) as u64).await {
          return RetryResult::Break;
        }
      }
    }
  }
  RetryResult::NoResult
}
