//! Diagnostic health checks for `aliast doctor`.
//!
//! Each check is independent and reports pass/fail with an actionable fix.
//! Doctor does NOT require a running daemon -- it probes the socket, reads
//! env vars, and tests external services directly.

use std::path::{Path, PathBuf};
use std::time::Duration;

use aliast_core::ai::AiBackend;

use crate::lifecycle;

/// Result of a single diagnostic check.
pub struct DoctorCheck {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
    pub fix: Option<String>,
}

/// Run all diagnostic checks and return results.
/// This function is async because checks 4-5 make HTTP calls.
pub async fn run_doctor_checks() -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    checks.push(check_daemon_running());
    checks.push(check_ai_backend_configured());
    checks.push(check_api_key_present());
    checks.push(check_ollama_reachable().await);
    checks.push(check_api_key_valid().await);
    checks.push(check_history_db());
    checks
}

/// Print a human-readable doctor report to stdout.
pub fn print_doctor_report(checks: &[DoctorCheck]) {
    println!("AliasT Doctor");
    println!("{}", "=".repeat(50));
    println!();

    for check in checks {
        let status = if check.passed { "[ok]" } else { "[!!]" };
        println!("{} {}", status, check.name);
        println!("     {}", check.detail);
        if let Some(ref fix) = check.fix {
            println!("     Fix: {}", fix);
        }
        println!();
    }

    let passed_count = checks.iter().filter(|c| c.passed).count();
    let total_count = checks.len();
    println!("{}/{} checks passed", passed_count, total_count);

    if passed_count < total_count {
        println!();
        println!("Note: Doctor reads env vars from the current shell session.");
        println!("The running daemon may have different values if it was started");
        println!("in a different shell or with different environment variables.");
    }
}

/// Check 1: Daemon running -- probe socket with sync UnixStream.
pub fn check_daemon_running() -> DoctorCheck {
    let socket_path = lifecycle::default_socket_path();
    check_daemon_running_at(&socket_path)
}

/// Testable variant of check_daemon_running that accepts a socket path.
pub fn check_daemon_running_at(socket_path: &Path) -> DoctorCheck {
    match std::os::unix::net::UnixStream::connect(socket_path) {
        Ok(_) => DoctorCheck {
            name: "Daemon running",
            passed: true,
            detail: format!("Connected to {}", socket_path.display()),
            fix: None,
        },
        Err(_) => DoctorCheck {
            name: "Daemon running",
            passed: false,
            detail: format!("Cannot connect to {}", socket_path.display()),
            fix: Some("Run `aliast start` to start the daemon".to_string()),
        },
    }
}

/// Check 2: AI backend configured -- read ALIAST_NL_BACKEND and ALIAST_NL_MODEL.
pub fn check_ai_backend_configured() -> DoctorCheck {
    let backend = std::env::var("ALIAST_NL_BACKEND").unwrap_or_default();
    let model = std::env::var("ALIAST_NL_MODEL").ok().filter(|m| !m.is_empty());

    match model {
        Some(model_name) => {
            let backend_display = if backend.is_empty() {
                "ollama (default)"
            } else {
                &backend
            };
            DoctorCheck {
                name: "AI backend configured",
                passed: true,
                detail: format!("Backend: {}, Model: {}", backend_display, model_name),
                fix: None,
            }
        }
        None => DoctorCheck {
            name: "AI backend configured",
            passed: false,
            detail: "No AI model configured -- NL mode will be disabled".to_string(),
            fix: Some(
                "Set ALIAST_NL_MODEL (e.g., export ALIAST_NL_MODEL=llama3.2)".to_string(),
            ),
        },
    }
}

/// Check 3: API key present -- check relevant key based on backend.
pub fn check_api_key_present() -> DoctorCheck {
    let backend = std::env::var("ALIAST_NL_BACKEND")
        .unwrap_or_else(|_| "ollama".to_string());

    match backend.as_str() {
        "ollama" | "" => DoctorCheck {
            name: "API key present",
            passed: true,
            detail: "Ollama backend -- no API key needed".to_string(),
            fix: None,
        },
        "claude" => {
            let has_key = std::env::var("ALIAST_ANTHROPIC_KEY")
                .ok()
                .filter(|k| !k.is_empty())
                .is_some();
            if has_key {
                DoctorCheck {
                    name: "API key present",
                    passed: true,
                    detail: "ALIAST_ANTHROPIC_KEY is set".to_string(),
                    fix: None,
                }
            } else {
                DoctorCheck {
                    name: "API key present",
                    passed: false,
                    detail: "ALIAST_ANTHROPIC_KEY not set".to_string(),
                    fix: Some(
                        "Set ALIAST_ANTHROPIC_KEY with your Anthropic API key".to_string(),
                    ),
                }
            }
        }
        "openai" => {
            let has_key = std::env::var("ALIAST_OPENAI_KEY")
                .ok()
                .filter(|k| !k.is_empty())
                .is_some();
            if has_key {
                DoctorCheck {
                    name: "API key present",
                    passed: true,
                    detail: "ALIAST_OPENAI_KEY is set".to_string(),
                    fix: None,
                }
            } else {
                DoctorCheck {
                    name: "API key present",
                    passed: false,
                    detail: "ALIAST_OPENAI_KEY not set".to_string(),
                    fix: Some("Set ALIAST_OPENAI_KEY with your OpenAI API key".to_string()),
                }
            }
        }
        other => DoctorCheck {
            name: "API key present",
            passed: false,
            detail: format!("Unknown backend: {}", other),
            fix: Some(
                "Set ALIAST_NL_BACKEND to ollama, claude, or openai".to_string(),
            ),
        },
    }
}

/// Check 4: Ollama reachable -- HTTP GET to localhost:11434.
async fn check_ollama_reachable() -> DoctorCheck {
    let backend = std::env::var("ALIAST_NL_BACKEND")
        .unwrap_or_else(|_| "ollama".to_string());

    if backend != "ollama" && !backend.is_empty() {
        return DoctorCheck {
            name: "Ollama reachable",
            passed: true,
            detail: format!("Skipped -- using {} backend", backend),
            fix: None,
        };
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    match client.get("http://localhost:11434/").send().await {
        Ok(response) if response.status().is_success() => DoctorCheck {
            name: "Ollama reachable",
            passed: true,
            detail: "Ollama is running at localhost:11434".to_string(),
            fix: None,
        },
        _ => DoctorCheck {
            name: "Ollama reachable",
            passed: false,
            detail: "Cannot reach Ollama at localhost:11434".to_string(),
            fix: Some("Start Ollama with: ollama serve".to_string()),
        },
    }
}

/// Check 5: API key valid -- lightweight test call to backend.
async fn check_api_key_valid() -> DoctorCheck {
    let backend = std::env::var("ALIAST_NL_BACKEND")
        .unwrap_or_else(|_| "ollama".to_string());
    let model = std::env::var("ALIAST_NL_MODEL").unwrap_or_default();

    match backend.as_str() {
        "claude" => {
            let api_key = match std::env::var("ALIAST_ANTHROPIC_KEY") {
                Ok(key) if !key.is_empty() => key,
                _ => {
                    return DoctorCheck {
                        name: "API key valid",
                        passed: false,
                        detail: "Cannot validate -- no API key set".to_string(),
                        fix: Some("Set ALIAST_ANTHROPIC_KEY first".to_string()),
                    };
                }
            };

            let claude_backend =
                aliast_core::ai::claude::ClaudeBackend::new(api_key, model);
            match claude_backend.health_check().await {
                Ok(()) => DoctorCheck {
                    name: "API key valid",
                    passed: true,
                    detail: "Claude API key is valid".to_string(),
                    fix: None,
                },
                Err(err) => DoctorCheck {
                    name: "API key valid",
                    passed: false,
                    detail: format!("Claude API check failed: {}", err),
                    fix: Some(
                        "Verify your ALIAST_ANTHROPIC_KEY is correct and has credits"
                            .to_string(),
                    ),
                },
            }
        }
        "openai" => {
            let api_key = match std::env::var("ALIAST_OPENAI_KEY") {
                Ok(key) if !key.is_empty() => key,
                _ => {
                    return DoctorCheck {
                        name: "API key valid",
                        passed: false,
                        detail: "Cannot validate -- no API key set".to_string(),
                        fix: Some("Set ALIAST_OPENAI_KEY first".to_string()),
                    };
                }
            };

            let openai_backend =
                aliast_core::ai::openai::OpenAiBackend::new(api_key, model);
            match openai_backend.health_check().await {
                Ok(()) => DoctorCheck {
                    name: "API key valid",
                    passed: true,
                    detail: "OpenAI API key is valid".to_string(),
                    fix: None,
                },
                Err(err) => DoctorCheck {
                    name: "API key valid",
                    passed: false,
                    detail: format!("OpenAI API check failed: {}", err),
                    fix: Some(
                        "Verify your ALIAST_OPENAI_KEY is correct and has credits".to_string(),
                    ),
                },
            }
        }
        _ => {
            // Ollama or unset -- skip API key validation
            DoctorCheck {
                name: "API key valid",
                passed: true,
                detail: "Skipped -- Ollama uses no API key".to_string(),
                fix: None,
            }
        }
    }
}

/// Check 6: History database -- check file exists and is readable.
pub fn check_history_db() -> DoctorCheck {
    let data_dir = directories::BaseDirs::new()
        .map(|dirs| dirs.data_local_dir().join("aliast"))
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("aliast")
        });
    let db_path = data_dir.join("history.db");
    check_history_db_at(&db_path)
}

/// Testable variant of check_history_db that accepts a path parameter.
pub fn check_history_db_at(db_path: &Path) -> DoctorCheck {
    if !db_path.exists() {
        return DoctorCheck {
            name: "History database",
            passed: true,
            detail: format!(
                "Database at {} does not exist yet -- will be created on first daemon start",
                db_path.display()
            ),
            fix: None,
        };
    }

    match std::fs::metadata(db_path) {
        Ok(metadata) => {
            if metadata.len() == 0 {
                DoctorCheck {
                    name: "History database",
                    passed: true,
                    detail: format!("Database at {} is empty -- will be populated on use", db_path.display()),
                    fix: None,
                }
            } else {
                DoctorCheck {
                    name: "History database",
                    passed: true,
                    detail: format!(
                        "Database at {} ({} bytes)",
                        db_path.display(),
                        metadata.len()
                    ),
                    fix: None,
                }
            }
        }
        Err(err) => DoctorCheck {
            name: "History database",
            passed: false,
            detail: format!("Cannot read {}: {}", db_path.display(), err),
            fix: Some(format!(
                "Check file permissions on {}",
                db_path.display()
            )),
        },
    }
}
