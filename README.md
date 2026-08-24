# Eye Break

A tray-icon app that reminds you to take periodic eye breaks, following the
20-20-20 rule (every 20 minutes, look at something 20 feet away for 20
seconds) — or a Pomodoro-style work/break cycle, if you prefer that instead.
Each break shows a short full-screen overlay with a guided exercise, on
every connected monitor.

- Configurable reminder interval, break duration, and snooze length
- Guided eye exercises that rotate each break
- Optional Pomodoro mode (work / short break / long break cycles)
- A small always-on-top corner countdown to the next break, with adjustable
  opacity
- Workday-hours scheduling (only remind during certain hours/days)
- Smart pausing while idle or while a fullscreen app (calls, video, slides)
  is active
- Runs on Linux and macOS

## Installation

### Linux — `.deb` package (recommended)

Download the latest `eye-break_<version>-1_amd64.deb` from the
[Releases page](https://github.com/noorhaq/eye_break/releases), then:

```sh
sudo dpkg -i eye-break_<version>-1_amd64.deb
# If it reports missing dependencies:
sudo apt-get install -f
```

This installs the binary to `/usr/bin/eye-break`, a `.desktop` launcher, an
autostart entry (so it starts on login), and app icons. Launch it from your
application menu, or run `eye-break` from a terminal.

To upgrade later, just `dpkg -i` a newer `.deb` over the old one. To
uninstall: `sudo dpkg -r eye-break`.

### macOS

Download `eye-break-<version>-macos-universal.zip` from the
[Releases page](https://github.com/noorhaq/eye_break/releases), unzip it,
and run the `eye-break` binary. There's no installer/autostart entry set up
automatically on macOS yet — use Settings → "Run on startup" inside the app
once it's running, or the `eye-break` CLI (see below).

### Building from source

Requires a recent [Rust toolchain](https://rustup.rs/). On Linux you'll also
need GTK3 and X11 development headers:

```sh
# Debian/Ubuntu
sudo apt-get install libgtk-3-dev libx11-dev libxdo-dev
```

Then build and run:

```sh
cargo build --release
./target/release/eye-break
```

To build a `.deb` package yourself (uses the `[package.metadata.deb]`
section in `Cargo.toml`):

```sh
cargo install cargo-deb   # once
cargo deb                 # produces target/debian/eye-break_<version>-1_amd64.deb
```

## Usage

Running `eye-break` with no arguments starts the tray icon and scheduler —
this is the normal way to run it day-to-day. Right-click (or click, on
macOS) the tray icon for reminder interval, break duration, snooze length,
and a link to the full Settings window (theme, Pomodoro, workday schedule,
sound, and usage stats).

It also has a small CLI, handy for scripting or for toggling things over
SSH without a GUI:

```
eye-break                    Run the tray icon + scheduler (default)
eye-break enable             Enable reminders
eye-break disable             Disable reminders
eye-break toggle             Toggle reminders on/off
eye-break status              Show current settings
eye-break interval <secs>    Set break interval
eye-break duration <secs>    Set overlay display duration
eye-break snooze <secs>      Set the Skip snooze length
eye-break skip                Push the next break out by the snooze length
eye-break --settings          Open the full Settings window directly
```

Only one instance of the tray/scheduler is ever allowed to run at a time —
launching a second one (e.g. via autostart *and* a manual launch) prints a
message and exits instead of racing the first.

### Configuration files

Settings and scheduling state live under your OS's standard config
directory, as JSON:

- Linux: `~/.config/eye-break/`
- macOS: `~/Library/Application Support/dev.eye-break.eye-break/`

The main files are `config.json` (everything in Settings), `state.json`
(next-break scheduling), and `pomodoro_state.json` (Pomodoro phase, when
that mode is enabled). These are safe to delete if you want to reset
everything back to defaults.

## Development

```sh
cargo build            # debug build
cargo test              # unit tests
cargo build --release   # optimized build, what gets packaged
```

`src/main.rs` is the entry point and CLI dispatch; each subcommand/window
(`--overlay`, `--timer`, `--settings`) runs as its own short-lived process,
spawned by the tray/scheduler in `src/tray.rs`, so they don't have to share
an event loop with it.
