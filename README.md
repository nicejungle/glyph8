# glyph8

[![CI](https://github.com/nicejungle/glyph8/actions/workflows/ci.yml/badge.svg)](https://github.com/nicejungle/glyph8/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)

> CLI NES emulator that renders to your terminal in 24-bit color halfblocks.

**Website:** [nicejungle.github.io/glyph8](https://nicejungle.github.io/glyph8/)

![demo](docs/assets/demo.gif)

> **Status:** Stage 1.B beta — playable picture, basic input, no audio yet.
> See the [Roadmap](#roadmap) and [Beta limitations](#beta-limitations) below.

## Install

```sh
cargo install --path crates/glyph8-cli
# Or run from the workspace without installing:
cargo run -p glyph8-cli -- path/to/your.nes
```

## Quick start

```sh
glyph8 tests/roms/nestest.nes              # bundled CPU validation ROM
glyph8 tests/roms/boing.nes                # bundled CC BY 4.0 demo (Brad Smith)
glyph8 path/to/your.nes                    # any standard iNES file
glyph8 --headless --frames=60 your.nes     # CI / determinism check
```

## Controls (Mednafen-style)

| Key | NES |
|---|---|
| ↑ ↓ ← → | D-pad |
| Z | B |
| X | A |
| Enter | Start |
| Tab | Select (beta — RightShift in 1.C) |
| R | Reset |
| Esc, Ctrl+C | Quit |

## Beta limitations

- **No audio yet.** Coming in Stage 1.D.
- **No key-release events** — your terminal sends presses only (without the
  Kitty keyboard protocol). Holding a button reads as repeated taps. Stage 1.C
  fixes this.
- **Halfblock-only renderer with nearest-neighbor downsampling** — the picture
  scales (with NES aspect preserved) to fit your terminal. Native resolution
  is 256×120 cells; below that the picture is downsampled. Below 64×33 cells
  the renderer bails. True per-pixel braille / ASCII modes still come in 1.E.
- **No status bar UI** — current status line is plain text. Stage 1.F adds a
  ratatui-based status bar with pause modal.
- **Commercial ROMs not bundled.** Bring your own legally-acquired ROM.

## Roadmap

| Stage | Focus | State |
|---|---|---|
| 1.A | CPU/PPU core via `tetanes-core` backend, halfblock encoder | done |
| 1.B | Adaptive halfblock renderer, interactive runloop, status line | **current** |
| 1.C | Kitty keyboard protocol — real key-release, RightShift Select | next |
| 1.D | Audio (cpal output) | planned |
| 1.E | Per-pixel braille + ASCII renderer modes | planned |
| 1.F | ratatui status bar + pause modal | planned |

## Development

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

See [`docs/qa-checklist.md`](docs/qa-checklist.md) for the per-stage
acceptance checks and [`docs/superpowers/`](docs/superpowers/) for design
docs and implementation plans. New contributors: read
[`CONTRIBUTING.md`](CONTRIBUTING.md) first.

## License

Dual-licensed under either of

- **Apache License, Version 2.0** ([`LICENSE-APACHE`](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- **MIT license** ([`LICENSE-MIT`](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

### Bundled ROMs

- `tests/roms/nestest.nes` — public domain CPU validation ROM by kevtris.
- `tests/roms/boing.nes` — © Brad Smith, distributed under
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Attribution
  preserved per the original release.
