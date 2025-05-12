use anyhow::Result;

use crate::script::ExecutorContext;
use common::protocol::controller::{AgentResponsePayload, ScriptEvalRequest, ScriptEvalResponse};

use super::TaskHandler;

pub(super) struct ScriptTask {
  script: String,
}

impl TaskHandler for ScriptTask {
  async fn handle(self) -> Result<AgentResponsePayload> {
    let ctx = ExecutorContext::try_new()?;
    let r = ctx.eval_async(&self.script).await?;
    Ok(
      ScriptEvalResponse {
        ok: true,
        result: r.to_string(),
      }
      .into(),
    )
  }
}

impl From<&ScriptEvalRequest> for ScriptTask {
  fn from(value: &ScriptEvalRequest) -> Self {
    ScriptTask {
      script: value.script.clone(),
    }
  }
}
