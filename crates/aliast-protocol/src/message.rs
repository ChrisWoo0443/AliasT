use serde::{Deserialize, Serialize};

/// Requests sent from the zsh plugin to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Request {
    /// Request a completion suggestion for the current buffer.
    #[serde(rename = "complete")]
    Complete {
        /// Unique request identifier for staleness detection.
        id: String,
        /// Current command-line buffer contents.
        buf: String,
        /// Cursor position within the buffer.
        cur: u32,
        /// Current working directory for context-aware ranking.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        /// Exit code of the last command for ranking adjustment.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
        /// Current git branch for context-aware ranking.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        git_branch: Option<String>,
    },

    /// Ping the daemon to check if it is alive.
    #[serde(rename = "ping")]
    Ping {
        /// Unique request identifier.
        id: String,
    },

    /// Record a command execution from the precmd hook.
    #[serde(rename = "record")]
    Record {
        /// Unique request identifier.
        id: String,
        /// The command that was executed.
        cmd: String,
        /// Working directory where the command was run.
        cwd: String,
        /// Exit code of the recorded command.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
    },

    /// Request to generate a shell command from natural language.
    #[serde(rename = "generate")]
    Generate {
        /// Unique request identifier.
        id: String,
        /// Natural language prompt describing the desired command.
        prompt: String,
        /// Current working directory for context-aware generation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        /// Exit code of the last command for context.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
        /// Current git branch for context-aware generation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        git_branch: Option<String>,
    },

    /// Request a graceful daemon shutdown.
    #[serde(rename = "shutdown")]
    Shutdown {
        /// Unique request identifier.
        id: String,
    },

    /// Enable suggestion delivery across all connections.
    #[serde(rename = "enable")]
    Enable {
        /// Unique request identifier.
        id: String,
    },

    /// Disable suggestion delivery across all connections.
    #[serde(rename = "disable")]
    Disable {
        /// Unique request identifier.
        id: String,
    },

    /// Request current daemon status.
    #[serde(rename = "get_status")]
    GetStatus {
        /// Unique request identifier.
        id: String,
    },
}

/// Responses sent from the daemon to the zsh plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Response {
    /// A completion suggestion to display as ghost text.
    #[serde(rename = "suggestion")]
    Suggestion {
        /// Request identifier this response corresponds to.
        id: String,
        /// The suggested text to append after the cursor.
        text: String,
    },

    /// Pong response confirming the daemon is alive.
    #[serde(rename = "pong")]
    Pong {
        /// Request identifier this response corresponds to.
        id: String,
        /// Daemon version string.
        v: String,
    },

    /// Error response indicating a problem with the request.
    #[serde(rename = "error")]
    Error {
        /// Request identifier this response corresponds to.
        id: String,
        /// Human-readable error message.
        msg: String,
    },

    /// Acknowledgement response for record requests.
    #[serde(rename = "ack")]
    Ack {
        /// Request identifier this response corresponds to.
        id: String,
    },

    /// Generated shell command from natural language input.
    #[serde(rename = "command")]
    Command {
        /// Request identifier this response corresponds to.
        id: String,
        /// The generated shell command text.
        text: String,
    },

    /// Acknowledgement that the daemon is shutting down.
    #[serde(rename = "shutting_down")]
    ShuttingDown {
        /// Request identifier this response corresponds to.
        id: String,
    },

    /// Current daemon status information.
    #[serde(rename = "status")]
    Status {
        /// Request identifier this response corresponds to.
        id: String,
        /// Whether suggestions are currently enabled.
        enabled: bool,
        /// Daemon version string.
        version: String,
        /// Configured AI backend name ("ollama", "claude", "openai", or "none").
        backend: String,
        /// Configured AI model name (empty string when no backend configured).
        model: String,
    },
}
