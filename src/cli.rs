//! Command-line interface definition.
//!
//! `sysevents` exposes a single dedicated subcommand, `show`, which keeps
//! the surface area intentionally small: this tool does one thing (report
//! boot/shutdown/suspend/resume events for a day) and does it well, in the
//! spirit of the Unix philosophy.

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "sysevents",
    version,
    about = "Show boot, shutdown, suspend and resume events for a given date",
    long_about = "sysevents reads the systemd journal directly and reports \
                   power-state transitions (BOOT, SHUTDOWN, SUSPEND, RESUME) \
                   that occurred on a given calendar day.",
    propagate_version = true,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Show power-state events (boot/shutdown/suspend/resume) for a date
    Show(ShowArgs),
}

#[derive(Args, Debug)]
pub struct ShowArgs {
    /// Date to inspect, in YYYY-MM-DD format (e.g. 2026-07-15)
    #[arg(value_name = "DATE")]
    pub date: Option<String>,

    /// Explicit date, alternative to the positional argument
    #[arg(long = "date", value_name = "DATE", conflicts_with = "date")]
    pub date_flag: Option<String>,

    /// Use today's date
    #[arg(long, conflicts_with_all = ["date", "date_flag", "yesterday"])]
    pub today: bool,

    /// Use yesterday's date
    #[arg(long, conflicts_with_all = ["date", "date_flag", "today"])]
    pub yesterday: bool,

    /// Emit machine-readable JSON instead of the human-readable table
    #[arg(long)]
    pub json: bool,

    /// Disable ANSI colour, even on a TTY
    #[arg(long)]
    pub no_color: bool,

    /// Local system timezone name is used implicitly; this flag prints
    /// timestamps in UTC instead of local time
    #[arg(long)]
    pub utc: bool,
}

impl ShowArgs {
    /// Resolves the three mutually exclusive date sources into one
    /// canonical `Option<String>` date argument, or a usage error if the
    /// user supplied none or more than one (clap's `conflicts_with`
    /// already prevents "more than one", this only handles "none").
    pub fn resolve_date_source(&self) -> Result<DateSource, crate::error::AppError> {
        if self.today {
            Ok(DateSource::Today)
        } else if self.yesterday {
            Ok(DateSource::Yesterday)
        } else if let Some(d) = self.date_flag.clone().or_else(|| self.date.clone()) {
            Ok(DateSource::Explicit(d))
        } else {
            Err(crate::error::AppError::UsageError(
                "no date specified: pass a DATE argument, --date <DATE>, --today or --yesterday"
                    .to_string(),
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub enum DateSource {
    Explicit(String),
    Today,
    Yesterday,
}
