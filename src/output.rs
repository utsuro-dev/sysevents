//! Output formatting: a plain, script-friendly aligned table by default,
//! or newline-delimited-free JSON array with `--json`.

use crate::events::Event;
use std::io::{self, IsTerminal, Write};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

pub struct OutputOptions {
    pub json: bool,
    pub color: bool,
    pub utc: bool,
}

/// Prints `events` to stdout according to `opts`. Returns the number of
/// events printed, which the caller uses to pick the right exit code.
pub fn print(events: &[Event], opts: &OutputOptions) -> io::Result<usize> {
    if opts.json {
        print_json(events)?;
    } else {
        print_table(events, opts)?;
    }
    Ok(events.len())
}

fn print_json(events: &[Event]) -> io::Result<()> {
    #[derive(serde::Serialize)]
    struct JsonEvent<'a> {
        r#type: String,
        timestamp: String,
        detail: &'a Option<String>,
    }

    let payload: Vec<JsonEvent> = events
        .iter()
        .map(|e| JsonEvent {
            r#type: e.kind.to_string(),
            timestamp: e.timestamp.to_rfc3339(),
            detail: &e.detail,
        })
        .collect();

    let json = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "[]".to_string());
    println!("{json}");
    Ok(())
}

fn print_table(events: &[Event], opts: &OutputOptions) -> io::Result<()> {
    let stdout = io::stdout();
    let use_color = opts.color && stdout.is_terminal();
    let mut handle = stdout.lock();

    if events.is_empty() {
        writeln!(handle, "No boot, shutdown, suspend or resume events found for this date.")?;
        return Ok(());
    }

    for event in events {
        let timestamp = if opts.utc {
            event
                .timestamp
                .with_timezone(&chrono::Utc)
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string()
        } else {
            event.timestamp.format("%Y-%m-%d %H:%M:%S").to_string()
        };

        let kind = event.kind.to_string();
        let detail = event.detail.as_deref().unwrap_or("-");

        if use_color {
            writeln!(
                handle,
                "{color}{BOLD}{kind:<9}{RESET} {timestamp}  {detail}",
                color = event.kind.ansi_color(),
                kind = kind,
                timestamp = timestamp,
                detail = detail,
            )?;
        } else {
            writeln!(handle, "{kind:<9} {timestamp}  {detail}")?;
        }
    }

    Ok(())
}
