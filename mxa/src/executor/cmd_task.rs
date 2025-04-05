use anyhow::Result;
use log::{trace, warn};

use crate::net::Context;
use crate::utils::execute_shell;
use common::messages::{AgentResponsePayload, CommandExecutionRequest, CommandExecutionResponse};

use super::TaskHandler;

pub(super) struct ExecuteTask {
    cmd: String,
    use_script_file: bool,
}

impl TaskHandler for ExecuteTask {
    async fn handle(self, ctx: Context) -> Result<()> {
        match execute_shell(&self.cmd, self.use_script_file).await {
            Ok((code, stdout, stderr)) => {
                trace!(
                    "Command '{}' executed with code {}: {} {}",
                    self.cmd, code, stdout, stderr
                );
                ctx.respond2(
                    true,
                    AgentResponsePayload::CommandExecutionResponse(CommandExecutionResponse {
                        code,
                        stdout,
                        stderr,
                    }),
                )
                .await;
            }
            Err(err) => {
                warn!("Failed to execute command {}: {}", self.cmd, err);
                ctx.respond2(
                    false,
                    AgentResponsePayload::CommandExecutionResponse(CommandExecutionResponse {
                        code: -1,
                        stdout: "".to_string(),
                        stderr: err.to_string(),
                    }),
                )
                .await;
            }
        }
        Ok(())
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
