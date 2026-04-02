use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

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

/// Returns the default socket path following XDG conventions.
///
/// Checks `XDG_RUNTIME_DIR` first, falls back to `/tmp/alias-{uid}/alias/alias.sock`.
fn default_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir).join("alias").join("alias.sock");
    }

    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/tmp/alias-{uid}"))
        .join("alias")
        .join("alias.sock")
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
            let socket_path = socket.unwrap_or_else(default_socket_path);
            tracing::info!(?socket_path, "starting daemon");
            eprintln!("alias-daemon start: not yet implemented (socket: {})", socket_path.display());
        }
        Commands::Stop => {
            eprintln!("alias-daemon stop: not yet implemented");
        }
        Commands::Status => {
            eprintln!("alias-daemon status: not yet implemented");
        }
    }

    Ok(())
}
