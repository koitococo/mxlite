use common::script::ValueType;
pub(crate) struct ExecutorContext {
  ctx: common::script::ExecutorContext,
}

impl ExecutorContext {
  pub fn try_new() -> anyhow::Result<Self> {
    let ctx = common::script::ExecutorContext::try_new()?;
    Ok(ExecutorContext { ctx })
  }

  pub async fn exec_async(&self, script: &str) -> anyhow::Result<()> {
    self.ctx.exec_async(script).await
  }

  pub async fn eval_async(&self, script: &str) -> anyhow::Result<ValueType> {
    self.ctx.eval_async(script).await
  }
}



