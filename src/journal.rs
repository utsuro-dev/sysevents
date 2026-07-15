//! Thin, defensive wrapper around the `journalctl` binary.
//!
//! Design rationale: rather than linking against `libsystemd` (which would
//! pull in `bindgen`/`clang` as build-time dependencies and complicate AUR
//! packaging), `sysevents` shells out to `journalctl`, which is guaranteed
//! to exist on every systemd system and is the same stable interface used
//! by `libsystemd`'s own `sd_journal_*` calls internally. Output is parsed
//! as newline-delimited JSON (`-o json`), never as human-formatted text,
//! which keeps parsing exact and locale-independent.
//!
//! Performance note: boot start/end timestamps are obtained by reading a
//! single JSON record per boot (the child process is killed immediately
//! after), so cost scales with the number of boots on the system, not with
//! the size of the journal itself.

use crate::error::AppError;
use chrono::{DateTime, Local, TimeZone};
use serde::Deserialize;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};

const JOURNALCTL: &str = "journalctl";

/// One decoded journal record, extracting only the fields sysevents needs.
#[derive(Debug, Deserialize)]
struct JournalRecord {
    #[serde(rename = "__REALTIME_TIMESTAMP")]
    realtime_timestamp: String,
    #[serde(rename = "MESSAGE")]
    message: Option<serde_json::Value>,
}

impl JournalRecord {
    /// `MESSAGE` is usually a string, but journald emits a byte array for
    /// non-UTF-8 payloads; we degrade gracefully in that rare case.
    fn message_text(&self) -> Option<String> {
        match &self.message {
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            Some(serde_json::Value::Array(_)) => Some("<binary message>".to_string()),
            _ => None,
        }
    }

    fn timestamp(&self) -> Result<DateTime<Local>, AppError> {
        let micros: i64 = self
            .realtime_timestamp
            .parse()
            .map_err(|_| AppError::MalformedJournalOutput(format!(
                "invalid __REALTIME_TIMESTAMP value: {}",
                self.realtime_timestamp
            )))?;
        Local
            .timestamp_micros(micros)
            .single()
            .ok_or_else(|| AppError::MalformedJournalOutput(format!(
                "out-of-range timestamp: {micros}"
            )))
    }
}

/// Verifies that `journalctl` exists on PATH before doing any real work,
/// producing a clear, actionable error otherwise.
pub fn ensure_available() -> Result<(), AppError> {
    match Command::new(JOURNALCTL).arg("--version").output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(classify_failure(out.status.code(), &out.stderr)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(AppError::JournalctlNotFound),
        Err(e) => Err(AppError::Io(e)),
    }
}

/// Returns the boot ID of the currently running kernel, normalised (no
/// dashes, lowercase) so it can be compared directly against the boot IDs
/// reported by `journalctl --list-boots`.
pub fn current_boot_id() -> Result<String, AppError> {
    let raw = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")?;
    Ok(normalize_boot_id(raw.trim()))
}

fn normalize_boot_id(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_lowercase()
}

/// Lists every boot ID known to the journal, oldest first.
pub fn list_boot_ids() -> Result<Vec<String>, AppError> {
    let output = Command::new(JOURNALCTL)
        .args(["--list-boots", "--no-pager", "-q"])
        .output()?;

    if !output.status.success() {
        return Err(classify_failure(output.status.code(), &output.stderr));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut ids = Vec::new();
    for line in text.lines() {
        // Layout: "<index> <boot_id> <first> ... <last> ..."
        // Index and boot_id are plain ASCII tokens, so plain whitespace
        // splitting is safe regardless of locale/timezone naming.
        if let Some(id) = line.split_whitespace().nth(1) {
            ids.push(normalize_boot_id(id));
        }
    }
    Ok(ids)
}

/// Reads a single boundary timestamp (first or last entry) for `boot_id`.
/// Returns `Ok(None)` if the boot has no readable entries (e.g. a boot
/// whose logs were rotated away, or a running boot with no data yet).
pub fn boot_boundary_timestamp(
    boot_id: &str,
    reverse: bool,
) -> Result<Option<(DateTime<Local>, Option<String>)>, AppError> {
    let mut cmd = Command::new(JOURNALCTL);
    cmd.args(["-b", boot_id, "-o", "json", "--no-pager"]);
    if reverse {
        cmd.arg("--reverse");
    }
    read_first_json_record(cmd)
}

/// Runs `journalctl --since S --until U --grep PATTERN -o json` and
/// returns every matching record as `(timestamp, message)`, oldest first.
pub fn grep_range(
    pattern: &str,
    since: &str,
    until: &str,
) -> Result<Vec<(DateTime<Local>, Option<String>)>, AppError> {
    let output = Command::new(JOURNALCTL)
        .args([
            "-o", "json",
            "--no-pager",
            "--since", since,
            "--until", until,
            "--grep", pattern,
        ])
        .output()?;

    // journalctl follows a grep-like convention when --grep is used: it
    // exits with status 1 (and no output) when zero entries match, rather
    // than exiting 0 with an empty result set. That is not an error case
    // for sysevents, so it is handled before the generic failure path.
    if output.status.code() == Some(1) && output.stdout.is_empty() {
        return Ok(Vec::new());
    }

    if !output.status.success() {
        return Err(classify_failure(output.status.code(), &output.stderr));
    }

    let mut events = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.trim().is_empty() {
            continue;
        }
        let record: JournalRecord = serde_json::from_str(line)
            .map_err(|e| AppError::MalformedJournalOutput(e.to_string()))?;
        events.push((record.timestamp()?, record.message_text()));
    }
    Ok(events)
}

/// Spawns `cmd`, reads exactly one JSON line from its stdout, then
/// terminates the child. This keeps boot-boundary lookups O(1) instead of
/// O(journal size), even on multi-gigabyte journals.
fn read_first_json_record(
    mut cmd: Command,
) -> Result<Option<(DateTime<Local>, Option<String>)>, AppError> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn()?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let bytes_read = reader.read_line(&mut line)?;

    // We only wanted one line; drop the rest without waiting for the
    // child to finish producing (potentially large) further output.
    let _ = child.kill();
    let _ = child.wait();

    if bytes_read == 0 {
        return Ok(None);
    }

    let record: JournalRecord = serde_json::from_str(line.trim_end())
        .map_err(|e| AppError::MalformedJournalOutput(e.to_string()))?;
    Ok(Some((record.timestamp()?, record.message_text())))
}

/// Turns a non-zero journalctl exit status plus captured stderr into a
/// specific, actionable [`AppError`].
fn classify_failure(code: Option<i32>, stderr: &[u8]) -> AppError {
    let mut text = String::new();
    let _ = { &stderr }.take(4096).read_to_string(&mut text);
    let text = if text.is_empty() {
        String::from_utf8_lossy(stderr).to_string()
    } else {
        text
    };

    if text.to_lowercase().contains("permission denied") {
        AppError::PermissionDenied
    } else {
        AppError::JournalctlFailed {
            code: code.unwrap_or(-1),
            stderr: text.trim().to_string(),
        }
    }
}
