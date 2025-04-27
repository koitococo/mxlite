use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub const PROTOCOL_VERSION: u32 = 3;
// pub const CLOSE_CODE : u16 = 1000;
// pub const CLOSE_MXA_SHUTDOWN: &str = "MXA_SHUTDOWN";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ControllerMessage {
  pub request: ControllerRequest,
  pub events: Option<Vec<ControllerEvent>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentMessage {
  pub response: Option<AgentResponse>,
  pub events: Option<Vec<AgentEvent>>,
}

impl FromStr for ControllerMessage {
  type Err = serde_json::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> { serde_json::from_str(s) }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ControllerMessage {
  fn to_string(&self) -> String { serde_json::to_string(self).unwrap() }
}

impl FromStr for AgentMessage {
  type Err = serde_json::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> { serde_json::from_str(s) }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for AgentMessage {
  fn to_string(&self) -> String { serde_json::to_string(self).unwrap() }
}

mod events;
mod requests;
pub use events::*;
pub use requests::*;
