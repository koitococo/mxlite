pub(crate) struct ExecutorContext {
  ctx: crate::script::ExecutorContext,
}

impl ExecutorContext {
  pub fn try_new() -> anyhow::Result<Self> {
    let ctx = crate::script::ExecutorContext::try_new()?;
    Ok(ExecutorContext { ctx })
  }

  pub async fn exec_async(&self, script: &str) -> anyhow::Result<()> {
    self.ctx.exec_async(script).await
  }
}


