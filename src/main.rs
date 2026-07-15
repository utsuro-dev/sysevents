//! `sysevents` — report BOOT / SHUTDOWN / SUSPEND / RESUME events for a
//! given date, read directly from the systemd journal.
//!
//! See `cli.rs` for the argument surface, `journal.rs` for how data is
//! extracted, and `error.rs` for the exit-code convention.

mod cli;
mod date;
mod error;
mod events;
mod journal;
mod output;
mod patterns;

use clap::Parser;
use cli::{Cli, Command};
use error::{AppError, Rendered, EXIT_FOUND, EXIT_NOT_FOUND, EXIT_USAGE};
use output::OutputOptions;

fn main() {
    let cli = Cli::parse();
    std::process::exit(run(cli));
}

fn run(cli: Cli) -> i32 {
    let Command::Show(args) = cli.command;

    let outcome = (|| -> Result<usize, AppError> {
        let source = args.resolve_date_source()?;
        let range = date::resolve(&source)?;

        journal::ensure_available()?;
        let events = events::collect(&range)?;

        let opts = OutputOptions {
            json: args.json,
            color: !args.no_color,
            utc: args.utc,
        };
        let count = output::print(&events, &opts).map_err(AppError::Io)?;
        Ok(count)
    })();

    match outcome {
        Ok(count) if count > 0 => EXIT_FOUND,
        Ok(_) => EXIT_NOT_FOUND,
        Err(err) => {
            eprintln!("{}", Rendered(&err));
            if matches!(err, AppError::UsageError(_) | AppError::InvalidDate { .. }) {
                EXIT_USAGE
            } else {
                err.exit_code()
            }
        }
    }
}
