# Changelog

All notable changes to Eye Break are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- A manual "Pause" option (tray menu, and `eye-break pause [mins]` /
  `eye-break resume` over SSH) for calls, presentations, and anything
  else where reminders shouldn't interrupt — distinct from unchecking
  "Enabled" (a deliberate, easy-to-forget-to-flip-back-on off switch) and
  from "Skip" (a one-shot push-out of just the *next* break, which a call
  running long can outlast). Choose 15/30/60/120 minutes or "Until I
  resume"; the tray's "Resume now" item shows time remaining and ends the
  pause early. Like idle detection, returning from a pause (however it
  ends) resets both schedulers' baselines and clears any pending Skip
  snooze, so the first break afterward is a full interval away rather than
  ambushing whoever just got off their call.
- `README.md` with installation (`.deb`, macOS zip, from-source), CLI, and
  config-file-location documentation.
- Licensed under GPL-3.0-or-later.
- Regression tests for the Pomodoro corner-timer math (`phase_progress`,
  phase-cycle ordering).

### Fixed
- VS Code (and presumably other apps sensitive to it) could become totally
  unresponsive to clicks and typing — not just during a break, but for the
  rest of the session — until eye-break itself was killed. The corner
  countdown, which runs continuously for as long as eye-break does, had
  picked up the *previous* fix's "hand focus back to whatever the overlay
  interrupted" mechanism too, but that mechanism has no natural end for a
  process that never stops running: it captured whatever was focused the
  moment the countdown started (e.g. at login) and then fought to force
  focus back onto that one specific window every 400ms, forever — including
  fighting every later click into anything else, indefinitely, not just
  around a single break. The break overlay itself keeps this behavior
  (correct there — it's bounded to a few seconds and the fight ends the
  moment it closes); the corner countdown no longer does.
- The break overlay no longer takes over the mouse and keyboard for its
  whole 5-30s duration. It's a normal top-level window covering the entire
  monitor, so previously it silently absorbed every click anywhere on
  screen — not just clicks on its own "OK"/"Skip" buttons — meaning you
  couldn't click back into whatever you were doing (an editor, a chat)
  until it closed. It's now click-through everywhere except directly over
  its own buttons, toggled dynamically as the cursor moves (real mouse
  position is polled independently of egui's own pointer tracking, since
  that goes blind the instant click-through engages — a chicken-and-egg
  problem inherent to this kind of overlay). Also stopped it from
  potentially stealing keyboard focus away from whatever you were typing
  into the moment it appeared: the window manager doing that on window-map
  is a WM-specific new/mapped-window heuristic, not something a window
  fully controls itself, so instead the app now explicitly hands focus
  back to whichever window had it right before the break was triggered,
  each time the overlay's on-top guard loop reasserts (using a focus
  primitive that doesn't also re-raise its target over the overlay).
- The `.deb` no longer installs an unlaunchable binary. Its `Depends` was
  hand-written in the release workflow and listed only `libgtk-3-0` and
  `libx11-6`, omitting `libxdo3` (linked by `tray-icon`); on a machine
  without `xdotool` already installed the package installed cleanly and
  then died at startup with `libxdo.so.3: cannot open shared object file`.
  Dependencies are now derived from the built binary with
  `dpkg-shlibdeps`. Existing 0.5.0 installs can be fixed with
  `sudo apt install libxdo3`.
- Corner-timer opacity (and other Settings changes) no longer silently
  reverts. The tray/scheduler process held its `Config` in memory from
  startup and re-saved that stale copy on every break or menu toggle,
  overwriting whatever the separate Settings process had just written to
  disk — it now reloads from disk before any local change.
- The corner countdown now tracks the actual Pomodoro phase (Focus Time /
  Short Break / Long Break) when Pomodoro mode is enabled, instead of
  counting down against the unrelated plain-interval schedule — previously
  it would hit 00:00 with no break shown and never reset.
- "Check for updates" no longer calls GitHub's API directly (60
  requests/hour, per IP, shared with everything else on a user's network —
  doesn't scale across installs and can fail for reasons unrelated to
  eye-break). It now reads `version.json` off the project website instead,
  which has no comparable limit.
- Clicking the app icon now actually opens Settings. It used to run the
  exact same silent, window-less launch as the autostart entry — fine at
  login, but clicking the icon looked like nothing had happened, and did
  nothing at all if a tray/scheduler was already running (e.g. from
  autostart). The app-menu launcher now runs `eye-break --open`, which
  starts the tray/scheduler if needed and opens (or raises) Settings; the
  autostart entry is now its own separate `.desktop` file, unchanged.
- A break could ambush you the instant you reconnected to a remote (RDP/
  VNC) session after being away for a while, in Pomodoro mode. The
  idle → active transition reset the plain-interval scheduler's baseline
  so returning from idle wouldn't immediately trigger a break, but
  Pomodoro mode tracks its own separate phase clock and that wasn't reset
  the same way — a Work phase that finished while you were away (idle
  detection only pauses *new* breaks, it doesn't rewind a phase already
  overdue) fired the moment your very next input arrived. Returning from
  idle now resets both schedulers' baselines.
- Fullscreen-aware pausing missed real-world fullscreen apps that don't
  set the formal `_NET_WM_STATE_FULLSCREEN` window state — some games and
  players resize themselves to cover the screen directly rather than
  asking the window manager for fullscreen. Detection now also checks
  `_NET_WM_BYPASS_COMPOSITOR` (the hint most video players/games set
  regardless) and falls back to comparing the active window's geometry
  against the real monitor list, so a borderless window that exactly
  covers a monitor counts as fullscreen even with neither hint set.
- The `.deb`'s `Depends` was missing `x11-xserver-utils` (`xrandr`)
  entirely, and had `xdotool`/`wmctrl`/`x11-utils`/`xprintidle` listed
  under `Recommends` rather than `Depends` — skipped by a plain `dpkg -i`
  and any install done without `apt`'s recommends resolution. Every one of
  those tools backs a feature that fails *open* (silently does nothing)
  rather than erroring when missing: no correctly-positioned multi-monitor
  overlay without `xrandr`, no fullscreen/idle-aware pausing without
  `xdotool`+`xprop`, no staying above other windows without `wmctrl`. On
  an install missing any of them, the app runs but quietly loses exactly
  the anti-disruption behavior it's supposed to have — indistinguishable
  from those features being broken. All five are now hard `Depends`;
  `dpkg-shlibdeps` only ever covered linked libraries, never these
  shelled-out CLI tools, so this needed a manual audit of every
  `Command::new(...)` call rather than something the automatic derivation
  could catch. Existing installs missing them can be fixed with
  `sudo apt-get install -f` (or `sudo apt-get install xdotool wmctrl
  x11-utils x11-xserver-utils xprintidle` directly).

## [0.5.0] - 2026-08-19

### Changed
- Settings window now follows the selected Theme (previously fixed to the
  warm "Classical" palette regardless of the chosen theme), with a proper
  per-theme derived color palette.
- Settings' content card gained real depth: shadowing and hover states.

### CI
- Release workflow grants `contents:write` so the release-attach step can
  upload build artifacts to GitHub Releases.

## [0.4.1] - 2026-08-19

### Fixed
- CI: install `libxdo-dev`, needed by the `arboard`/`libxdo` transitive
  dependency (Linux release builds).

## [0.4.0] - 2026-08-19

### Added
- Native macOS support (universal binary, via a `tao`-driven event loop
  instead of GTK).

### Changed
- Release build optimized for size (`opt-level = "s"`, LTO, single
  codegen unit, stripped, `panic = "abort"`) — the app is idle almost all
  the time, so a smaller binary was preferred over marginally faster code.

## [0.3.3] - 2026-08-19

### Changed
- Overlay, corner timer, and Settings visuals matched precisely to the
  Claude Design mockup: real fonts (Cormorant Garamond + Lora), line-art
  icons, restyled corner timer.

### Added
- Corner-timer opacity control.

## [0.3.2] - 2026-08-19

### Fixed
- Toggle switches (and the Stats period control) were blowing up their
  row height under an unconstrained `right_to_left` layout.

## [0.3.1] - 2026-08-19

### Added
- Single-instance enforcement for the tray, Settings window, and corner
  timer — a second launch (e.g. autostart + a manual start) now exits
  instead of racing the first.

## [0.3.0] - 2026-08-19

### Added
- Full Settings window ported from the Claude Design "Classical" mockup:
  sidebar nav, warm palette, painted gauges (interval dial, workday clock,
  24h activity wheel).
- Idle detection and fullscreen-awareness ("smart pausing").
- Instant-dismiss button on the break overlay; usage-stats bar chart.

## [0.2.0] - 2026-08-19

### Added
- Full settings window wiring theme, sound, Pomodoro, workday schedule,
  usage stats, autostart, and the update checker together.
- Reminder-text customization, autostart toggle, GitHub-release update
  checker.
- Daily usage-stats tracking and workday-schedule configuration.
- Pomodoro timer module (work/break cycle, epoch-based).
- Notification sounds module (CLI-player based, avoids ALSA dev headers).
- Theming module with 5 palettes (Dark, Light, Solarized, High Contrast,
  Nord), applied to the overlay and corner timer.

### Packaging
- `pulseaudio-utils`/`alsa-utils` added as a `.deb` Recommends, for
  notification sounds.

## [0.1.0] - 2026-08-19

### Added
- Initial working prototype: tray icon, 20-20-20 interval scheduler,
  guided break-exercise overlay.
- Debian packaging (`cargo-deb`) with icons, `.desktop` entry, and
  autostart.

[Unreleased]: https://github.com/noorhaq/eye_break/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/noorhaq/eye_break/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/noorhaq/eye_break/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/noorhaq/eye_break/compare/v0.3.3...v0.4.0
[0.3.3]: https://github.com/noorhaq/eye_break/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/noorhaq/eye_break/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/noorhaq/eye_break/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/noorhaq/eye_break/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/noorhaq/eye_break/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/noorhaq/eye_break/releases/tag/v0.1.0
