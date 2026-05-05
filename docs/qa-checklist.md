# QA Checklist

## Stage 0 + 1.A complete:
- Workspace builds clean
- nes-core: Frame, ControllerState, EmulatorError, iNES parser, EmulatorBackend trait — all unit-tested
- nes-tetanes-backend: TetanesBackend implements EmulatorBackend
- Integration: synthetic NROM loads + steps + produces audio + resets
- Tests passing: cargo test --workspace
- Lints clean: cargo clippy --workspace --all-targets -- -D warnings
