use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::signal::unix::{signal, SignalKind};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use aliast_core::ai::claude::ClaudeBackend;
use aliast_core::ai::openai::OpenAiBackend;
use aliast_core::ai::{AiBackend, ollama::OllamaBackend};
use aliast_core::history::{parse_history_file, HistoryStore};

mod connection;
mod lifecycle;
pub mod migration;
mod server;

const LONG_HELP: &str = "\
AI Setup:
  AliasT uses a local or cloud AI backend for natural-language mode (Ctrl+Space).

  Environment Variables:
    ALIAST_NL_MODEL      Model name (e.g. llama3.2, claude-sonnet-4-20250514)
    ALIAST_NL_BACKEND    Backend: ollama (default), claude, openai
    ALIAST_ANTHROPIC_KEY API key for Claude backend
    ALIAST_OPENAI_KEY    API key for OpenAI backend

  Quick Start (Ollama -- free, local):
    1. Install Ollama: brew install ollama && ollama serve
    2. Pull a model: ollama pull llama3.2
    3. Export in .zshrc:
         export ALIAST_NL_MODEL=llama3.2

  Quick Start (Claude):
    1. Get an API key from console.anthropic.com
    2. Export in .zshrc:
         export ALIAST_NL_BACKEND=claude
         export ALIAST_NL_MODEL=claude-sonnet-4-20250514
         export ALIAST_ANTHROPIC_KEY=sk-ant-...

  Run `aliast doctor` for setup diagnostics.";

/// AliasT suggestion daemon -- serves ghost-text completions over a Unix socket.
#[derive(Parser)]
#[command(
    name = "aliast",
    version,
    about = "AliasT suggestion daemon",
    after_help = "Run `aliast doctor` for setup diagnostics.",
    after_long_help = LONG_HELP,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available daemon subcommands.
#[derive(Subcommand)]
enum Commands {
    /// Start the daemon, listening on the specified socket path.
    Start {
        /// Path to the Unix domain socket.
        #[arg(long)]
        socket: Option<PathBuf>,
    },
    /// Stop a running daemon.
    Stop,
    /// Check daemon status.
    Status,
}

/// Initializes tracing with file-based logging.
///
/// Logs to `~/.local/share/aliast/daemon.log`.
/// Log level is controlled by `ALIAST_LOG_LEVEL` env var, defaulting to `warn`.
fn init_tracing() -> Result<()> {
    let log_dir = directories::BaseDirs::new()
        .map(|dirs| dirs.data_local_dir().join("aliast"))
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".local").join("share").join("aliast")
        });

    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("daemon.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let filter = EnvFilter::try_from_env("ALIAST_LOG_LEVEL")
        .unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(log_file)
        .with_ansi(false)
        .init();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Migrate data files from old alias/ to new aliast/ directory (best-effort, silent)
    let old_data_dir = directories::BaseDirs::new()
        .map(|dirs| dirs.data_local_dir().join("alias"))
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".local").join("share").join("alias")
        });
    let new_data_dir = directories::BaseDirs::new()
        .map(|dirs| dirs.data_local_dir().join("aliast"))
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".local").join("share").join("aliast")
        });
    let _ = migration::migrate_data_files(&old_data_dir, &new_data_dir);

    init_tracing()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Start { socket } => {
            let socket_path = socket.unwrap_or_else(lifecycle::default_socket_path);
            tracing::info!(?socket_path, "starting daemon");

            // Initialize HistoryStore at ~/.local/share/aliast/history.db
            let data_dir = directories::BaseDirs::new()
                .map(|dirs| dirs.data_local_dir().join("aliast"))
                .unwrap_or_else(|| {
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                    PathBuf::from(home).join(".local").join("share").join("aliast")
                });
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("history.db");

            let store = HistoryStore::open(&db_path)?;
            tracing::info!(?db_path, "opened history database");

            // Auto-import zsh history when database is empty
            if store.count()? == 0 {
                let home = std::env::var("HOME").unwrap_or_default();
                let zsh_history_path = PathBuf::from(&home).join(".zsh_history");
                if zsh_history_path.exists() {
                    match std::fs::read_to_string(&zsh_history_path) {
                        Ok(content) => {
                            let mut entries = parse_history_file(&content);
                            // Assign synthetic timestamps to entries missing them
                            for (index, entry) in entries.iter_mut().enumerate() {
                                if entry.timestamp.is_none() {
                                    entry.timestamp = Some((index + 1) as i64);
                                }
                            }
                            match store.import_entries(&entries) {
                                Ok(count) => {
                                    tracing::info!(count, "imported zsh history entries");
                                }
                                Err(err) => {
                                    tracing::error!("failed to import zsh history: {err}");
                                }
                            }
                        }
                        Err(err) => {
                            tracing::error!("failed to read zsh history file: {err}");
                        }
                    }
                } else {
                    tracing::debug!("no zsh history file found, starting with empty database");
                }
            }

            let shared_store = Arc::new(Mutex::new(store));

            // Initialize AI backend from ALIAST_NL_BACKEND + ALIAST_NL_MODEL env vars
            let ai_backend: Option<Arc<dyn AiBackend>> = {
                let model = std::env::var("ALIAST_NL_MODEL").ok().filter(|m| !m.is_empty());
                let backend_name = std::env::var("ALIAST_NL_BACKEND")
                    .unwrap_or_else(|_| "ollama".to_string());

                match model {
                    Some(model) => match backend_name.as_str() {
                        "claude" => match std::env::var("ALIAST_ANTHROPIC_KEY") {
                            Ok(key) if !key.is_empty() => {
                                tracing::info!(model = %model, "AI backend initialized: claude");
                                Some(Arc::new(ClaudeBackend::new(key, model)))
                            }
                            _ => {
                                tracing::warn!("ALIAST_NL_BACKEND=claude but ALIAST_ANTHROPIC_KEY not set -- NL mode disabled");
                                None
                            }
                        },
                        "openai" => match std::env::var("ALIAST_OPENAI_KEY") {
                            Ok(key) if !key.is_empty() => {
                                tracing::info!(model = %model, "AI backend initialized: openai");
                                Some(Arc::new(OpenAiBackend::new(key, model)))
                            }
                            _ => {
                                tracing::warn!("ALIAST_NL_BACKEND=openai but ALIAST_OPENAI_KEY not set -- NL mode disabled");
                                None
                            }
                        },
                        _ => {
                            tracing::info!(model = %model, "AI backend initialized: ollama");
                            Some(Arc::new(OllamaBackend::new(model)))
                        }
                    },
                    None => {
                        tracing::info!("No ALIAST_NL_MODEL set -- NL mode disabled");
                        None
                    }
                }
            };

            let cancel_token = CancellationToken::new();
            let server_token = cancel_token.clone();

            let server_handle = tokio::spawn(async move {
                server::run_server(&socket_path, server_token, shared_store, ai_backend).await
            });

            // Wait for shutdown signals
            let mut sigterm = signal(SignalKind::terminate())?;
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Received SIGINT, shutting down");
                }
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM, shutting down");
                }
            }

            cancel_token.cancel();

            // Wait for server to finish cleanup
            if let Err(err) = server_handle.await? {
                tracing::error!("Server error during shutdown: {err}");
            }

            tracing::info!("Daemon stopped cleanly");
        }
        Commands::Stop => {
            eprintln!("aliast-daemon stop: not yet implemented");
        }
        Commands::Status => {
            let socket_path = lifecycle::default_socket_path();
            match std::os::unix::net::UnixStream::connect(&socket_path) {
                Ok(_) => {
                    println!("aliast-daemon is running (socket: {})", socket_path.display());
                }
                Err(_) => {
                    println!("aliast-daemon is not running");
                }
            }
        }
    }

    Ok(())
}
