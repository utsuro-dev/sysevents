# sysevents

Show **BOOT**, **SHUTDOWN**, **SUSPEND** and **RESUME** events for a given
date, read directly from the systemd journal.

`sysevents` is a small, dependency-light CLI for Arch Linux (and any
systemd-based distribution) that answers one question precisely: *what
power-state transitions happened on my machine on day X?*

```
$ sysevents show 2026-07-15
BOOT      2026-07-15 08:11:34  boot id 1a2b3c4d5e6f7890abcdef1234567890
SUSPEND   2026-07-15 13:22:18  PM: suspend entry (s2idle)
RESUME    2026-07-15 14:05:11  PM: suspend exit
SHUTDOWN  2026-07-15 22:47:59  boot id 1a2b3c4d5e6f7890abcdef1234567890
```

## Usage

```
sysevents show <DATE>
sysevents show --date <DATE>
sysevents show --today
sysevents show --yesterday

Options:
      --json         Emit machine-readable JSON instead of a table
      --no-color     Disable ANSI colour, even on a TTY
      --utc          Print timestamps in UTC instead of local time
  -h, --help         Print help
  -V, --version      Print version
```

`<DATE>` must be in strict `YYYY-MM-DD` form (e.g. `2026-07-15`).

## Exit codes

`sysevents` follows a `grep`-like convention so it composes cleanly in
scripts:

| Code | Meaning                                                        |
|------|-----------------------------------------------------------------|
| 0    | Success — at least one matching event was found and printed     |
| 1    | Success — the command ran fine, but no event matched            |
| 2    | Usage error — invalid date, conflicting or missing arguments    |
| 3    | Runtime error — `journalctl` missing, unreadable journal, I/O   |

## How it works

- **BOOT** and **SHUTDOWN** are derived structurally from
  `journalctl --list-boots`: the first journal entry of each boot is the
  boot time; the last journal entry of each *past* boot (i.e. not the
  currently running one) is used as a shutdown-time proxy. This is exact
  for boot times and a reliable approximation for shutdowns (a hard
  crash or power loss will simply have no later "shutdown" entry, which
  is the correct, honest behaviour).
- **SUSPEND** and **RESUME** are detected via `journalctl --grep`
  against a small, documented set of kernel/`systemd-logind` message
  patterns (see `src/patterns.rs`), since no single stable message
  catalog ID exists across kernel/systemd versions for these transitions.
- All journal access goes through the standard `journalctl` binary — not
  `libsystemd` bindings — so the package has zero build-time system
  library dependencies beyond a Rust toolchain.
- Boot boundary lookups read a single journal record per boot and then
  terminate the `journalctl` child process, so cost scales with the
  **number of boots**, not the size of the journal — the tool stays fast
  even on multi-gigabyte journals.

## Requirements

- Arch Linux (or any systemd distribution) with `journalctl` on `PATH`.
- Read access to the systemd journal (membership of the
  `systemd-journal` group, or root).

## Building from source

```
cargo build --release
install -Dm755 target/release/sysevents /usr/local/bin/sysevents
```

## License

MIT — see `LICENSE`.
