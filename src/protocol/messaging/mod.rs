mod requsting;
pub use requsting::*;

use serde::{Deserialize, Serialize};

// pub const CLOSE_CODE : u16 = 1000;
// pub const CLOSE_MXA_SHUTDOWN: &str = "MXA_SHUTDOWN";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Message {
  None,
  ControllerRequest(ControllerRequest),
  AgentResponse(AgentResponse),
}

impl TryFrom<&str> for Message {
  type Error = serde_json::Error;

  fn try_from(s: &str) -> Result<Self, Self::Error> { serde_json::from_str(s) }
}

impl TryFrom<Message> for String {
  type Error = serde_json::Error;

  fn try_from(value: Message) -> Result<Self, Self::Error> { serde_json::to_string(&value) }
}

impl From<ControllerRequest> for Message {
  fn from(value: ControllerRequest) -> Self { Message::ControllerRequest(value) }
}
impl From<AgentResponse> for Message {
  fn from(value: AgentResponse) -> Self { Message::AgentResponse(value) }
}
