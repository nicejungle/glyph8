# glyph8

CLI NES emulator — runs in your terminal as ANSI 24-bit color halfblocks.

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

## Development

See `docs/qa-checklist.md` for the per-stage acceptance checks and
`docs/superpowers/` for design docs and implementation plans.
