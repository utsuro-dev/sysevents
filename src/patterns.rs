//! Detection patterns for events that have no single, stable systemd
//! message-catalog ID across kernel/systemd versions (unlike boot and
//! shutdown, which are derived structurally from `journalctl --list-boots`
//! and are therefore always accurate).
//!
//! Suspend/resume is logged by `systemd-logind` and by the kernel's power
//! management subsystem, and the exact wording has changed over the years
//! and varies slightly between suspend-to-RAM and suspend-to-idle (s2idle).
//! To keep detection robust without depending on a specific systemd
//! release, we match against a small set of extended regular expressions
//! (passed to `journalctl --grep`, which uses PCRE-style syntax) and take
//! the first match per event.
//!
//! This module is the single, documented place to extend or tune matching:
//! if your distribution/kernel logs suspend/resume differently, add a
//! pattern here rather than touching the query logic in `journal.rs`.

/// Patterns identifying a system entering suspend (or hibernate).
pub const SUSPEND_PATTERNS: &[&str] = &[
    r"PM: suspend entry",
    r"PM: hibernation entry",
    r"Suspending system",
    r"Reached target Sleep",
    r"Starting Suspend",
];

/// Patterns identifying a system resuming from suspend (or hibernate).
pub const RESUME_PATTERNS: &[&str] = &[
    r"PM: suspend exit",
    r"PM: hibernation exit",
    r"System resumed",
    r"Stopped target Sleep",
    r"ACPI: Waking up from system sleep state",
];

/// Builds a single alternation regex (`a|b|c`) suitable for
/// `journalctl --grep`, from a pattern list.
pub fn alternation(patterns: &[&str]) -> String {
    patterns.join("|")
}
