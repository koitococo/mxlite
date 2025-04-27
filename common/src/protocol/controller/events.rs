use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ControllerEvent {
  pub id: u64,
  pub event: String,
  pub data: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentEvent {
  pub id: u64,
  pub event: String,
  pub data: String,
}
