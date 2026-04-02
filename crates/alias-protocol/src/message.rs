use serde::{Deserialize, Serialize};

/// Requests sent from the zsh plugin to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Request {
    // TODO: implement variants
}

/// Responses sent from the daemon to the zsh plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Response {
    // TODO: implement variants
}
