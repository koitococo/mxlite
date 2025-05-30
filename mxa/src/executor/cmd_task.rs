use log::debug;

use crate::utils::{execute_command, execute_shell};
use anyhow::Result;
use common::protocol::messaging::{CommandExecutionRequest, CommandExecutionResponse, ErrorResponse};

use super::RequestHandler;
  
impl RequestHandler<CommandExecutionResponse> for CommandExecutionRequest {
  async fn handle(&self) -> Result<CommandExecutionResponse, ErrorResponse> {
    let Ok((code, stdout, stderr)) = (if self.use_shell.unwrap_or(true) {
      execute_shell(&self.command, self.use_script_file.unwrap_or(false)).await
    } else {
      execute_command(&self.command, self.args.clone().unwrap_or_default()).await
    }) else {
      return Err(ErrorResponse {
        code: "ERR_COMMAND_EXECUTION".to_string(),
        message: "Command execution failed".to_string(),
      });
    };
    debug!(
      "Command '{}' executed with code {}: {} {}",
      self.command, code, stdout, stderr
    );
    Ok(CommandExecutionResponse { code, stdout, stderr })
  }
}