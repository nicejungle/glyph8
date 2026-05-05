# QA Checklist

## Stage 0 + 1.A complete:
- Workspace builds clean
- nes-core: Frame, ControllerState, EmulatorError, iNES parser, EmulatorBackend trait — all unit-tested
- nes-tetanes-backend: TetanesBackend implements EmulatorBackend
- Integration: synthetic NROM loads + steps + produces audio + resets
- Tests passing: cargo test --workspace
- Lints clean: cargo clippy --workspace --all-targets -- -D warnings

## Stage 1.B complete:
- nes-render: HalfblockRenderer (full + diff) — unit-tested
- glyph8-cli: --headless deterministic on nestest (integration test)
- glyph8-cli: interactive runloop runs nestest + bundled boing.nes
- Manual: ESC restores terminal cleanly (alt screen exit, raw mode off, cursor visible)
- Manual: R resets the emulator mid-run
- Tests passing: cargo test --workspace
- Lints clean: cargo clippy --workspace --all-targets -- -D warnings
