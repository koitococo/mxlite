use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub const PROTOCOL_VERSION: u32 = 3;
// pub const CLOSE_CODE : u16 = 1000;
// pub const CLOSE_MXA_SHUTDOWN: &str = "MXA_SHUTDOWN";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Message {
  None,
  ControllerRequest(ControllerRequest),
  AgentResponse(AgentResponse),
}

impl FromStr for Message {
  type Err = serde_json::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> { serde_json::from_str(s) }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for Message {
  fn to_string(&self) -> String { serde_json::to_string(self).unwrap() }
}

mod requests;
pub use requests::*;
