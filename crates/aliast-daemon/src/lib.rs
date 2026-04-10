use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use aliast_core::ai::AiBackend;
use aliast_core::history::HistoryStore;
use tokio_util::sync::CancellationToken;

pub mod connection;
pub mod doctor;
pub mod lifecycle;
pub mod migration;
pub mod server;

/// Shared daemon state passed through the server stack.
#[derive(Clone)]
pub struct DaemonState {
    pub store: Arc<Mutex<HistoryStore>>,
    pub ai_backend: Option<Arc<dyn AiBackend>>,
    pub cancel_token: CancellationToken,
    pub enabled: Arc<AtomicBool>,
}
