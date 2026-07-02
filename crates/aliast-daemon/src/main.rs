use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use aliast_core::ai::claude::ClaudeBackend;
use aliast_core::ai::openai::OpenAiBackend;
use aliast_core::ai::{AiBackend, ollama::OllamaBackend};
use aliast_core::history::{HistoryStore, parse_history_bytes};
use aliast_daemon::{DaemonState, doctor, lifecycle, migration, server};

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

    /// Path to the Unix domain socket (defaults to the XDG/tmp runtime path).
    /// Applies to every subcommand so control commands can reach a daemon
    /// started on a non-default socket.
    #[arg(long, global = true)]
    socket: Option<PathBuf>,
}

/// Available daemon subcommands.
#[derive(Subcommand)]
enum Commands {
    /// Start the daemon, listening on the specified socket path.
    Start {
        /// Run in the foreground instead of daemonizing.
        #[arg(long)]
        foreground: bool,
    },
    /// Stop a running daemon.
    Stop,
    /// Restart the daemon (stop if running, then start).
    Restart,
    /// Check daemon status.
    Status,
    /// Enable suggestions across all shells.
    On,
    /// Disable suggestions across all shells.
    Off,
    /// Run diagnostic health checks.
    Doctor,
    /// Show the daemon log (last 50 lines).
    Logs,
    /// Import new entries from ~/.zsh_history into the suggestion database.
    Import,
    /// Show history statistics: top commands and accepted suggestions.
    Stats,
}

/// Resolves an application data directory (~/Library/Application Support/<name>
/// on macOS, ~/.local/share/<name> as fallback).
fn app_data_dir(app_name: &str) -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.data_local_dir().join(app_name))
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join(app_name)
        })
}

/// The aliast data directory (history db, daemon log).
fn data_dir() -> PathBuf {
    app_data_dir("aliast")
}

/// Initializes tracing with file-based logging.
///
/// Logs to `~/.local/share/aliast/daemon.log`.
/// Log level is controlled by `ALIAST_LOG_LEVEL` env var, defaulting to `warn`.
/// Send an NDJSON request to the daemon and read the response.
/// Uses sync UnixStream -- no tokio runtime needed.
fn send_ipc_request(socket_path: &Path, request_json: &str) -> Result<String> {
    let stream = std::os::unix::net::UnixStream::connect(socket_path)
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

/// Re-exec ourselves detached (new session, null stdio) with `--foreground`,
/// wait (bounded) until the child is serving, and return -- so the launcher
/// does not capture the user's terminal. Exits the process non-zero if the
/// daemon fails to come up.
fn daemonize(socket_path: &Path) -> Result<()> {
    let exe = std::env::current_exe()?;
    let mut command = std::process::Command::new(exe);
    command
        .arg("start")
        .arg("--foreground")
        .arg("--socket")
        .arg(socket_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(|| {
            // Detach from the controlling terminal so closing it
            // does not SIGHUP the daemon.
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command.spawn()?;

    // Wait (bounded) for the child to bind before reporting success.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if std::os::unix::net::UnixStream::connect(socket_path).is_ok() {
            println!("aliast: daemon started");
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    eprintln!("aliast: daemon did not start within 5s (check the log)");
    std::process::exit(1);
}

fn init_tracing() -> Result<()> {
    let log_dir = data_dir();

    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("daemon.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let filter =
        EnvFilter::try_from_env("ALIAST_LOG_LEVEL").unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(log_file)
        .with_ansi(false)
        .init();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Restrict every file this daemon creates (log, history db + WAL/SHM sidecars,
    // socket) to the owner. Shell history and suggestions are private data.
    #[cfg(unix)]
    unsafe {
        libc::umask(0o077);
    }

    // Migrate data files from old alias/ to new aliast/ directory (best-effort, silent)
    let _ = migration::migrate_data_files(&app_data_dir("alias"), &data_dir());

    init_tracing()?;

    let cli = Cli::parse();

    let socket_path = cli.socket.unwrap_or_else(lifecycle::default_socket_path);

    match cli.command {
        Commands::Start { foreground } => {
            // An explicit start always re-enables plugin auto-start.
            lifecycle::enable_autostart(&socket_path);

            // Default: daemonize, so `aliast start` does not capture the
            // user's terminal.
            if !foreground {
                if std::os::unix::net::UnixStream::connect(&socket_path).is_ok() {
                    eprintln!("aliast: daemon is already running");
                    std::process::exit(1);
                }
                return daemonize(&socket_path);
            }

            tracing::info!(?socket_path, "starting daemon");

            // Initialize the HistoryStore in the aliast data directory.
            let store_dir = data_dir();
            std::fs::create_dir_all(&store_dir)?;
            let db_path = store_dir.join("history.db");

            let store = HistoryStore::open(&db_path)?;
            tracing::info!(?db_path, "opened history database");

            // Auto-import zsh history when database is empty
            if store.count()? == 0 {
                let home = std::env::var("HOME").unwrap_or_default();
                let zsh_history_path = PathBuf::from(&home).join(".zsh_history");
                if zsh_history_path.exists() {
                    match std::fs::read(&zsh_history_path) {
                        Ok(bytes) => {
                            let mut entries = parse_history_bytes(&bytes);
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
                let model = std::env::var("ALIAST_NL_MODEL")
                    .ok()
                    .filter(|m| !m.is_empty());
                let backend_name =
                    std::env::var("ALIAST_NL_BACKEND").unwrap_or_else(|_| "ollama".to_string());

                match model {
                    Some(model) => match backend_name.as_str() {
                        "claude" => match std::env::var("ALIAST_ANTHROPIC_KEY") {
                            Ok(key) if !key.is_empty() => {
                                tracing::info!(model = %model, "AI backend initialized: claude");
                                Some(Arc::new(ClaudeBackend::new(key, model)))
                            }
                            _ => {
                                tracing::warn!(
                                    "ALIAST_NL_BACKEND=claude but ALIAST_ANTHROPIC_KEY not set -- NL mode disabled"
                                );
                                None
                            }
                        },
                        "openai" => match std::env::var("ALIAST_OPENAI_KEY") {
                            Ok(key) if !key.is_empty() => {
                                tracing::info!(model = %model, "AI backend initialized: openai");
                                Some(Arc::new(OpenAiBackend::new(key, model)))
                            }
                            _ => {
                                tracing::warn!(
                                    "ALIAST_NL_BACKEND=openai but ALIAST_OPENAI_KEY not set -- NL mode disabled"
                                );
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

            let (derived_backend_name, derived_model_name) = match &ai_backend {
                Some(b) => {
                    let name = b.name().to_string();
                    let model = std::env::var("ALIAST_NL_MODEL").unwrap_or_default();
                    (name, model)
                }
                None => ("none".to_string(), String::new()),
            };

            let state = DaemonState {
                store: shared_store,
                ai_backend,
                cancel_token: server_token,
                enabled: Arc::new(AtomicBool::new(true)),
                backend_name: derived_backend_name,
                model_name: derived_model_name,
            };

            // Bind synchronously so a failed or duplicate bind exits non-zero
            // here, instead of the error vanishing inside the spawned task --
            // the shutdown select! below never observes the server task, so a
            // failed bind would otherwise leave the process hanging as a silent
            // orphan (holding the DB connection) that `aliast stop` cannot reach.
            let listener = server::bind(&socket_path)?;

            let server_handle = tokio::spawn(async move {
                server::run_server_with_listener(listener, &socket_path, state).await
            });

            // Wait for shutdown signals OR cancellation from IPC shutdown command
            let mut sigterm = signal(SignalKind::terminate())?;
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Received SIGINT, shutting down");
                    cancel_token.cancel();
                }
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM, shutting down");
                    cancel_token.cancel();
                }
                _ = cancel_token.cancelled() => {
                    tracing::info!("Received IPC shutdown, shutting down");
                }
            }

            // Wait for server to finish cleanup
            if let Err(err) = server_handle.await? {
                tracing::error!("Server error during shutdown: {err}");
            }

            tracing::info!("Daemon stopped cleanly");
        }
        Commands::Stop => {
            // Record the explicit stop BEFORE shutting down, so no shell's
            // precmd/keystroke hook can respawn the daemon in the gap. Without
            // this, the plugin resurrects the daemon before the next prompt and
            // `aliast stop` is effectively a no-op in plugin-enabled shells.
            if let Err(err) = lifecycle::disable_autostart(&socket_path) {
                tracing::warn!("could not write autostart marker: {err}");
            }
            match send_ipc_request(&socket_path, r#"{"id":"stop-1","type":"shutdown"}"#) {
                Ok(response) => {
                    if response.contains("\"shutting_down\"") {
                        println!("aliast: daemon stopped (auto-start paused until `aliast start`)");
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
        Commands::Restart => {
            // Keep plugin auto-start paused during the gap (mirrors `stop`) so
            // no shell respawns the old-env daemon between shutdown and the
            // new bind. Re-enabled below once the new daemon is serving.
            if let Err(err) = lifecycle::disable_autostart(&socket_path) {
                tracing::warn!("could not write autostart marker: {err}");
            }
            // Stop any running daemon; one that is not running is fine.
            if send_ipc_request(&socket_path, r#"{"id":"restart-1","type":"shutdown"}"#).is_ok() {
                // Wait for the old daemon to release the socket so the new
                // bind cannot race it.
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while socket_path.exists() && std::time::Instant::now() < deadline {
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
                if socket_path.exists() {
                    eprintln!("aliast: old daemon did not shut down within 5s");
                    std::process::exit(1);
                }
            }
            daemonize(&socket_path)?;
            lifecycle::enable_autostart(&socket_path);
        }
        Commands::Status => {
            match send_ipc_request(&socket_path, r#"{"id":"status-1","type":"get_status"}"#) {
                Ok(response) => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response) {
                        if parsed["type"] == "status" {
                            let enabled = parsed["enabled"].as_bool().unwrap_or(true);
                            let version = parsed["version"].as_str().unwrap_or("unknown");
                            let backend = parsed["backend"].as_str().unwrap_or("unknown");
                            let model = parsed["model"].as_str().unwrap_or("");
                            let status_label = if enabled { "enabled" } else { "disabled" };
                            println!("aliast v{} is running ({})", version, status_label);
                            println!("  socket: {}", socket_path.display());
                            if backend != "none" && !model.is_empty() {
                                println!("  ai: {} ({})", backend, model);
                            } else {
                                println!("  ai: not configured");
                            }
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
        Commands::On => match send_ipc_request(&socket_path, r#"{"id":"on-1","type":"enable"}"#) {
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
        },
        Commands::Off => match send_ipc_request(&socket_path, r#"{"id":"off-1","type":"disable"}"#)
        {
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
        },
        Commands::Doctor => {
            let checks = doctor::run_doctor_checks().await;
            doctor::print_doctor_report(&checks);
            let has_failures = checks.iter().any(|c| !c.passed);
            if has_failures {
                std::process::exit(1);
            }
        }
        Commands::Logs => {
            let log_path = data_dir().join("daemon.log");
            // Tracing setup creates an empty daemon.log on every invocation,
            // so missing and empty are the same user-visible state.
            let bytes = std::fs::read(&log_path).unwrap_or_default();
            let content = String::from_utf8_lossy(&bytes);
            let all_lines: Vec<&str> = content.lines().collect();
            if all_lines.is_empty() {
                println!("aliast: no log entries yet at {}", log_path.display());
                return Ok(());
            }
            let start_index = all_lines.len().saturating_sub(50);
            println!(
                "{} (last {} of {} lines)",
                log_path.display(),
                all_lines.len() - start_index,
                all_lines.len()
            );
            for line in &all_lines[start_index..] {
                println!("{line}");
            }
        }
        Commands::Import => {
            let home = std::env::var("HOME").unwrap_or_default();
            let zsh_history_path = PathBuf::from(&home).join(".zsh_history");
            let bytes = match std::fs::read(&zsh_history_path) {
                Ok(bytes) => bytes,
                Err(err) => {
                    eprintln!("aliast: cannot read {}: {err}", zsh_history_path.display());
                    std::process::exit(1);
                }
            };
            let mut entries = parse_history_bytes(&bytes);
            // Match the first-run import: synthesize stable index-based
            // timestamps for entries without one, so re-imports dedup cleanly.
            for (index, entry) in entries.iter_mut().enumerate() {
                if entry.timestamp.is_none() {
                    entry.timestamp = Some((index + 1) as i64);
                }
            }

            let dir = data_dir();
            std::fs::create_dir_all(&dir)?;
            let store = HistoryStore::open(&dir.join("history.db"))?;
            let inserted = store.import_entries_dedup(&entries)?;
            println!(
                "aliast: imported {} new entries ({} already present)",
                inserted,
                entries.len() - inserted
            );
        }
        Commands::Stats => {
            let db_path = data_dir().join("history.db");
            if !db_path.exists() {
                println!("aliast: no history database yet (run the daemon or `aliast import`)");
                return Ok(());
            }
            let store = HistoryStore::open(&db_path)?;
            println!("History: {} recorded commands", store.count()?);
            println!();
            println!("Top commands:");
            for (command, uses) in store.top_commands(10)? {
                println!("  {:>5}x  {}", uses, command);
            }
            let accepted = store.top_accepted(5)?;
            if !accepted.is_empty() {
                println!();
                println!("Most-accepted suggestions:");
                for (command, count) in accepted {
                    println!("  {:>5}x  {}", count, command);
                }
            }
        }
    }

    Ok(())
}
