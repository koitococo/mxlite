use log::debug;

use crate::utils::{execute_command, execute_shell};
use anyhow::Result;
use common::protocol::controller::{AgentResponsePayload, CommandExecutionRequest, CommandExecutionResponse};

use super::TaskHandler;

pub(super) struct ExecutionTask {
  cmd: String,
  args: Vec<String>,
  use_script_file: bool,
  use_shell: bool,
}

impl TaskHandler for ExecutionTask {
  async fn handle(self) -> Result<AgentResponsePayload> {
    let (code, stdout, stderr) = if self.use_shell {
      execute_shell(&self.cmd, self.use_script_file).await?
    } else {
      execute_command(&self.cmd, self.args).await?
    };
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
      args: value.args.clone().unwrap_or(Vec::with_capacity(0)),
      use_script_file: value.use_script_file.unwrap_or(false),
      use_shell: value.use_shell.unwrap_or(true),
    }
  }
}
