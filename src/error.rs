//! Centralised error type for `sysevents`.
//!
//! Every fallible operation in the crate returns [`AppError`]. Each variant
//! maps to a specific process exit code via [`AppError::exit_code`], so the
//! tool behaves predictably when used inside shell scripts.
//!
//! Exit code convention (grep-like, POSIX-friendly):
//!   0 -> success, at least one event was found and printed
//!   1 -> success, the command ran fine but no event matched (not an error)
//!   2 -> usage error (bad arguments, invalid/malformed date)
//!   3 -> runtime error (journalctl missing, journal unreadable, I/O failure)

use std::fmt;

/// Exit code returned when the command ran correctly and printed >=1 event.
pub const EXIT_FOUND: i32 = 0;
/// Exit code returned when the command ran correctly but found no events.
pub const EXIT_NOT_FOUND: i32 = 1;
/// Exit code returned for invalid user input (bad date, bad flags).
pub const EXIT_USAGE: i32 = 2;
/// Exit code returned for environment/runtime failures.
pub const EXIT_RUNTIME: i32 = 3;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The date string supplied by the user is not a valid `YYYY-MM-DD` date.
    #[error("invalid date '{input}': expected format YYYY-MM-DD (e.g. 2026-07-15)")]
    InvalidDate { input: String },

    /// More than one date source (positional, --date, --today, --yesterday)
    /// was supplied, or none at all.
    #[error("{0}")]
    UsageError(String),

    /// The `journalctl` binary could not be located on PATH.
    #[error("'journalctl' was not found on PATH: sysevents requires systemd")]
    JournalctlNotFound,

    /// `journalctl` ran but returned a non-zero exit status.
    #[error("journalctl failed (exit code {code}): {stderr}")]
    JournalctlFailed { code: i32, stderr: String },

    /// The journal exists but the current user lacks read permission
    /// (typically requires membership of the `systemd-journal` group).
    #[error("permission denied while reading the systemd journal: \
              add your user to the 'systemd-journal' group or run as root")]
    PermissionDenied,

    /// `journalctl -o json` produced output that could not be parsed.
    #[error("failed to parse journal output: {0}")]
    MalformedJournalOutput(String),

    /// Generic I/O failure (spawning the process, reading pipes, etc.).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl AppError {
    /// Maps the error to the process exit code that `main` should return.
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::InvalidDate { .. } | AppError::UsageError(_) => EXIT_USAGE,
            AppError::JournalctlNotFound
            | AppError::JournalctlFailed { .. }
            | AppError::PermissionDenied
            | AppError::MalformedJournalOutput(_)
            | AppError::Io(_) => EXIT_RUNTIME,
        }
    }
}

/// Small helper so call sites can write `eprintln!("{}", err.render())`
/// with a consistent `sysevents: error: ...` prefix, matching the
/// conventions of coreutils-style CLI tools.
pub struct Rendered<'a>(pub &'a AppError);

impl fmt::Display for Rendered<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sysevents: error: {}", self.0)
    }
}
