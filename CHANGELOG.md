# AOOSTAR WTR MAX Screen Control Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

_Changes in the next release_

## v0.3.0 - 2026-06-13
### Added
- `aster-ui`, a headless widget dashboard renderer for the 960x376 LCD.
- Declarative TOML dashboard configuration and a validated CSS subset for
  flex layouts, styling, text, images, and progress indicators.
- Advanced dashboard widgets: circular progress, graphs, gauges, conditional
  content, and reusable parameterized components.
- Sensor value bindings with defaults and numeric formatting.
- One-shot dashboard previews using `--dashboard`, `--render-once`, and
  `--save`.
- Continuous dashboard rendering with sensor updates, frame deduplication, and
  live reload of dashboard, stylesheet, font, and image assets.
- Dashboard examples for system metrics, storage, advanced widgets, the
  original AOOSTAR panel, and an ultrawide hardware monitor.
- Explicit JetBrains Mono and Nerd Font assets for reproducible dashboard
  typography and icons.
- Nix flake development environment.
- Sensor identifier mapping and filtering configuration.
- Internal date and time sensor values.
- CPU usage, load average, hostname, and formatted uptime values from
  `aster-sysinfo`.

### Changed
- Temperature values and units are exposed separately for more flexible
  dashboard formatting.
- Sensor file handling supports filtering and improved integration between
  `aster-sysinfo` output and AOOSTAR-compatible sensor identifiers.
- Documentation now covers widget dashboards, bindings, supported CSS,
  preview rendering, continuous display mode, and this fork's additions.

### Fixed
- Dashboard reload failures retain the last valid frame instead of
  interrupting continuous display mode.
- Identical dashboard frames are no longer saved or transmitted repeatedly.

---

## v0.2.0 - 2025-08-31
### Fixed
- Misplaced text sensors in custom panels ([#11](https://github.com/zehnm/aoostar-rs/issues/11)).
- Wrong start position for circular progress (fan) sensor using a counter-clockwise direction ([#12](https://github.com/zehnm/aoostar-rs/issues/12)).
- aster-sysinfo tool: make sensor file world-readable, create all parent directories.

### Added
- Simple sensor panel with a file-based data source ([#6](https://github.com/zehnm/aoostar-rs/issues/6)). 
- Initial support for fan-, progress-, & pointer-sensors ([#8](https://github.com/zehnm/aoostar-rs/pull/8)).
- Use [mdBook](https://rust-lang.github.io/mdBook/) for documentation and publish user guide to GitHub pages ([#10](https://github.com/zehnm/aoostar-rs/pull/10)).
- Initial `aster-sysinfo` tool for providing sensor values in a text file for `asterctl`.

### Changed
- Project structure using a Cargo workspace.

---

## v0.1.0 - 2025-08-02
### Added
- Initial `asterctl` tool release for controlling the LCD: on, off, display an image.
- systemd service file to switch off LCD on system start.
- Demo mode.
