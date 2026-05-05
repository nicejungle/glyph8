# Glyph8 — Stage 1.B Renderer + CLI Beta Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `nes-render` (halfblock terminal renderer + minimal `Renderer` trait) and `glyph8-cli` (the `glyph8` binary with interactive runloop and headless mode), so the user can run `glyph8 path/to/rom.nes` and play a NES ROM in the terminal. End state: `cargo test --workspace` green, `glyph8` binary works on bundled `nestest.nes` + at least one homebrew demo, ESC quits cleanly.

**Architecture:** Two new workspace crates. `nes-render` exposes a 3-method `Renderer` trait and one impl (`HalfblockRenderer`) that diff-encodes NES frames into ANSI 24-bit color escapes. `glyph8-cli` is a binary crate that wires `TetanesBackend` + `Input` (crossterm key events → `ControllerState`) + `HalfblockRenderer` in a single-threaded fixed-step loop (NTSC 60.0988 Hz). A separate `--headless` mode steps N frames and prints a blake3 hash for CI / golden testing.

**Tech Stack:** Rust 2021, crossterm 0.28 (terminal IO + key events), clap 4 derive (CLI), anyhow (CLI error wrapping), blake3 (frame hash), `tetanes-core` 0.12.2 (already wired via `nes-tetanes-backend`).

---

## Spec Reference

This plan implements `docs/superpowers/specs/2026-05-05-glyph8-stage-1b-renderer-cli-design.md`. Refer to the spec for design rationale; this plan is execution-only.

## File Structure

```
glyph8/
├── Cargo.toml                            # workspace — add 2 new members
├── crates/
│   ├── nes-render/                       # NEW crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                    # Renderer trait + module re-exports
│   │       └── halfblock.rs              # HalfblockRenderer
│   └── glyph8-cli/                       # NEW crate; produces `glyph8` binary
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs                   # arg parsing + dispatch
│           ├── cli.rs                    # clap Args struct
│           ├── runloop.rs                # interactive mode
│           ├── headless.rs               # --headless mode
│           ├── input.rs                  # crossterm KeyEvent → ControllerState
│           └── fps.rs                    # FPS meter
├── tests/
│   └── roms/
│       ├── nestest.nes                   # public-domain CPU validation ROM
│       └── <homebrew>.nes                # CC0/PD demo, picked in Task 12
└── docs/
    ├── qa-checklist.md                   # appended in Task 13
    └── README.md                         # appended in Task 13 (or new file)
```

Why this split:

- `nes-render` separate from CLI: per spec §3 (`nes-render` is its own crate so 1.E can add `BrailleRenderer` / `AsciiRenderer` next to `HalfblockRenderer` without touching CLI)
- `glyph8-cli` split into 6 small files: each ~50–150 LOC, each one responsibility (clap, runloop, headless, input, fps)
- `tests/roms/` at workspace root (not inside any crate) so both manual testing and integration tests reference the same paths

## Conventions

- After every step that adds or modifies code, run `cargo fmt` before committing.
- One task = one (or rarely two) commit(s). Don't batch tasks.
- Commit message format: `<area>: <what changed>`. Areas: `render`, `cli`, `roms`, `docs`, `chore`.
- Each implementation task is TDD: write the failing test → run it → implement → run again → commit.
- The `Renderer` trait file (`nes-render/src/lib.rs`) doesn't have its own test (a trait is a contract, not behavior); coverage starts at Task 3 when `HalfblockRenderer` exists.

---

### Task 1: nes-render crate scaffold + Renderer trait

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `crates/nes-render/Cargo.toml`
- Create: `crates/nes-render/src/lib.rs`

- [ ] **Step 1: Add `nes-render` to workspace members**

Edit `Cargo.toml` (workspace root). The `members` array should be:

```toml
members = [
    "crates/nes-core",
    "crates/nes-tetanes-backend",
    "crates/nes-render",
]
```

- [ ] **Step 2: Create `crates/nes-render/Cargo.toml`**

```toml
[package]
name = "nes-render"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
nes-core = { path = "../nes-core" }
crossterm = "0.28"

[lints]
workspace = true
```

- [ ] **Step 3: Create `crates/nes-render/src/lib.rs`**

```rust
//! Terminal renderers for NES frames.
//!
//! The [`Renderer`] trait is the abstraction that the CLI talks to.
//! Stage 1.B ships only [`halfblock::HalfblockRenderer`]; later stages
//! (1.E) will add braille and ASCII implementations.

use std::io;

use nes_core::Frame;

pub mod halfblock;

pub trait Renderer {
    /// Enter alternate screen / raw mode / hide cursor.
    /// Implementations not attached to a terminal (e.g. test sinks) may no-op.
    fn enter(&mut self) -> io::Result<()>;

    /// Draw a single frame. Caller guarantees `frame.pixels.len() == nes_core::FRAME_BYTES`.
    fn draw(&mut self, frame: &Frame) -> io::Result<()>;

    /// Restore terminal state. Must be safe to call multiple times.
    fn leave(&mut self) -> io::Result<()>;
}
```

- [ ] **Step 4: Create stub `crates/nes-render/src/halfblock.rs`**

Stub keeps the module declared so the crate compiles; real impl in Task 2.

```rust
//! Halfblock renderer — see Task 2.
```

- [ ] **Step 5: Verify the workspace builds**

Run: `cargo check --workspace`
Expected: `Finished`, no errors. The `nes-render` crate compiles with just the trait + empty module.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/nes-render
git commit -m "render: crate scaffold + Renderer trait"
```

---

### Task 2: HalfblockRenderer — full-frame ANSI encoding

**Files:**
- Modify: `crates/nes-render/src/halfblock.rs`
- Test: same file, `#[cfg(test)] mod tests`

This task gets a Frame → ANSI bytes encoder working with full repaint every call (no diff yet — diff is Task 3). The renderer is generic over `W: Write` so tests can use a `Vec<u8>` sink.

- [ ] **Step 1: Write the failing test**

Add to `crates/nes-render/src/halfblock.rs`:

```rust
use std::io::{self, Write};

use nes_core::{Frame, BPP, HEIGHT, WIDTH};

/// A NES frame rendered as halfblock characters (▀, U+2580).
/// Each terminal cell encodes 2 vertical NES pixels: fg = top, bg = bottom.
pub struct HalfblockRenderer<W: Write> {
    out: W,
    manage_terminal: bool,
}

impl<W: Write> HalfblockRenderer<W> {
    /// For tests / non-terminal sinks. `enter`/`leave` will not touch terminal state.
    pub fn with_writer(out: W) -> Self {
        Self {
            out,
            manage_terminal: false,
        }
    }
}

impl<W: Write> crate::Renderer for HalfblockRenderer<W> {
    fn enter(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn draw(&mut self, frame: &Frame) -> io::Result<()> {
        // Implementation comes in Step 3.
        let _ = frame;
        Ok(())
    }
    fn leave(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Renderer;

    #[test]
    fn draw_emits_one_halfblock_char_per_cell() {
        let mut buf: Vec<u8> = Vec::new();
        let mut r = HalfblockRenderer::with_writer(&mut buf);
        let frame = Frame::default();
        r.draw(&frame).unwrap();
        // 256 cols × 120 row-pairs = 30,720 cells, each containing one ▀ (3 UTF-8 bytes).
        let halfblocks = buf.windows(3).filter(|w| *w == "▀".as_bytes()).count();
        assert_eq!(halfblocks, WIDTH * HEIGHT / 2);
    }

    #[test]
    fn draw_includes_24bit_color_escape_for_each_pixel() {
        // A frame with one bright-red top pixel at (0,0) should produce an SGR 38;2;255;0;0 escape.
        let mut buf: Vec<u8> = Vec::new();
        let mut r = HalfblockRenderer::with_writer(&mut buf);
        let mut frame = Frame::default();
        // Frame is RGB; pixel (0,0) → bytes [0..3]
        frame.pixels[0] = 255;
        frame.pixels[1] = 0;
        frame.pixels[2] = 0;
        r.draw(&frame).unwrap();
        let s = String::from_utf8_lossy(&buf);
        assert!(
            s.contains("38;2;255;0;0"),
            "expected fg escape for red pixel, got: {}",
            &s[..s.len().min(200)]
        );
    }
}
```

Note: `nes_core::WIDTH`, `HEIGHT`, `BPP` should already be exported (they were added in Stage 0 + 1.A). If a re-export is missing, fix it as part of this task.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p nes-render`
Expected: 2 tests, both FAIL (the stub `draw` does nothing).

- [ ] **Step 3: Implement `draw`**

Replace the `draw` method body in `crates/nes-render/src/halfblock.rs`:

```rust
fn draw(&mut self, frame: &Frame) -> io::Result<()> {
    use std::io::Write as _;

    debug_assert_eq!(frame.pixels.len(), WIDTH * HEIGHT * BPP);

    let row_pairs = HEIGHT / 2;
    for ry in 0..row_pairs {
        // Cursor to row ry+1, col 1 (1-indexed in ANSI).
        write!(self.out, "\x1b[{};1H", ry + 1)?;
        for x in 0..WIDTH {
            // Top pixel: row 2*ry, col x. Bottom: row 2*ry+1, col x.
            let top = (2 * ry * WIDTH + x) * BPP;
            let bot = ((2 * ry + 1) * WIDTH + x) * BPP;
            let (tr, tg, tb) = (frame.pixels[top], frame.pixels[top + 1], frame.pixels[top + 2]);
            let (br, bg, bb) = (frame.pixels[bot], frame.pixels[bot + 1], frame.pixels[bot + 2]);
            // SGR: foreground 24-bit color, then background 24-bit color, then ▀.
            write!(
                self.out,
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                tr, tg, tb, br, bg, bb
            )?;
        }
    }
    // Reset attributes at end of frame.
    write!(self.out, "\x1b[0m")?;
    self.out.flush()?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p nes-render`
Expected: 2 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/nes-render/src/halfblock.rs
git commit -m "render: halfblock encoder (full-frame, no diff)"
```

---

### Task 3: HalfblockRenderer — diff against previous frame

**Files:**
- Modify: `crates/nes-render/src/halfblock.rs`

The full-frame encoder of Task 2 emits ~30k cells × 60 fps ≈ 70 MB/s of ANSI to stdout. Most terminals choke. This task adds frame-to-frame diffing: only re-emit a cell whose top/bottom pixel pair changed since the previous frame.

- [ ] **Step 1: Write the failing test**

Append to the `tests` module in `crates/nes-render/src/halfblock.rs`:

```rust
#[test]
fn second_draw_of_identical_frame_is_minimal() {
    let mut buf: Vec<u8> = Vec::new();
    let mut r = HalfblockRenderer::with_writer(&mut buf);
    let frame = Frame::default();
    r.draw(&frame).unwrap();
    let bytes_first = buf.len();
    buf.clear();
    r.draw(&frame).unwrap();
    let bytes_second = buf.len();
    // After Task 3, redrawing the same frame should emit < 5% the bytes
    // (only the SGR reset + maybe a cursor home), no per-cell escapes.
    assert!(
        bytes_second * 20 < bytes_first,
        "diff redraw too large: first={}, second={}",
        bytes_first,
        bytes_second
    );
}

#[test]
fn diff_emits_changed_cell_only() {
    let mut buf: Vec<u8> = Vec::new();
    let mut r = HalfblockRenderer::with_writer(&mut buf);
    let mut frame = Frame::default();
    r.draw(&frame).unwrap();
    buf.clear();
    // Flip one top pixel to red.
    frame.pixels[0] = 255;
    r.draw(&frame).unwrap();
    let s = String::from_utf8_lossy(&buf);
    assert!(s.contains("38;2;255;0;0"), "expected the changed cell's red fg in the diff output");
    // The number of ▀ chars should be 1 (only the changed cell).
    let halfblocks = buf.windows(3).filter(|w| *w == "▀".as_bytes()).count();
    assert_eq!(halfblocks, 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p nes-render`
Expected: previous 2 tests still PASS; 2 new ones FAIL (full-frame encoder writes everything every time).

- [ ] **Step 3: Add `prev` buffer and rewrite `draw` with diff**

Replace the full struct + impl block (keeping the doc comment) in `crates/nes-render/src/halfblock.rs`:

```rust
type Cell = ((u8, u8, u8), (u8, u8, u8)); // (fg=top, bg=bottom)

pub struct HalfblockRenderer<W: Write> {
    out: W,
    manage_terminal: bool,
    /// Previous frame's cells, indexed by (row_pair * WIDTH + x).
    /// `None` until first draw — first draw paints everything.
    prev: Option<Vec<Cell>>,
}

impl<W: Write> HalfblockRenderer<W> {
    pub fn with_writer(out: W) -> Self {
        Self {
            out,
            manage_terminal: false,
            prev: None,
        }
    }
}

impl<W: Write> crate::Renderer for HalfblockRenderer<W> {
    fn enter(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn draw(&mut self, frame: &Frame) -> io::Result<()> {
        debug_assert_eq!(frame.pixels.len(), WIDTH * HEIGHT * BPP);

        let row_pairs = HEIGHT / 2;
        let cell_count = row_pairs * WIDTH;
        let mut current: Vec<Cell> = Vec::with_capacity(cell_count);
        for ry in 0..row_pairs {
            for x in 0..WIDTH {
                let top = (2 * ry * WIDTH + x) * BPP;
                let bot = ((2 * ry + 1) * WIDTH + x) * BPP;
                current.push((
                    (frame.pixels[top], frame.pixels[top + 1], frame.pixels[top + 2]),
                    (frame.pixels[bot], frame.pixels[bot + 1], frame.pixels[bot + 2]),
                ));
            }
        }

        let prev_ref = self.prev.as_deref();
        for ry in 0..row_pairs {
            // Track whether the cursor is currently at the start of run we want to draw.
            // We only emit a cursor-position escape when we hit a changed cell after a skip.
            let mut last_drawn_x: Option<usize> = None;
            for x in 0..WIDTH {
                let idx = ry * WIDTH + x;
                let cur = current[idx];
                let changed = match prev_ref {
                    None => true, // first draw paints everything
                    Some(p) => p[idx] != cur,
                };
                if !changed {
                    last_drawn_x = None;
                    continue;
                }
                // Position cursor only when we just resumed after skipping.
                if last_drawn_x != Some(x.wrapping_sub(1)) {
                    write!(self.out, "\x1b[{};{}H", ry + 1, x + 1)?;
                }
                let ((tr, tg, tb), (br, bg, bb)) = cur;
                write!(
                    self.out,
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                    tr, tg, tb, br, bg, bb
                )?;
                last_drawn_x = Some(x);
            }
        }
        write!(self.out, "\x1b[0m")?;
        self.out.flush()?;
        self.prev = Some(current);
        Ok(())
    }

    fn leave(&mut self) -> io::Result<()> {
        Ok(())
    }
}
```

- [ ] **Step 4: Run tests to verify all 4 pass**

Run: `cargo test -p nes-render`
Expected: 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/nes-render/src/halfblock.rs
git commit -m "render: halfblock diff redraw (skip unchanged cells)"
```

---

### Task 4: HalfblockRenderer — terminal lifecycle (`for_stdout`, enter/leave)

**Files:**
- Modify: `crates/nes-render/src/halfblock.rs`

Adds the production constructor that owns `io::Stdout` and toggles raw mode + alt screen + cursor visibility on `enter`/`leave`. Test sinks (`with_writer`) keep `manage_terminal = false` so they don't touch the real terminal.

- [ ] **Step 1: Write the failing test**

Append to the `tests` module in `crates/nes-render/src/halfblock.rs`:

```rust
#[test]
fn enter_leave_are_noop_for_writer_sink() {
    // Sanity: the test constructor must NOT actually toggle terminal state.
    let mut buf: Vec<u8> = Vec::new();
    let mut r = HalfblockRenderer::with_writer(&mut buf);
    r.enter().unwrap();
    r.leave().unwrap();
    // Buffer should still be empty (no escapes for terminal toggling).
    assert!(buf.is_empty());
}
```

- [ ] **Step 2: Run test to verify it passes (already does)**

Run: `cargo test -p nes-render`
Expected: 5 tests PASS. This test guards a property we're about to leave intact while adding the stdout path.

- [ ] **Step 3: Add `for_stdout` constructor + real enter/leave**

Modify `crates/nes-render/src/halfblock.rs`. Add the constructor after the `with_writer` impl block:

```rust
impl HalfblockRenderer<io::Stdout> {
    /// Constructor for production use: owns stdout, toggles terminal state
    /// on `enter`/`leave` (raw mode, alt screen, cursor hide/show).
    pub fn for_stdout() -> Self {
        Self {
            out: io::stdout(),
            manage_terminal: true,
            prev: None,
        }
    }
}
```

Replace `enter` and `leave` in the `Renderer` impl:

```rust
fn enter(&mut self) -> io::Result<()> {
    if self.manage_terminal {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            self.out,
            crossterm::terminal::EnterAlternateScreen,
            crossterm::cursor::Hide,
        )
        .map_err(io::Error::other)?;
    }
    Ok(())
}

fn leave(&mut self) -> io::Result<()> {
    if self.manage_terminal {
        // Best-effort: try to restore everything even if some steps fail.
        let _ = crossterm::execute!(
            self.out,
            crossterm::cursor::Show,
            crossterm::terminal::LeaveAlternateScreen,
        );
        let _ = crossterm::terminal::disable_raw_mode();
        // Mark it done so a subsequent leave (e.g. via Drop) doesn't double-toggle.
        self.manage_terminal = false;
    }
    Ok(())
}
```

Add a `Drop` impl at the bottom of `crates/nes-render/src/halfblock.rs`:

```rust
impl<W: Write> Drop for HalfblockRenderer<W> {
    fn drop(&mut self) {
        // Best-effort terminal restoration on panic / unwind.
        let _ = <Self as crate::Renderer>::leave(self);
    }
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test -p nes-render`
Expected: 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/nes-render/src/halfblock.rs
git commit -m "render: HalfblockRenderer::for_stdout + raw mode lifecycle"
```

---

### Task 5: glyph8-cli crate scaffold + clap Args

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `crates/glyph8-cli/Cargo.toml`
- Create: `crates/glyph8-cli/src/main.rs`
- Create: `crates/glyph8-cli/src/cli.rs`

Stand up the binary crate with a working CLI parse, but no execution yet — `main` just prints what it parsed.

- [ ] **Step 1: Add `glyph8-cli` to workspace members**

`Cargo.toml` (workspace root):

```toml
members = [
    "crates/nes-core",
    "crates/nes-tetanes-backend",
    "crates/nes-render",
    "crates/glyph8-cli",
]
```

- [ ] **Step 2: Create `crates/glyph8-cli/Cargo.toml`**

```toml
[package]
name = "glyph8-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[[bin]]
name = "glyph8"
path = "src/main.rs"

[dependencies]
nes-core = { path = "../nes-core" }
nes-tetanes-backend = { path = "../nes-tetanes-backend" }
nes-render = { path = "../nes-render" }
crossterm = "0.28"
clap = { version = "4", features = ["derive"] }
anyhow = "1"
blake3 = "1"

[lints]
workspace = true
```

- [ ] **Step 3: Create `crates/glyph8-cli/src/cli.rs`**

```rust
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "glyph8", version, about = "CLI NES emulator (terminal)")]
pub struct Args {
    /// Path to a .nes ROM file.
    pub rom: PathBuf,

    /// Run N frames headless and print a blake3 hash of the final frame buffer.
    #[arg(long)]
    pub headless: bool,

    /// Number of frames to step in headless mode. Ignored without --headless.
    #[arg(long, default_value_t = 60)]
    pub frames: u32,
}
```

- [ ] **Step 4: Create `crates/glyph8-cli/src/main.rs`**

```rust
mod cli;

use clap::Parser;

fn main() {
    let args = cli::Args::parse();
    eprintln!(
        "parsed: rom={}, headless={}, frames={}",
        args.rom.display(),
        args.headless,
        args.frames
    );
}
```

- [ ] **Step 5: Verify the workspace builds**

Run: `cargo check --workspace`
Expected: `Finished`, no errors.

- [ ] **Step 6: Smoke-test the binary parses args**

Run: `cargo run -p glyph8-cli -- --help`
Expected: clap-rendered help text including the `<ROM>`, `--headless`, `--frames` flags.

Run: `cargo run -p glyph8-cli -- some/fake.nes --headless --frames=120`
Expected: `parsed: rom=some/fake.nes, headless=true, frames=120`. (No actual ROM read yet.)

- [ ] **Step 7: Commit**

```bash
cargo fmt
git add Cargo.toml crates/glyph8-cli
git commit -m "cli: glyph8-cli crate scaffold + clap args"
```

---

### Task 6: Add nestest.nes to tests/roms/

**Files:**
- Create: `tests/roms/nestest.nes` (binary)
- Create: `tests/roms/README.md`
- Create: `tests/roms/.gitattributes`

`nestest.nes` is the de-facto standard CPU-validation ROM, public domain, ~24 KB. Source: `http://nickmass.com/images/nestest.nes` (well-known mirror used by every major NES emulator).

- [ ] **Step 1: Download nestest.nes**

```bash
mkdir -p tests/roms
curl -fsSL -o tests/roms/nestest.nes http://nickmass.com/images/nestest.nes
```

Verify size and magic bytes:

```bash
ls -l tests/roms/nestest.nes
# Expected: ~24 KB (24,592 bytes is the canonical size)
xxd tests/roms/nestest.nes | head -1
# Expected: first 4 bytes "NES\x1a"
```

If the URL ever moves: search the NESdev Wiki or `nesdev_compo` GitHub mirrors for `nestest.nes` and update the README accordingly.

- [ ] **Step 2: Mark binary file in .gitattributes**

Create `tests/roms/.gitattributes`:

```
*.nes binary
```

This prevents Git from line-ending mangling and from trying to diff the binary.

- [ ] **Step 3: Create `tests/roms/README.md`**

```markdown
# Bundled Test ROMs

ROMs in this directory are used by integration tests and manual QA.

## nestest.nes

- **Author:** Kevin Horton (kevtris)
- **License:** Public domain (community consensus, used by every major NES emulator)
- **Source:** http://nickmass.com/images/nestest.nes
- **Purpose:** CPU instruction validation. The canonical reference for "does
  the 6502 core execute every documented opcode correctly?"

## <homebrew>.nes

(Added in plan Task 12 — a CC0 / public-domain homebrew demo so the user
can see something move on the screen, not just CPU-test patterns.)

## Commercial ROMs

Commercial NES ROMs (Super Mario Bros, Zelda, Contra, etc.) are NOT bundled
here for copyright reasons. To run one, point glyph8 at a ROM file you
legally own:

    glyph8 path/to/your.nes
```

- [ ] **Step 4: Verify the ROM loads via TetanesBackend**

This is a one-liner sanity check, not committed:

```bash
cargo run -p glyph8-cli -- tests/roms/nestest.nes --headless --frames=1
# Expected: still prints the parsed-args debug line (Task 5's stub).
# We're just confirming the path is reachable; real headless behavior comes in Task 7.
```

- [ ] **Step 5: Commit**

```bash
git add tests/roms
git commit -m "roms: bundle nestest.nes (public domain, CPU validation)"
```

---

### Task 7: glyph8-cli headless mode + integration test

**Files:**
- Create: `crates/glyph8-cli/src/headless.rs`
- Modify: `crates/glyph8-cli/src/main.rs`
- Create: `crates/glyph8-cli/tests/headless_nestest.rs`

End-to-end first: prove the backend drives `nestest.nes` reproducibly. Once this works, interactive mode is just glue.

- [ ] **Step 1: Write the failing integration test**

Create `crates/glyph8-cli/tests/headless_nestest.rs`:

```rust
//! End-to-end: run `glyph8 --headless --frames=60` on bundled nestest.nes
//! and assert that the emitted blake3 hash is reproducible across runs.
//!
//! We don't pin a specific hash value here — tetanes-core could change its
//! frame output between releases. Instead we run twice and assert equality,
//! which is a stronger property: the emulator must be deterministic.

use std::process::Command;

fn rom_path() -> String {
    // Workspace-root-relative — `cargo test` runs with CWD at the crate root,
    // so we go up two levels.
    let manifest = env!("CARGO_MANIFEST_DIR"); // .../crates/glyph8-cli
    format!("{}/../../tests/roms/nestest.nes", manifest)
}

#[test]
fn headless_run_is_deterministic() {
    let bin = env!("CARGO_BIN_EXE_glyph8");
    let run = || -> String {
        let out = Command::new(bin)
            .args(["--headless", "--frames=60", &rom_path()])
            .output()
            .expect("failed to run glyph8");
        assert!(
            out.status.success(),
            "glyph8 exited non-zero: stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };
    let h1 = run();
    let h2 = run();
    assert_eq!(h1, h2, "headless run must be deterministic across invocations");
    // Hash should be 64 hex chars (blake3 default = 256 bits).
    assert_eq!(h1.len(), 64, "blake3 hash should be 64 hex chars, got {:?}", h1);
}
```

- [ ] **Step 2: Run the test (expect failure)**

Run: `cargo test -p glyph8-cli --test headless_nestest`
Expected: FAIL — `main` currently just prints debug line; no hash output, exit code from Command call is 0 but stdout doesn't contain a 64-char hex.

- [ ] **Step 3: Implement headless mode**

Create `crates/glyph8-cli/src/headless.rs`:

```rust
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use nes_core::EmulatorBackend;
use nes_tetanes_backend::TetanesBackend;

pub fn run(rom_path: &Path, frames: u32) -> Result<()> {
    let bytes = fs::read(rom_path)
        .with_context(|| format!("reading ROM {}", rom_path.display()))?;
    let mut backend = TetanesBackend::new();
    backend.load_rom(&bytes)?;
    for _ in 0..frames {
        backend.step_frame()?;
    }
    let hash = blake3::hash(&backend.frame().pixels);
    println!("{}", hash.to_hex());
    Ok(())
}
```

- [ ] **Step 4: Wire `main.rs` to dispatch on `--headless`**

Replace `crates/glyph8-cli/src/main.rs`:

```rust
mod cli;
mod headless;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    if args.headless {
        headless::run(&args.rom, args.frames)
    } else {
        // Interactive runloop wired in Task 10.
        anyhow::bail!("interactive mode not yet implemented (use --headless for now)");
    }
}
```

- [ ] **Step 5: Run the integration test**

Run: `cargo test -p glyph8-cli --test headless_nestest`
Expected: PASS. Both runs produce identical 64-char blake3 hashes.

- [ ] **Step 6: Run all workspace tests**

Run: `cargo test --workspace`
Expected: existing core/backend tests still PASS, new test PASSes. No regressions.

- [ ] **Step 7: Commit**

```bash
cargo fmt
git add crates/glyph8-cli/src/headless.rs crates/glyph8-cli/src/main.rs crates/glyph8-cli/tests
git commit -m "cli: --headless mode + nestest determinism integration test"
```

---

### Task 8: glyph8-cli FPS meter

**Files:**
- Create: `crates/glyph8-cli/src/fps.rs`
- Modify: `crates/glyph8-cli/src/main.rs` (add `mod fps;`)

A small ring-buffer-based FPS meter, used by the runloop to render the status bar. Independent and trivially unit-testable, so we knock it out before runloop.

- [ ] **Step 1: Write the failing test**

Create `crates/glyph8-cli/src/fps.rs`:

```rust
use std::time::{Duration, Instant};

/// Sliding-window FPS meter. Records a tick on each frame; reports the
/// instantaneous rate over the most recent `WINDOW` ticks.
pub struct FpsMeter {
    window: Vec<Instant>,
    capacity: usize,
}

impl FpsMeter {
    pub fn new(capacity: usize) -> Self {
        Self {
            window: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Record a tick (one rendered frame). Returns the current FPS
    /// estimate, or 0.0 if fewer than 2 ticks have been seen.
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        if self.window.len() == self.capacity {
            self.window.remove(0);
        }
        self.window.push(now);
        if self.window.len() < 2 {
            return 0.0;
        }
        let span: Duration = *self.window.last().unwrap() - self.window[0];
        if span.is_zero() {
            return 0.0;
        }
        (self.window.len() - 1) as f32 / span.as_secs_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn first_tick_returns_zero() {
        let mut m = FpsMeter::new(4);
        assert_eq!(m.tick(), 0.0);
    }

    #[test]
    fn estimates_roughly_60fps_when_ticks_are_16ms_apart() {
        let mut m = FpsMeter::new(8);
        // 5 ticks at ~16 ms intervals should give ~60 fps.
        m.tick();
        for _ in 0..4 {
            sleep(Duration::from_millis(16));
            let _ = m.tick();
        }
        let fps = m.tick();
        // Allow generous tolerance for sleep jitter, but it should be in the 50–80 range.
        assert!(fps > 40.0 && fps < 100.0, "expected ~60fps, got {}", fps);
    }
}
```

- [ ] **Step 2: Wire the module into main.rs**

Add to top of `crates/glyph8-cli/src/main.rs`:

```rust
mod cli;
mod fps;
mod headless;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p glyph8-cli`
Expected: 2 new fps tests PASS, existing integration test still PASSes. (The slow test takes ~80 ms; that's fine.)

- [ ] **Step 4: Commit**

```bash
cargo fmt
git add crates/glyph8-cli/src/fps.rs crates/glyph8-cli/src/main.rs
git commit -m "cli: sliding-window FPS meter"
```

---

### Task 9: glyph8-cli Input module

**Files:**
- Create: `crates/glyph8-cli/src/input.rs`
- Modify: `crates/glyph8-cli/src/main.rs` (add `mod input;`)

Maps crossterm `KeyEvent`s to NES `ControllerState` and runloop control events. Pure-function entry point (`handle_event`) so tests don't need a real terminal.

- [ ] **Step 1: Write the failing test**

Create `crates/glyph8-cli/src/input.rs`:

```rust
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use nes_core::ControllerState;

/// Outcome of polling input for a frame.
#[derive(Debug, PartialEq)]
pub enum PollOutcome {
    Continue(ControllerState),
    Reset,
    Quit,
}

/// State carried across frames: which buttons are currently considered "held".
#[derive(Default)]
pub struct Input {
    p1: ControllerState,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a single crossterm event. Returns `Some(outcome)` if the event
    /// produced a runloop-level decision (Reset / Quit); otherwise `None`,
    /// meaning "state updated, keep looping".
    pub fn handle_event(&mut self, ev: &Event) -> Option<PollOutcome> {
        let Event::Key(KeyEvent { code, modifiers, kind, .. }) = ev else {
            return None;
        };
        // Most terminals only deliver `Press` (no repeat or release) without the
        // Kitty enhancement protocol. We treat every Press as both "press" and
        // "release at end of frame" — the runloop clears `self.p1` once per
        // frame before draining events. See spec §3.3 for the trade-off note.
        if !matches!(kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return None;
        }
        // Ctrl+C — quit regardless of code (covers Ctrl+C as char 'c').
        if modifiers.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('c') | KeyCode::Char('C')) {
            return Some(PollOutcome::Quit);
        }
        match code {
            KeyCode::Esc => return Some(PollOutcome::Quit),
            KeyCode::Char('r') | KeyCode::Char('R') => return Some(PollOutcome::Reset),
            KeyCode::Up => self.p1.press(ControllerState::UP),
            KeyCode::Down => self.p1.press(ControllerState::DOWN),
            KeyCode::Left => self.p1.press(ControllerState::LEFT),
            KeyCode::Right => self.p1.press(ControllerState::RIGHT),
            KeyCode::Char('z') | KeyCode::Char('Z') => self.p1.press(ControllerState::B),
            KeyCode::Char('x') | KeyCode::Char('X') => self.p1.press(ControllerState::A),
            KeyCode::Enter => self.p1.press(ControllerState::START),
            // Right Shift detection: crossterm reports it as a SHIFT modifier on
            // an empty key — there's no dedicated keycode. As a beta compromise,
            // we accept the more reliable signal: Tab as Select.
            // (Mednafen-style RShift→Select needs Kitty protocol; that comes in 1.C.)
            KeyCode::Tab => self.p1.press(ControllerState::SELECT),
            _ => {}
        }
        None
    }

    /// Reset the held-button mask. Call once per frame *before* draining events.
    pub fn begin_frame(&mut self) {
        self.p1 = ControllerState::empty();
    }

    /// Snapshot the current pressed-button mask for submission to the backend.
    pub fn p1(&self) -> ControllerState {
        self.p1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn esc_quits() {
        let mut i = Input::new();
        let r = i.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(r, Some(PollOutcome::Quit));
    }

    #[test]
    fn ctrl_c_quits() {
        let mut i = Input::new();
        let r = i.handle_event(&key(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(r, Some(PollOutcome::Quit));
    }

    #[test]
    fn r_resets() {
        let mut i = Input::new();
        let r = i.handle_event(&key(KeyCode::Char('r'), KeyModifiers::NONE));
        assert_eq!(r, Some(PollOutcome::Reset));
    }

    #[test]
    fn z_presses_b() {
        let mut i = Input::new();
        i.handle_event(&key(KeyCode::Char('z'), KeyModifiers::NONE));
        assert!(i.p1().pressed(ControllerState::B));
        assert!(!i.p1().pressed(ControllerState::A));
    }

    #[test]
    fn arrows_press_dpad() {
        let mut i = Input::new();
        i.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
        i.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
        assert!(i.p1().pressed(ControllerState::UP));
        assert!(i.p1().pressed(ControllerState::RIGHT));
    }

    #[test]
    fn begin_frame_clears_held_buttons() {
        let mut i = Input::new();
        i.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
        assert!(i.p1().pressed(ControllerState::UP));
        i.begin_frame();
        assert_eq!(i.p1(), ControllerState::empty());
    }
}
```

Note on the `Tab` → Select compromise: the spec said RightShift, but crossterm without Kitty protocol can't distinguish RShift from LShift, and the SHIFT modifier alone has no associated `KeyCode` event. Tab is unambiguous and rarely used in NES games. The README will document this; 1.C swaps it for true RightShift via Kitty protocol.

- [ ] **Step 2: Wire the module into main.rs**

Update `crates/glyph8-cli/src/main.rs` mod block:

```rust
mod cli;
mod fps;
mod headless;
mod input;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p glyph8-cli`
Expected: all input tests PASS, fps PASS, integration PASS.

- [ ] **Step 4: Commit**

```bash
cargo fmt
git add crates/glyph8-cli/src/input.rs crates/glyph8-cli/src/main.rs
git commit -m "cli: input module — keymap + ControllerState"
```

---

### Task 10: glyph8-cli runloop module

**Files:**
- Create: `crates/glyph8-cli/src/runloop.rs`
- Modify: `crates/glyph8-cli/src/main.rs`

Wires backend + renderer + input into the fixed-step main loop. There's no easy way to unit-test a runloop that owns real stdin/stdout; instead, we test it indirectly via Task 11's manual smoke run, and rely on each component being individually unit-tested.

- [ ] **Step 1: Create `crates/glyph8-cli/src/runloop.rs`**

```rust
use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::poll as event_poll;
use crossterm::event::read as event_read;

use nes_core::{ControllerState, EmulatorBackend};
use nes_render::Renderer;
use nes_render::halfblock::HalfblockRenderer;
use nes_tetanes_backend::TetanesBackend;

use crate::fps::FpsMeter;
use crate::input::{Input, PollOutcome};

/// NTSC frame period: ~16.639 ms.
const FRAME_DUR: Duration = Duration::from_nanos(16_639_267);

pub fn run(rom_path: &Path) -> Result<()> {
    let rom_bytes = fs::read(rom_path)
        .with_context(|| format!("reading ROM {}", rom_path.display()))?;
    let rom_label = rom_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("rom")
        .to_string();

    let mut backend = TetanesBackend::new();
    backend.load_rom(&rom_bytes)?;

    let mut renderer = HalfblockRenderer::for_stdout();
    let mut input = Input::new();
    let mut fps = FpsMeter::new(60);

    renderer.enter()?;
    let result = run_loop(&mut backend, &mut renderer, &mut input, &mut fps, &rom_label);
    // Always restore terminal, even on error.
    let _ = renderer.leave();
    result
}

fn run_loop(
    backend: &mut TetanesBackend,
    renderer: &mut HalfblockRenderer<std::io::Stdout>,
    input: &mut Input,
    fps: &mut FpsMeter,
    rom_label: &str,
) -> Result<()> {
    let mut next = Instant::now() + FRAME_DUR;
    loop {
        // Drain all pending events without blocking.
        input.begin_frame();
        let mut control: Option<PollOutcome> = None;
        while event_poll(Duration::ZERO)? {
            let ev = event_read()?;
            if let Some(outcome) = input.handle_event(&ev) {
                control = Some(outcome);
                break;
            }
        }
        match control {
            Some(PollOutcome::Quit) => return Ok(()),
            Some(PollOutcome::Reset) => backend.reset(),
            _ => {}
        }
        backend.submit_input(input.p1(), ControllerState::empty());
        backend.step_frame()?;
        renderer.draw(backend.frame())?;
        write_status_line(rom_label, fps.tick())?;
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        }
        next += FRAME_DUR;
    }
}

fn write_status_line(rom_label: &str, fps: f32) -> std::io::Result<()> {
    use crossterm::cursor::MoveTo;
    use crossterm::queue;
    use crossterm::style::Print;
    use crossterm::terminal::{Clear, ClearType};

    let mut out = std::io::stdout();
    // Status row is just below the 120 halfblock rows.
    queue!(
        out,
        MoveTo(0, 121),
        Clear(ClearType::CurrentLine),
        Print(format!(
            "{} | FPS: {:>5.1} | ESC: quit | R: reset",
            rom_label, fps
        )),
    )?;
    out.flush()
}
```

- [ ] **Step 2: Wire into main.rs and dispatch interactive path**

Replace `crates/glyph8-cli/src/main.rs`:

```rust
mod cli;
mod fps;
mod headless;
mod input;
mod runloop;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    if args.headless {
        headless::run(&args.rom, args.frames)
    } else {
        runloop::run(&args.rom)
    }
}
```

- [ ] **Step 3: Verify the workspace builds**

Run: `cargo check --workspace`
Expected: `Finished`, no errors. If you see an unresolved-import error on `nes_tetanes_backend`, fix the import line per the note above.

- [ ] **Step 4: Run all tests**

Run: `cargo test --workspace`
Expected: all PASS, no regressions. (No new tests; runloop is exercised by manual smoke in Step 5.)

- [ ] **Step 5: Manual smoke test**

```bash
cargo run -p glyph8-cli -- tests/roms/nestest.nes
```

Expected:
- Terminal switches to alt screen
- 256-column halfblock-rendered NES frame appears
- Status line at row 121: `nestest.nes | FPS: 60.0 | ESC: quit | R: reset` (FPS may show ~60 after a second or two)
- Pressing `Esc` exits, terminal restored to its previous state (cursor visible, no leftover ANSI)

If terminal dimensions are smaller than 256×122 cells, the picture will wrap or clip — that's expected for beta; 1.E will add adaptive sizing. Resize the terminal larger if needed.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add crates/glyph8-cli/src/runloop.rs crates/glyph8-cli/src/main.rs
git commit -m "cli: interactive runloop (input + step + halfblock + status)"
```

---

### Task 11: Workspace clippy + fmt sweep

**Files:**
- (no source changes expected unless lints trip)

- [ ] **Step 1: Clippy across the whole workspace**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: 0 warnings.

Common ones likely to trip in this code:
- `clippy::needless_pass_by_value` on small Copy structs — allow on the offending function
- `clippy::too_many_arguments` on `run_loop` — split or `#[allow]`
- `clippy::missing_errors_doc` — add a one-line `# Errors` to public fns or allow at crate level

Fix in place. If a lint is genuinely a code smell, fix it; if it's a false positive, allow narrowly with a comment explaining why.

- [ ] **Step 2: fmt check**

Run: `cargo fmt --all -- --check`
Expected: no diff.

If diff: run `cargo fmt --all`, review changes, commit them.

- [ ] **Step 3: Full test run**

Run: `cargo test --workspace`
Expected: all tests PASS.

- [ ] **Step 4: Commit any lint/fmt fixes**

```bash
git add -A
git diff --cached --quiet || git commit -m "chore: workspace clippy + fmt pass"
```

---

### Task 12: Research + bundle one homebrew demo ROM

**Files:**
- Create: `tests/roms/<chosen-demo>.nes`
- Modify: `tests/roms/README.md`

This task **requires user interaction**: the implementer presents 2–3 candidates, the user picks one, then it gets bundled. Don't make this decision unilaterally.

- [ ] **Step 1: Research candidates**

Goals: CC0 / public-domain / explicit-redistribute license, ≤ 100 KB, produces visible motion (not just title screen). Search:

- NESdev Wiki "Homebrew" page
- nesdev.org GitHub orgs
- itch.io NES homebrew tagged "free"
- `cc0-game.com` and similar

For each candidate, capture:
- Name
- Author
- License (verbatim quote from a README or itch.io page — not just "free")
- Source URL (the canonical download, not a third-party mirror)
- Size in bytes
- One-line description of expected behavior on first 60 frames

- [ ] **Step 2: Present 2–3 candidates to the user**

Use a message in the form:

```
Here are 3 candidates:

A. <name> by <author>
   License: <quoted license>
   Source: <URL>
   Size: <KB>
   Behavior: <what user will see>

B. ...
C. ...

Which one (or "find more")?
```

Wait for user choice.

- [ ] **Step 3: Download the chosen ROM**

```bash
curl -fsSL -o tests/roms/<demo>.nes <URL>
```

Verify:
```bash
ls -l tests/roms/<demo>.nes
# Expected: matches the size from research
xxd tests/roms/<demo>.nes | head -1
# Expected: starts with "NES\x1a"
```

- [ ] **Step 4: Smoke-test the new ROM**

```bash
cargo run -p glyph8-cli -- --headless --frames=60 tests/roms/<demo>.nes
# Expected: prints a 64-char blake3 hash (no error)
cargo run -p glyph8-cli -- tests/roms/<demo>.nes
# Expected: visible animation in terminal; ESC exits cleanly
```

- [ ] **Step 5: Update `tests/roms/README.md`**

Replace the `## <homebrew>.nes` placeholder section with the real entry:

```markdown
## <chosen-demo>.nes

- **Author:** <name>
- **License:** <quoted license>
- **Source:** <URL>
- **Purpose:** Visual sanity check — beta users can see motion on screen
  without supplying their own ROM.
```

- [ ] **Step 6: Commit**

```bash
git add tests/roms
git commit -m "roms: bundle <demo>.nes (<license>)"
```

---

### Task 13: README + qa-checklist update

**Files:**
- Create or modify: `README.md` (workspace root)
- Modify: `docs/qa-checklist.md`

Document how to install, run, the keymap, and known beta limitations.

- [ ] **Step 1: Create / overwrite `README.md`**

```markdown
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
- **No adaptive sizing** — terminal must be ≥ 256 columns × 122 rows for the
  picture to fit without wrapping. Stage 1.E adds braille / ASCII modes for
  smaller terminals.
- **No status bar UI** — current status line is plain text. Stage 1.F adds a
  ratatui-based status bar with pause modal.
- **Commercial ROMs not bundled.** Bring your own legally-acquired ROM.

## Development

See `docs/qa-checklist.md` for the per-stage acceptance checks and
`docs/superpowers/` for design docs and implementation plans.
```

- [ ] **Step 2: Append a Stage 1.B section to `docs/qa-checklist.md`**

Add at the bottom of `docs/qa-checklist.md`:

```markdown

## Stage 1.B complete:
- nes-render: HalfblockRenderer (full + diff) — unit-tested
- glyph8-cli: --headless deterministic on nestest (integration test)
- glyph8-cli: interactive runloop runs nestest + bundled homebrew demo
- Manual: ESC restores terminal cleanly (alt screen exit, raw mode off, cursor visible)
- Manual: R resets the emulator mid-run
- Tests passing: cargo test --workspace
- Lints clean: cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 3: Commit**

```bash
git add README.md docs/qa-checklist.md
git commit -m "docs: README + QA section for Stage 1.B"
```

---

### Task 14: Final verification & beta tag

**Files:**
- (no source changes; verification only)

- [ ] **Step 1: Full test sweep**

Run: `cargo test --workspace`
Expected: all PASS — approximate numbers:
- `nes-core` 18 unit + 3 public_api
- `nes-tetanes-backend` 5
- `nes-render` 5 (encoding + diff + lifecycle)
- `glyph8-cli` 8 unit (input × 6, fps × 2) + 1 integration

Numbers will shift slightly as you implement; don't pin exactly. The point is "everything green".

- [ ] **Step 2: Lints**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: 0 warnings.

Run: `cargo fmt --all -- --check`
Expected: no diff.

- [ ] **Step 3: Manual end-to-end**

Run all five of these from the workspace root, in order. Each should behave as described:

```sh
# Headless determinism on nestest:
cargo run -p glyph8-cli -- --headless --frames=60 tests/roms/nestest.nes
# → 64-char hex hash, exit 0

# Headless on homebrew:
cargo run -p glyph8-cli -- --headless --frames=60 tests/roms/<demo>.nes
# → 64-char hex hash, exit 0

# Interactive on nestest (smoke):
cargo run -p glyph8-cli -- tests/roms/nestest.nes
# → terminal shows picture; press Esc; terminal restored cleanly

# Interactive on homebrew (the real beta validation):
cargo run -p glyph8-cli -- tests/roms/<demo>.nes
# → motion visible; press R then Esc; terminal restored cleanly

# Help text:
cargo run -p glyph8-cli -- --help
# → clap help; mentions --headless, --frames, ROM
```

- [ ] **Step 4: Print verification report**

```
Stage 1.B (CLI Beta) complete:
- glyph8 binary builds and installs
- Halfblock renderer with diff redraw — visible game on screen
- crossterm input (Mednafen keymap) — controls game
- --headless mode deterministic — CI-ready
- nestest + 1 homebrew demo bundled in tests/roms/
- README + qa-checklist updated
- All tests passing, lints clean
```

- [ ] **Step 5: Commit any leftover changes and tag**

```bash
git status
# If anything is uncommitted, commit it:
# git add -A && git commit -m "chore: stage 1.B final tidy"
git tag v0.1.0-beta.1 -m "Stage 1.B CLI beta"
```

(Don't push the tag unless the user asks; tagging is a local milestone for now.)

---

## Self-Review Notes

**Spec coverage** (against `2026-05-05-glyph8-stage-1b-renderer-cli-design.md`):

| Spec section | Covered by |
|---|---|
| §2 architecture (4-crate workspace) | Tasks 1, 5 |
| §3.1 Renderer trait | Task 1 |
| §3.2 HalfblockRenderer + diff | Tasks 2, 3, 4 |
| §3.3 Input keymap | Task 9 (with documented Tab→Select beta compromise) |
| §3.4 runloop | Task 10 |
| §3.5 headless | Task 7 |
| §3.6 CLI args | Task 5 |
| §4 data flow | Implicit in Task 10 |
| §5 error handling | Tasks 7, 10 (anyhow + Drop chain) |
| §6 testing strategy | Tasks 2, 3, 7, 8, 9, 14 |
| §7 ROM strategy | Tasks 6 (nestest), 12 (homebrew) |
| §8 dependency increments | Tasks 1, 5 |
| §10 completion criteria | Task 14 |

**Deviations from spec** (called out for the reviewer):

- **Tab → Select instead of RightShift.** crossterm without the Kitty protocol can't reliably distinguish RShift from LShift, and SHIFT alone has no `KeyCode`. Tab is unambiguous and rarely used by NES games. README documents this; 1.C upgrades to true RightShift via Kitty enhancement flags.
- **No tests for runloop.** A loop that owns stdin/stdout has no clean unit-test surface. Mitigation: each owned component (Input, FpsMeter, HalfblockRenderer, headless) is independently unit-tested; runloop is exercised by manual smoke in Tasks 10 + 14.
- **Status line via crossterm queue!**, not a separate trait method on `Renderer`. Spec §3.1 said "状态栏不进 trait" — implementation matches.

**Type / signature consistency check**:

- `Renderer::draw(&mut self, frame: &Frame) -> io::Result<()>` — used identically in Tasks 1, 2, 3, 10
- `Input::handle_event(&mut self, ev: &Event) -> Option<PollOutcome>` — defined Task 9, used Task 10
- `PollOutcome::{Continue, Reset, Quit}` — Task 9 defines all three; runloop matches all three
- `FpsMeter::tick(&mut self) -> f32` — Task 8 → used in Task 10
- `headless::run(rom_path: &Path, frames: u32) -> Result<()>` — Task 7 → matches main.rs dispatch in Task 10
- `runloop::run(rom_path: &Path) -> Result<()>` — Task 10 → matches main.rs dispatch in Task 10

**Out-of-scope items deferred to next plan**:

- Stage 1.C: Kitty keyboard protocol — release events, RightShift detection, true held-button input
- Stage 1.D: cpal-based audio — uses `backend.drain_audio()` already in place
- Stage 1.E: BrailleRenderer / AsciiRenderer + adaptive sizing
- Stage 1.F: ratatui status bar + pause modal
- CI: GitHub Actions yaml (deferred per spec; local test/clippy pass is the bar for Stage 1.B)
- `--render=` / `--backend=` CLI flags (need at least 2 options to be meaningful; defer to 1.E / Stage 2)
