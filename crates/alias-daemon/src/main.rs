use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::signal::unix::{signal, SignalKind};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

mod connection;
mod lifecycle;
mod server;

/// Alias suggestion daemon -- serves ghost-text completions over a Unix socket.
#[derive(Parser)]
#[command(name = "alias-daemon", version, about = "Alias suggestion daemon")]
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
/// Logs to `~/.local/share/alias/daemon.log`.
/// Log level is controlled by `ALIAS_LOG_LEVEL` env var, defaulting to `warn`.
fn init_tracing() -> Result<()> {
    let log_dir = directories::BaseDirs::new()
        .map(|dirs| dirs.data_local_dir().join("alias"))
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".local").join("share").join("alias")
        });

    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("daemon.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let filter = EnvFilter::try_from_env("ALIAS_LOG_LEVEL")
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
    init_tracing()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Start { socket } => {
            let socket_path = socket.unwrap_or_else(lifecycle::default_socket_path);
            tracing::info!(?socket_path, "starting daemon");

            let cancel_token = CancellationToken::new();
            let server_token = cancel_token.clone();

            let server_handle = tokio::spawn(async move {
                server::run_server(&socket_path, server_token).await
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
            eprintln!("alias-daemon stop: not yet implemented");
        }
        Commands::Status => {
            let socket_path = lifecycle::default_socket_path();
            match std::os::unix::net::UnixStream::connect(&socket_path) {
                Ok(_) => {
                    println!("alias-daemon is running (socket: {})", socket_path.display());
                }
                Err(_) => {
                    println!("alias-daemon is not running");
                }
            }
        }
    }

    Ok(())
}
