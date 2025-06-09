use anyhow::Result;

use common::protocol::messaging::{ErrorResponse, ScriptEvalRequest, ScriptEvalResponse};

use super::RequestHandler;

const ERR_SCRIPT_CONTEXT: &str = "ERR_SCRIPT_CONTEXT";
const ERR_SCRIPT_EVAL: &str = "ERR_SCRIPT_EVAL";

impl RequestHandler<ScriptEvalResponse> for ScriptEvalRequest {
  async fn handle(&self) -> Result<ScriptEvalResponse, ErrorResponse> {
    let Ok(ctx) = crate::script::ExecutorContext::try_new() else {
    return Err(ErrorResponse {
        code: ERR_SCRIPT_CONTEXT.to_string(),
        message: "Failed to create script execution context".to_string(),
      });
    };
    let Ok(result) = ctx.eval_async(&self.script).await else {
      return Err(ErrorResponse {
        code: ERR_SCRIPT_EVAL.to_string(),
        message: "Script evaluation failed".to_string(),
      });
    };
    Ok(ScriptEvalResponse {
      ok: true,
      result: result.to_string(),
    })
  }
}
