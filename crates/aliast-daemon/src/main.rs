use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
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
use aliast_daemon::DaemonState;

mod connection;
mod lifecycle;
pub mod migration;
mod server;

/// AliasT suggestion daemon -- serves ghost-text completions over a Unix socket.
#[derive(Parser)]
#[command(name = "aliast", version, about = "AliasT suggestion daemon")]
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
    /// Enable suggestions across all shells.
    On,
    /// Disable suggestions across all shells.
    Off,
    /// Run diagnostic health checks.
    Doctor,
}

/// Initializes tracing with file-based logging.
///
/// Logs to `~/.local/share/aliast/daemon.log`.
/// Log level is controlled by `ALIAST_LOG_LEVEL` env var, defaulting to `warn`.
/// Send an NDJSON request to the daemon and read the response.
/// Uses sync UnixStream -- no tokio runtime needed.
fn send_ipc_request(request_json: &str) -> Result<String> {
    let socket_path = lifecycle::default_socket_path();
    let stream = std::os::unix::net::UnixStream::connect(&socket_path)
        .map_err(|_| anyhow::anyhow!("aliast: daemon is not running"))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;

    use std::io::{BufRead, Write};
    let mut writer = std::io::BufWriter::new(&stream);
    let mut line = request_json.to_string();
    line.push('\n');
    writer.write_all(line.as_bytes())?;
    writer.flush()?;

    let mut reader = std::io::BufReader::new(&stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;
    Ok(response_line)
}

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

            let state = DaemonState {
                store: shared_store,
                ai_backend,
                cancel_token: server_token,
                enabled: Arc::new(AtomicBool::new(true)),
            };

            let server_handle = tokio::spawn(async move {
                server::run_server(&socket_path, state).await
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
            match send_ipc_request(r#"{"id":"stop-1","type":"shutdown"}"#) {
                Ok(response) => {
                    if response.contains("\"shutting_down\"") {
                        println!("aliast: daemon stopped");
                    } else {
                        eprintln!("aliast: unexpected response from daemon");
                        std::process::exit(1);
                    }
                }
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
            }
        }
        Commands::Status => {
            let socket_path = lifecycle::default_socket_path();
            match send_ipc_request(r#"{"id":"status-1","type":"get_status"}"#) {
                Ok(response) => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response) {
                        if parsed["type"] == "status" {
                            let enabled = parsed["enabled"].as_bool().unwrap_or(true);
                            let version = parsed["version"].as_str().unwrap_or("unknown");
                            let status_label = if enabled { "enabled" } else { "disabled" };
                            println!("aliast v{} is running ({})", version, status_label);
                            println!("  socket: {}", socket_path.display());
                        } else {
                            println!("aliast is running (socket: {})", socket_path.display());
                        }
                    } else {
                        println!("aliast is running (socket: {})", socket_path.display());
                    }
                }
                Err(_) => {
                    println!("aliast is not running");
                    std::process::exit(1);
                }
            }
        }
        Commands::On => {
            match send_ipc_request(r#"{"id":"on-1","type":"enable"}"#) {
                Ok(response) => {
                    if response.contains("\"ack\"") {
                        println!("aliast: suggestions enabled");
                    } else {
                        eprintln!("aliast: unexpected response");
                        std::process::exit(1);
                    }
                }
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
            }
        }
        Commands::Off => {
            match send_ipc_request(r#"{"id":"off-1","type":"disable"}"#) {
                Ok(response) => {
                    if response.contains("\"ack\"") {
                        println!("aliast: suggestions disabled");
                    } else {
                        eprintln!("aliast: unexpected response");
                        std::process::exit(1);
                    }
                }
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
            }
        }
        Commands::Doctor => {
            eprintln!("aliast doctor: not yet implemented");
        }
    }

    Ok(())
}
