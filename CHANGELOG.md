# Changelog

All notable changes to glyph8 are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning
follows [SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha] - 2026-05-07

First public preview. Stage 1.A + 1.B work, packaged as an open-source
release. No `cargo publish` yet.

### Added
- Workspace with four crates: `nes-core`, `nes-tetanes-backend`,
  `nes-render`, `glyph8-cli`.
- `tetanes-core`-backed CPU/PPU emulation (Stage 1.A).
- 24-bit ANSI halfblock encoder with diff redraw.
- Adaptive halfblock renderer that scales to any terminal at least
  64×33 cells while preserving NES aspect; bails with a friendly error
  below that.
- Interactive runloop: input + step + halfblock blit + status line.
- Mednafen-style keymap (Z/X/Enter/Tab/arrows + R reset).
- Sliding-window FPS meter.
- `--headless --frames=N` mode with deterministic output, used for the
  bundled nestest integration test.
- Bundled ROMs: `nestest.nes` (public domain, kevtris) and `boing.nes`
  (CC BY 4.0, Brad Smith).

### Known limitations
- No audio (planned for 1.D).
- No key-release events without the Kitty keyboard protocol — held keys
  read as repeated taps. Fixed in Stage 1.C.
- Halfblock-only renderer; per-pixel braille / ASCII modes come in 1.E.
- Status line is plain text; ratatui status bar with pause modal is 1.F.

[Unreleased]: https://github.com/nicejungle/glyph8/compare/v0.1.0-alpha...HEAD
[0.1.0-alpha]: https://github.com/nicejungle/glyph8/releases/tag/v0.1.0-alpha
