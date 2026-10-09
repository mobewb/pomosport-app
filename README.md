# pomosport

Pomodoro TUI (ratatui) with an exercise wheel. macOS terminal.

## Install

```
cargo install --git https://github.com/mobewb/pomosport-app
```

Or download a tarball (Apple silicon or Intel) from the GitHub Releases page,
extract it and put `pomosport` on your `PATH`.

Homebrew tap (optional, the tap repository itself is not part of this project).
A formula for a `homebrew-tap` repo would look like:

```ruby
class Pomosport < Formula
  desc "Pomodoro timer TUI with an exercise wheel"
  homepage "https://github.com/mobewb/pomosport-app"
  url "https://github.com/mobewb/pomosport-app/releases/download/vX.Y.Z/pomosport-vX.Y.Z-aarch64-apple-darwin.tar.gz"
  sha256 "<sha256 of the tarball>"

  def install
    bin.install "pomosport"
  end
end
```

## Usage

```
pomosport --work 25 --short 5 --long 15   # minutes, fractions allowed
pomosport stats                           # completed sessions: today, week, total
```

Options: `--cycles N` (sessions before a long break, default 4), `--exercises
"10 push-ups,20 squats"`, `--auto-start`, `--mute`, `--no-notify`.

Keys: Space start/pause, s skip, r reset, Enter continue, q/Esc/Ctrl-C quit.

## Config and history

Optional `~/.config/pomosport/config.toml`; command-line flags override it:

```toml
work = 25
short = 5
long = 15
cycles = 4
exercises = ["10 push-ups", "20 squats"]
auto_start = false
mute = false
no_notify = false
```

Each completed session is appended as a JSON line to
`~/.local/share/pomosport/history.jsonl` (used by `pomosport stats`; "today"
means the last 24 hours and "week" the last 7 days).

## Development

`mise.toml` pins the Rust toolchain; run `mise install` in the repo root. CI
(`.github/workflows/ci.yml`, macOS) uses the same pin and runs fmt, clippy and
tests. Tagging `vX.Y.Z` builds and attaches release binaries
(`.github/workflows/release.yml`).

## Notifications

A work-end notification is sent via `osascript`, which always shows the Script
Editor icon. To show the pomosport icon, install terminal-notifier (optional);
it is used automatically when found on `PATH`:

```
brew install terminal-notifier
```
