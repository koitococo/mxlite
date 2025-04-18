use anyhow::Result;
use log::{debug, warn};

use crate::utils::execute_shell;
use common::protocol::controller::{AgentResponsePayload, CommandExecutionRequest, CommandExecutionResponse};

use super::TaskHandler;

pub(super) struct ExecuteTask {
  cmd: String,
  use_script_file: bool,
}

impl TaskHandler for ExecuteTask {
  async fn handle(self) -> Result<(bool, AgentResponsePayload)> {
    match execute_shell(&self.cmd, self.use_script_file).await {
      Ok((code, stdout, stderr)) => {
        debug!(
          "Command '{}' executed with code {}: {} {}",
          self.cmd, code, stdout, stderr
        );
        Ok((
          true,
          AgentResponsePayload::CommandExecutionResponse(CommandExecutionResponse { code, stdout, stderr }),
        ))
      }
      Err(err) => {
        warn!("Failed to execute command {}: {}", self.cmd, err);
        Ok((
          false,
          AgentResponsePayload::CommandExecutionResponse(CommandExecutionResponse {
            code: -1,
            stdout: "".to_string(),
            stderr: err.to_string(),
          }),
        ))
      }
    }
  }
}

impl From<&CommandExecutionRequest> for ExecuteTask {
  fn from(value: &CommandExecutionRequest) -> Self {
    ExecuteTask {
      cmd: value.command.clone(),
      use_script_file: value.use_script_file,
    }
  }
}
