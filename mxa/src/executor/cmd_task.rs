use log::debug;

use crate::utils::execute_shell;
use anyhow::Result;
use common::protocol::controller::{AgentResponsePayload, CommandExecutionRequest, CommandExecutionResponse};

use super::TaskHandler;

pub(super) struct ExecutionTask {
  cmd: String,
  use_script_file: bool,
}

impl TaskHandler for ExecutionTask {
  async fn handle(self) -> Result<AgentResponsePayload> {
    let (code, stdout, stderr) = execute_shell(&self.cmd, self.use_script_file).await?;
    debug!(
      "Command '{}' executed with code {}: {} {}",
      self.cmd, code, stdout, stderr
    );
    Ok(CommandExecutionResponse { code, stdout, stderr }.into())
  }
}

impl From<&CommandExecutionRequest> for ExecutionTask {
  fn from(value: &CommandExecutionRequest) -> Self {
    ExecutionTask {
      cmd: value.command.clone(),
      use_script_file: value.use_script_file,
    }
  }
}
