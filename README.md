# pomosport

Pomodoro TUI (ratatui) with an exercise wheel. macOS terminal.

```
cargo run -- --work 25 --short 5 --long 15   # minutes, fractions allowed
```

Keys: Space start/pause, s skip, r reset, Enter continue, q/Esc quit.

## Setup

`mise.toml` pins the Rust toolchain; run `mise install` in the repo root.

## Notifications

A work-end notification is sent via `osascript`, which always shows the Script
Editor icon. To show the pomosport icon, install terminal-notifier (optional);
it is used automatically when found on `PATH`:

```
brew install terminal-notifier
```
