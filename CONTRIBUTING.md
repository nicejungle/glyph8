# Contributing to glyph8

Thanks for your interest! glyph8 is a small Rust project and contributions
are welcome. This guide covers the basics of building, testing, and getting
a change merged.

By contributing, you agree your changes will be dual-licensed under
**Apache-2.0** and **MIT** (see [`LICENSE-APACHE`](LICENSE-APACHE) and
[`LICENSE-MIT`](LICENSE-MIT)).

## Getting started

```sh
git clone https://github.com/nicejungle/glyph8
cd glyph8
cargo build --workspace
cargo test --workspace
```

The binary lives in `crates/glyph8-cli`:

```sh
cargo run -p glyph8-cli -- tests/roms/nestest.nes
cargo run -p glyph8-cli -- --headless --frames=60 tests/roms/nestest.nes
```

## Project layout

- `crates/nes-core` — shared NES types/traits.
- `crates/nes-tetanes-backend` — adapter to the `tetanes-core` emulator.
- `crates/nes-render` — terminal renderer (24-bit halfblock).
- `crates/glyph8-cli` — the `glyph8` binary, runloop, input, status line.
- `tests/roms/` — bundled ROMs (see their licenses in
  [`README.md`](README.md#bundled-roms)).
- `docs/superpowers/` — design specs and implementation plans, organized
  by stage. Read the spec for the stage you're touching before changing
  related code.
- `docs/qa-checklist.md` — per-stage manual acceptance checks. Re-run the
  relevant section when you change rendering, input, or the runloop.

## Before you open a PR

Run all of these from the workspace root:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If your change affects rendering, input, or terminal behavior, also walk
through the relevant section of [`docs/qa-checklist.md`](docs/qa-checklist.md)
in a real terminal — CI can't catch visual regressions.

## Commit style

- One logical change per commit.
- Subject line: imperative mood, 72 chars or less, prefixed with the
  affected area (e.g. `render:`, `cli:`, `core:`, `docs:`).
- Body explains *why* the change is needed when it isn't obvious from the
  diff. Recent history (`git log --oneline`) shows the prevailing style.

## Filing issues

- **Bugs** — please use the bug report template and include your terminal
  emulator, terminal size, OS, and the output of `glyph8 --headless` if
  the bug is reproducible non-interactively.
- **Feature ideas** — open a discussion before large changes; the roadmap
  in [`README.md`](README.md#roadmap) shows what's already planned.

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
Please be respectful in all interactions.
