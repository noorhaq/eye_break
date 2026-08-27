# Changelog

All notable changes to Eye Break are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed
- Solarized is now the default theme for new installs (was Dark). Existing
  `config.json` files already record an explicit `theme`, so this changes
  nothing for current users — pick it in Settings › Theme.
- The Settings window is back to its fixed warm "Classical" palette
  regardless of the selected Theme, reverting the 0.5.0 change that made it
  follow the theme. `Theme` once again applies to the break overlay only,
  which is what its own description in Settings has always said. The
  procedurally-derived per-theme palette (`design::palette_for`) is gone;
  `design::apply` no longer takes a `Theme`.

### Added
- `README.md` with installation (`.deb`, macOS zip, from-source), CLI, and
  config-file-location documentation.
- Licensed under GPL-3.0-or-later.
- Regression tests for the Pomodoro corner-timer math (`phase_progress`,
  phase-cycle ordering).

### Fixed
- Break overlays no longer appear while a fullscreen app is focused. The
  check shelled out to `xdotool`, which is only a `Recommends` and so is
  absent on a `dpkg -i` install; being fail-open, a missing tool silently
  disabled fullscreen suppression entirely. It now reads the focused
  window from the root `_NET_ACTIVE_WINDOW` property via `xprop`, with
  `xdotool` kept only as a fallback.
- The overlay's "OK, I'm done" / "Skip" buttons are no longer swallowed by
  the taskbar. They were anchored 70px above the screen bottom — inside the
  strip a dock occupies — and the always-on-top re-assertion that was meant
  to keep the overlay above the dock depends on `wmctrl`/`xdotool`, absent
  for the same packaging reason, leaving the break undismissable by mouse.
  The buttons now sit just below the countdown text.
- `xdotool`, `wmctrl`, `xprintidle`, `x11-utils`, and `x11-xserver-utils`
  are now hard `Depends` rather than `Recommends`. Every feature built on
  them (fullscreen suppression, always-on-top, idle-based smart pause,
  multi-monitor geometry) fails open, so on a `dpkg -i` install they were
  all silently inert.
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
