# Glyph8 — Stage 0 + 1.A Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Cargo workspace skeleton, the `nes-core` abstraction crate (data types + iNES parser + `EmulatorBackend` trait), and a working `nes-tetanes-backend` that implements the trait by wrapping the `tetanes` crate. End state: `cargo test --workspace` passes, including an integration test that loads a synthesized NROM and steps a frame through the tetanes backend.

**Architecture:** Cargo workspace with two member crates initially. `nes-core` is a pure-Rust, dependency-light crate exposing `Frame`, `ControllerState`, `RomInfo`, `Mirroring`, `EmulatorError`, the iNES parser, and the `EmulatorBackend` trait. `nes-tetanes-backend` depends on `nes-core` + `tetanes` and provides a single struct `TetanesBackend` that adapts tetanes's API to our trait.

**Tech Stack:** Rust 2021, Cargo workspace, `thiserror` for errors, `tetanes` for the emulator core in stage 1, plain `#[cfg(test)]` unit tests.

---

## Spec Deviations

These are intentional simplifications from the spec, surfaced here for review:

1. **`Frame` stores RGB bytes (256·240·3 = 184,320 B), not palette indices.**
   The spec says `Frame { pixels: [u8; 256·240] }` of palette indices, with a separate `Palette` lookup in the renderer. tetanes outputs RGB directly and reverse-mapping RGB → index is fragile (depends on tetanes's exact palette). Keeping `Frame` as RGB lets both backends produce it the same way and removes the renderer's `&Palette` parameter.
   **Consequence**: "palette hot-swap" feature is deferred; in stage 2 the native backend can apply a configurable palette internally before producing RGB. The trade-off is acceptable for stage 0/1.A — palette swap was a "nice to have", not a hard requirement.
2. **`tetanes` crate API is verified at Task 11, not pre-baked into this plan.**
   The exact public API surface (struct names, method signatures, sample-rate accessor) varies across `tetanes` versions. Task 11 has a research step that calls out exactly what to look up; subsequent tasks adjust if needed.

---

## File Structure

```
glyph8/
├── Cargo.toml                              # workspace root
├── .gitignore
├── crates/
│   ├── nes-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                      # re-exports + crate docs
│   │       ├── frame.rs                    # Frame struct
│   │       ├── controller.rs               # ControllerState
│   │       ├── ines.rs                     # iNES header parser + RomInfo + Mirroring
│   │       ├── error.rs                    # EmulatorError
│   │       └── backend.rs                  # EmulatorBackend trait
│   └── nes-tetanes-backend/
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs                      # TetanesBackend
└── docs/superpowers/
    ├── specs/2026-05-05-glyph8-cli-nes-emulator-design.md  (already exists)
    └── plans/2026-05-05-glyph8-stage-0-and-1a.md           (this file)
```

Why this split:

- One file per concept in `nes-core` keeps each ≤ 100 LOC and makes diff review trivial.
- `ines.rs` co-locates `RomInfo`, `Mirroring`, and the parser because they only have meaning together.
- `nes-tetanes-backend` is a single file because it's just an adapter — splitting it adds noise.

---

## Conventions

- After every step that adds or modifies code, run `cargo fmt` before committing.
- All commits go on `main`. Commit message format: `<area>: <what changed>`. Examples: `core: add Frame struct`, `tetanes-backend: implement load_rom`.
- Each task ends with a commit. Do not batch tasks into one commit.

---

### Task 1: Workspace skeleton + .gitignore

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`

- [ ] **Step 1: Create the workspace `Cargo.toml`**

Create `Cargo.toml` at repo root with:

```toml
[workspace]
resolver = "2"
members = [
    "crates/nes-core",
    "crates/nes-tetanes-backend",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/<owner>/glyph8"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
```

- [ ] **Step 2: Create `.gitignore`**

```
/target
**/*.rs.bk
Cargo.lock.bak
.DS_Store
```

(Keep `Cargo.lock` tracked — this is a binary workspace.)

- [ ] **Step 3: Verify cargo can read the workspace**

Run: `cargo check --workspace`
Expected: errors about missing member crates (we haven't created them yet) — this confirms cargo is parsing the workspace. We will add the crates next.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml .gitignore
git commit -m "chore: workspace skeleton + gitignore"
```

---

### Task 2: nes-core crate scaffold

**Files:**
- Create: `crates/nes-core/Cargo.toml`
- Create: `crates/nes-core/src/lib.rs`

- [ ] **Step 1: Create `crates/nes-core/Cargo.toml`**

```toml
[package]
name = "nes-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
thiserror = "1"

[lints]
workspace = true
```

- [ ] **Step 2: Create `crates/nes-core/src/lib.rs`**

```rust
//! Core abstractions for the Glyph8 NES emulator.
//!
//! This crate defines the [`EmulatorBackend`] trait and the value types
//! ([`Frame`], [`ControllerState`], [`RomInfo`]) shared between any backend
//! implementation (e.g. `nes-tetanes-backend`, `nes-native`) and the CLI
//! frontend.
```

- [ ] **Step 3: Verify it builds**

Run: `cargo check -p nes-core`
Expected: clean build (no warnings, no errors). The `nes-tetanes-backend` will still error because it's listed in the workspace but doesn't exist yet — we'll fix that in Task 12.

- [ ] **Step 4: Temporarily remove `nes-tetanes-backend` from workspace members so we can compile**

In `Cargo.toml`, change:
```toml
members = [
    "crates/nes-core",
    "crates/nes-tetanes-backend",
]
```
to:
```toml
members = [
    "crates/nes-core",
]
```

We'll re-add it in Task 12.

- [ ] **Step 5: Verify clean build**

Run: `cargo build --workspace && cargo test --workspace`
Expected: PASS, 0 tests.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/nes-core
git commit -m "core: crate scaffold"
```

---

### Task 3: Frame struct (RGB) + tests

**Files:**
- Create: `crates/nes-core/src/frame.rs`
- Modify: `crates/nes-core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/nes-core/src/frame.rs`:

```rust
//! A single rendered NES frame, stored as packed RGB.

/// NES native horizontal resolution.
pub const WIDTH: usize = 256;
/// NES native vertical resolution.
pub const HEIGHT: usize = 240;
/// Bytes per pixel (R, G, B — no alpha).
pub const BPP: usize = 3;
/// Total bytes in a frame's pixel buffer.
pub const FRAME_BYTES: usize = WIDTH * HEIGHT * BPP;

/// One rendered frame as packed RGB pixels in row-major order.
///
/// Pixel `(x, y)` lives at byte offset `(y * WIDTH + x) * BPP`.
#[derive(Clone)]
pub struct Frame {
    pub pixels: Box<[u8; FRAME_BYTES]>,
}

impl Frame {
    pub fn new() -> Self {
        Self { pixels: Box::new([0; FRAME_BYTES]) }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: [u8; 3]) {
        let off = (y * WIDTH + x) * BPP;
        self.pixels[off..off + 3].copy_from_slice(&rgb);
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let off = (y * WIDTH + x) * BPP;
        [self.pixels[off], self.pixels[off + 1], self.pixels[off + 2]]
    }
}

impl Default for Frame {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_dimensions_are_nes_native() {
        assert_eq!(WIDTH, 256);
        assert_eq!(HEIGHT, 240);
        assert_eq!(FRAME_BYTES, 256 * 240 * 3);
    }

    #[test]
    fn new_frame_is_all_zeros() {
        let f = Frame::new();
        assert!(f.pixels.iter().all(|&b| b == 0));
    }

    #[test]
    fn set_then_get_round_trips() {
        let mut f = Frame::new();
        f.set_pixel(10, 20, [0xAA, 0xBB, 0xCC]);
        assert_eq!(f.get_pixel(10, 20), [0xAA, 0xBB, 0xCC]);
        // Untouched pixel stays zero.
        assert_eq!(f.get_pixel(0, 0), [0, 0, 0]);
    }

    #[test]
    fn set_pixel_corners() {
        let mut f = Frame::new();
        f.set_pixel(0, 0, [1, 2, 3]);
        f.set_pixel(WIDTH - 1, HEIGHT - 1, [4, 5, 6]);
        assert_eq!(f.get_pixel(0, 0), [1, 2, 3]);
        assert_eq!(f.get_pixel(WIDTH - 1, HEIGHT - 1), [4, 5, 6]);
    }
}
```

Wire it up in `crates/nes-core/src/lib.rs`:

```rust
//! Core abstractions for the Glyph8 NES emulator.

pub mod frame;

pub use frame::{Frame, BPP, FRAME_BYTES, HEIGHT, WIDTH};
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p nes-core --lib frame::`
Expected: 4 tests PASS.

- [ ] **Step 3: Commit**

```bash
cargo fmt
git add crates/nes-core
git commit -m "core: add Frame (RGB pixel buffer)"
```

---

### Task 4: ControllerState + tests

**Files:**
- Create: `crates/nes-core/src/controller.rs`
- Modify: `crates/nes-core/src/lib.rs`

- [ ] **Step 1: Write the implementation + tests**

Create `crates/nes-core/src/controller.rs`:

```rust
//! NES controller (Famicom standard pad) state.

/// Bit-packed state of one controller. `1` = pressed.
///
/// Bit layout matches the order the NES strobe protocol shifts out: A, B,
/// Select, Start, Up, Down, Left, Right.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct ControllerState(pub u8);

impl ControllerState {
    pub const A: u8      = 1 << 0;
    pub const B: u8      = 1 << 1;
    pub const SELECT: u8 = 1 << 2;
    pub const START: u8  = 1 << 3;
    pub const UP: u8     = 1 << 4;
    pub const DOWN: u8   = 1 << 5;
    pub const LEFT: u8   = 1 << 6;
    pub const RIGHT: u8  = 1 << 7;

    pub const fn empty() -> Self { Self(0) }

    pub fn pressed(self, mask: u8) -> bool { self.0 & mask != 0 }

    pub fn press(&mut self, mask: u8)  { self.0 |= mask; }
    pub fn release(&mut self, mask: u8) { self.0 &= !mask; }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_no_buttons_pressed() {
        let s = ControllerState::empty();
        for mask in [
            ControllerState::A, ControllerState::B,
            ControllerState::SELECT, ControllerState::START,
            ControllerState::UP, ControllerState::DOWN,
            ControllerState::LEFT, ControllerState::RIGHT,
        ] {
            assert!(!s.pressed(mask));
        }
    }

    #[test]
    fn press_then_release() {
        let mut s = ControllerState::empty();
        s.press(ControllerState::A);
        assert!(s.pressed(ControllerState::A));
        assert!(!s.pressed(ControllerState::B));
        s.release(ControllerState::A);
        assert!(!s.pressed(ControllerState::A));
    }

    #[test]
    fn bit_layout_is_strobe_order() {
        // A is LSB, Right is MSB — required to match NES $4016 read order.
        assert_eq!(ControllerState::A,     0b0000_0001);
        assert_eq!(ControllerState::B,     0b0000_0010);
        assert_eq!(ControllerState::SELECT, 0b0000_0100);
        assert_eq!(ControllerState::START, 0b0000_1000);
        assert_eq!(ControllerState::UP,    0b0001_0000);
        assert_eq!(ControllerState::DOWN,  0b0010_0000);
        assert_eq!(ControllerState::LEFT,  0b0100_0000);
        assert_eq!(ControllerState::RIGHT, 0b1000_0000);
    }
}
```

Update `crates/nes-core/src/lib.rs`:

```rust
//! Core abstractions for the Glyph8 NES emulator.

pub mod controller;
pub mod frame;

pub use controller::ControllerState;
pub use frame::{Frame, BPP, FRAME_BYTES, HEIGHT, WIDTH};
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p nes-core --lib controller::`
Expected: 3 tests PASS.

- [ ] **Step 3: Commit**

```bash
cargo fmt
git add crates/nes-core
git commit -m "core: add ControllerState"
```

---

### Task 5: EmulatorError

**Files:**
- Create: `crates/nes-core/src/error.rs`
- Modify: `crates/nes-core/src/lib.rs`

- [ ] **Step 1: Write the implementation + test**

Create `crates/nes-core/src/error.rs`:

```rust
//! Errors that can arise from any [`crate::EmulatorBackend`] operation.

#[derive(thiserror::Error, Debug)]
pub enum EmulatorError {
    #[error("invalid iNES header")]
    InvalidINesHeader,
    #[error("rom too small ({0} bytes)")]
    RomTooSmall(usize),
    #[error("unsupported mapper {0}")]
    UnsupportedMapper(u8),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("backend error: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_are_useful() {
        let e = EmulatorError::InvalidINesHeader;
        assert_eq!(e.to_string(), "invalid iNES header");

        let e = EmulatorError::UnsupportedMapper(7);
        assert_eq!(e.to_string(), "unsupported mapper 7");

        let e = EmulatorError::RomTooSmall(15);
        assert_eq!(e.to_string(), "rom too small (15 bytes)");
    }

    #[test]
    fn io_error_converts_via_from() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "x");
        let e: EmulatorError = io.into();
        assert!(matches!(e, EmulatorError::Io(_)));
    }
}
```

Update `crates/nes-core/src/lib.rs`:

```rust
//! Core abstractions for the Glyph8 NES emulator.

pub mod controller;
pub mod error;
pub mod frame;

pub use controller::ControllerState;
pub use error::EmulatorError;
pub use frame::{Frame, BPP, FRAME_BYTES, HEIGHT, WIDTH};
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p nes-core --lib error::`
Expected: 2 tests PASS.

- [ ] **Step 3: Commit**

```bash
cargo fmt
git add crates/nes-core
git commit -m "core: add EmulatorError"
```

---

### Task 6: iNES parser — happy path (valid NROM)

**Files:**
- Create: `crates/nes-core/src/ines.rs`
- Modify: `crates/nes-core/src/lib.rs`

This task and the next together build the iNES parser. Split because each test set can stand alone.

- [ ] **Step 1: Write the failing test**

Create `crates/nes-core/src/ines.rs`:

```rust
//! iNES (.nes) ROM header parser.
//!
//! See https://www.nesdev.org/wiki/INES for the format spec.

use crate::error::EmulatorError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RomInfo {
    pub mapper: u8,
    pub prg_rom_size: usize,
    pub chr_rom_size: usize,
    pub mirroring: Mirroring,
    pub has_battery: bool,
}

const HEADER_SIZE: usize = 16;
const MAGIC: &[u8; 4] = b"NES\x1A";
const PRG_BANK_SIZE: usize = 16 * 1024;
const CHR_BANK_SIZE: usize = 8 * 1024;

pub fn parse_header(rom: &[u8]) -> Result<RomInfo, EmulatorError> {
    if rom.len() < HEADER_SIZE {
        return Err(EmulatorError::RomTooSmall(rom.len()));
    }
    if &rom[0..4] != MAGIC {
        return Err(EmulatorError::InvalidINesHeader);
    }

    let prg_banks = rom[4] as usize;
    let chr_banks = rom[5] as usize;
    let flags6 = rom[6];
    let flags7 = rom[7];

    let prg_rom_size = prg_banks * PRG_BANK_SIZE;
    let chr_rom_size = chr_banks * CHR_BANK_SIZE;

    let mirroring = if flags6 & 0b0000_1000 != 0 {
        Mirroring::FourScreen
    } else if flags6 & 0b0000_0001 != 0 {
        Mirroring::Vertical
    } else {
        Mirroring::Horizontal
    };
    let has_battery = flags6 & 0b0000_0010 != 0;
    let mapper = (flags7 & 0b1111_0000) | (flags6 >> 4);

    let expected_min = HEADER_SIZE + prg_rom_size + chr_rom_size;
    if rom.len() < expected_min {
        return Err(EmulatorError::RomTooSmall(rom.len()));
    }

    Ok(RomInfo { mapper, prg_rom_size, chr_rom_size, mirroring, has_battery })
}

#[cfg(test)]
pub(crate) fn make_minimal_nrom() -> Vec<u8> {
    let mut rom = Vec::with_capacity(16 + 16 * 1024 + 8 * 1024);
    rom.extend_from_slice(b"NES\x1A");
    rom.push(1); // 1 × 16 KB PRG
    rom.push(1); // 1 × 8 KB CHR
    rom.push(0); // flags 6: mapper 0, horizontal mirror, no battery
    rom.push(0); // flags 7
    rom.extend(std::iter::repeat(0u8).take(8)); // padding
    rom.extend(std::iter::repeat(0u8).take(16 * 1024));
    rom.extend(std::iter::repeat(0u8).take(8 * 1024));
    rom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_nrom() {
        let rom = make_minimal_nrom();
        let info = parse_header(&rom).unwrap();
        assert_eq!(info.mapper, 0);
        assert_eq!(info.prg_rom_size, 16 * 1024);
        assert_eq!(info.chr_rom_size, 8 * 1024);
        assert_eq!(info.mirroring, Mirroring::Horizontal);
        assert!(!info.has_battery);
    }

    #[test]
    fn vertical_mirror_flag() {
        let mut rom = make_minimal_nrom();
        rom[6] |= 0b0000_0001;
        let info = parse_header(&rom).unwrap();
        assert_eq!(info.mirroring, Mirroring::Vertical);
    }

    #[test]
    fn four_screen_mirror_flag() {
        let mut rom = make_minimal_nrom();
        rom[6] |= 0b0000_1000;
        let info = parse_header(&rom).unwrap();
        assert_eq!(info.mirroring, Mirroring::FourScreen);
    }

    #[test]
    fn battery_flag() {
        let mut rom = make_minimal_nrom();
        rom[6] |= 0b0000_0010;
        let info = parse_header(&rom).unwrap();
        assert!(info.has_battery);
    }

    #[test]
    fn mapper_id_split_across_flags6_and_flags7() {
        let mut rom = make_minimal_nrom();
        // mapper 0x4A: low nybble in flags6 high nybble, high nybble in flags7 high nybble
        rom[6] = 0xA0;
        rom[7] = 0x40;
        let info = parse_header(&rom).unwrap();
        assert_eq!(info.mapper, 0x4A);
    }
}
```

Update `crates/nes-core/src/lib.rs`:

```rust
//! Core abstractions for the Glyph8 NES emulator.

pub mod controller;
pub mod error;
pub mod frame;
pub mod ines;

pub use controller::ControllerState;
pub use error::EmulatorError;
pub use frame::{Frame, BPP, FRAME_BYTES, HEIGHT, WIDTH};
pub use ines::{parse_header, Mirroring, RomInfo};
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p nes-core --lib ines::`
Expected: 5 tests PASS.

- [ ] **Step 3: Commit**

```bash
cargo fmt
git add crates/nes-core
git commit -m "core: iNES parser (happy path + flag variants)"
```

---

### Task 7: iNES parser — error cases

**Files:**
- Modify: `crates/nes-core/src/ines.rs`

- [ ] **Step 1: Write failing tests**

Append to the `tests` module in `crates/nes-core/src/ines.rs`:

```rust
    #[test]
    fn rejects_too_short_for_header() {
        let rom = b"NES\x1A".to_vec(); // 4 bytes, less than HEADER_SIZE
        let err = parse_header(&rom).unwrap_err();
        assert!(matches!(err, EmulatorError::RomTooSmall(4)));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut rom = make_minimal_nrom();
        rom[0] = b'X';
        let err = parse_header(&rom).unwrap_err();
        assert!(matches!(err, EmulatorError::InvalidINesHeader));
    }

    #[test]
    fn rejects_truncated_prg() {
        let mut rom = make_minimal_nrom();
        // claim 2 PRG banks (32 KB) but only ship 1
        rom[4] = 2;
        let err = parse_header(&rom).unwrap_err();
        assert!(matches!(err, EmulatorError::RomTooSmall(_)));
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p nes-core --lib ines::`
Expected: now 8 tests PASS (5 from Task 6 + 3 new).

- [ ] **Step 3: Commit**

```bash
git add crates/nes-core
git commit -m "core: iNES parser — error cases"
```

---

### Task 8: EmulatorBackend trait + mock backend test

**Files:**
- Create: `crates/nes-core/src/backend.rs`
- Modify: `crates/nes-core/src/lib.rs`

This task asserts the trait is *implementable* by writing a mock backend in the test module. If the mock compiles and the test passes, the trait is well-shaped.

- [ ] **Step 1: Write the trait + mock test**

Create `crates/nes-core/src/backend.rs`:

```rust
//! The [`EmulatorBackend`] trait — the abstraction the CLI frontend talks to.
//!
//! Both `nes-tetanes-backend` (stage 1) and `nes-native` (stage 2) implement this.

use crate::{ControllerState, EmulatorError, Frame, RomInfo};

pub trait EmulatorBackend: Send {
    fn load_rom(&mut self, rom: &[u8]) -> Result<RomInfo, EmulatorError>;
    fn step_frame(&mut self);
    fn frame(&self) -> &Frame;
    fn submit_input(&mut self, p1: ControllerState, p2: ControllerState);
    fn drain_audio(&mut self) -> &[f32];
    fn reset(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ines::{make_minimal_nrom, parse_header};

    /// A trivial in-memory backend used purely to verify the trait shape.
    /// It does no actual emulation.
    #[derive(Default)]
    struct MockBackend {
        frame: Frame,
        audio: Vec<f32>,
        loaded: Option<RomInfo>,
    }

    impl EmulatorBackend for MockBackend {
        fn load_rom(&mut self, rom: &[u8]) -> Result<RomInfo, EmulatorError> {
            let info = parse_header(rom)?;
            self.loaded = Some(info);
            Ok(info)
        }
        fn step_frame(&mut self) {
            self.audio.push(0.0);
        }
        fn frame(&self) -> &Frame { &self.frame }
        fn submit_input(&mut self, _p1: ControllerState, _p2: ControllerState) {}
        fn drain_audio(&mut self) -> &[f32] {
            // Return everything; clearing happens on next step.
            let slice = &self.audio[..];
            // SAFETY: we only need to expose then clear later. Simpler:
            // we just leave the buffer; real backends should clear.
            slice
        }
        fn reset(&mut self) {
            self.audio.clear();
            self.frame = Frame::default();
        }
    }

    #[test]
    fn mock_backend_implements_trait_and_loads_rom() {
        let mut be: Box<dyn EmulatorBackend> = Box::new(MockBackend::default());
        let rom = make_minimal_nrom();
        let info = be.load_rom(&rom).unwrap();
        assert_eq!(info.mapper, 0);
        be.submit_input(ControllerState::empty(), ControllerState::empty());
        be.step_frame();
        assert_eq!(be.frame().pixels.len(), crate::FRAME_BYTES);
        be.reset();
    }
}
```

Update `crates/nes-core/src/lib.rs`:

```rust
//! Core abstractions for the Glyph8 NES emulator.

pub mod backend;
pub mod controller;
pub mod error;
pub mod frame;
pub mod ines;

pub use backend::EmulatorBackend;
pub use controller::ControllerState;
pub use error::EmulatorError;
pub use frame::{Frame, BPP, FRAME_BYTES, HEIGHT, WIDTH};
pub use ines::{parse_header, Mirroring, RomInfo};
```

Also: the `make_minimal_nrom` helper introduced in Task 6 is `#[cfg(test)] pub(crate)`, so it's visible from the `backend::tests` module. Confirm this in the parser file (already done in Task 6).

- [ ] **Step 2: Run tests**

Run: `cargo test -p nes-core`
Expected: 13 tests PASS (4 frame + 3 controller + 2 error + 8 ines + 1 backend = adjust as needed; main signal is **all PASS, 0 FAIL**).

- [ ] **Step 3: Commit**

```bash
cargo fmt
git add crates/nes-core
git commit -m "core: EmulatorBackend trait + mock backend test"
```

---

### Task 9: Smoke test of `nes-core` public surface

**Files:**
- Create: `crates/nes-core/tests/public_api.rs`

A quick external-consumer test that catches "I forgot to re-export" mistakes — only public items can be referenced from this file.

- [ ] **Step 1: Write the test**

```rust
//! Verifies the public API surface of nes-core.

use nes_core::{
    parse_header, ControllerState, EmulatorBackend, EmulatorError, Frame, Mirroring,
    RomInfo, FRAME_BYTES, HEIGHT, WIDTH,
};

fn _trait_is_object_safe(_: Box<dyn EmulatorBackend>) {}

#[test]
fn dimensions_are_exported() {
    assert_eq!(WIDTH, 256);
    assert_eq!(HEIGHT, 240);
    assert_eq!(FRAME_BYTES, 256 * 240 * 3);
}

#[test]
fn types_can_be_constructed_externally() {
    let _f = Frame::default();
    let _c = ControllerState::empty();
    let _m = Mirroring::Horizontal;
    let info = RomInfo {
        mapper: 0,
        prg_rom_size: 16 * 1024,
        chr_rom_size: 8 * 1024,
        mirroring: Mirroring::Horizontal,
        has_battery: false,
    };
    assert_eq!(info.mapper, 0);
}

#[test]
fn parse_header_is_callable() {
    let err = parse_header(b"too short").unwrap_err();
    assert!(matches!(err, EmulatorError::RomTooSmall(_)));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p nes-core --test public_api`
Expected: 3 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/nes-core
git commit -m "core: public API smoke test"
```

---

### Task 10: Lint pass on nes-core

**Files:**
- (lint fixes if any)

- [ ] **Step 1: Run clippy with the workspace deny config**

Run: `cargo clippy -p nes-core --all-targets -- -D warnings`
Expected: 0 warnings, 0 errors. If anything trips, fix it (typical: missing `#[must_use]`, unused imports).

- [ ] **Step 2: Run fmt check**

Run: `cargo fmt --all -- --check`
Expected: 0 diffs.

- [ ] **Step 3: Commit if any changes were made**

```bash
git add -A
git diff --cached --quiet || git commit -m "core: clippy + fmt"
```

(The `||` clause is a no-op if there's nothing staged.)

---

### Task 11: Research tetanes API + crate scaffold

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `crates/nes-tetanes-backend/Cargo.toml`
- Create: `crates/nes-tetanes-backend/src/lib.rs`

This task has no TDD red-green; it's a research + scaffold step. The next task immediately starts TDD.

- [ ] **Step 1: Look up the current `tetanes` crate API**

The tetanes project ships either `tetanes` (binary + lib) or `tetanes-core` (lib-only) depending on the version. Verify which is current:

Run: `cargo search tetanes`
Expected: see published versions.

Then visit `https://docs.rs/tetanes-core` and `https://docs.rs/tetanes` to confirm:

1. The crate name to depend on (likely `tetanes` or `tetanes-core`).
2. The struct that drives the emulator (likely `ControlDeck` or `Nes`).
3. The methods we need:
   - load a ROM from bytes (likely `load_rom(name: &str, &mut impl Read)` or `load_rom_data(&[u8])`).
   - clock/step exactly one frame (likely `clock_frame()`).
   - read the rendered RGB frame buffer (likely `frame_buffer() -> &[u8]`).
   - submit joypad state (likely `joypad_mut(slot)` or `set_button`).
   - read audio samples produced this frame (likely `audio_samples() -> &[f32]`, or a callback).
   - reset (likely `reset(ResetKind::Soft)` or `power_cycle()`).

**Record the actual names found** in a short comment at the top of `lib.rs` (Step 3). The remaining tasks were written assuming `ControlDeck` with method names listed above; if the real names differ, adjust calls accordingly. The trait method names on **our** `EmulatorBackend` do NOT change — only the inside of the adapter.

- [ ] **Step 2: Re-add `nes-tetanes-backend` to the workspace members**

In root `Cargo.toml`:

```toml
members = [
    "crates/nes-core",
    "crates/nes-tetanes-backend",
]
```

- [ ] **Step 3: Create `crates/nes-tetanes-backend/Cargo.toml`**

Use whichever crate name you confirmed in Step 1. Example assuming `tetanes-core`:

```toml
[package]
name = "nes-tetanes-backend"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
nes-core = { path = "../nes-core" }
tetanes-core = "0.13"   # ← replace with the version found in Step 1

[lints]
workspace = true
```

- [ ] **Step 4: Create `crates/nes-tetanes-backend/src/lib.rs`**

```rust
//! `EmulatorBackend` implementation backed by the `tetanes` crate.
//!
//! tetanes API verified against version <X.Y.Z>: ControlDeck, methods used
//! are <list-here>. Update this comment if the dependency is bumped.

use nes_core::{ControllerState, EmulatorBackend, EmulatorError, Frame, RomInfo};

/// Stage-1 backend that delegates emulation to the `tetanes` crate.
pub struct TetanesBackend {
    // Fields filled in by Task 12.
}
```

- [ ] **Step 5: Verify it compiles (even with empty struct)**

Run: `cargo check -p nes-tetanes-backend`
Expected: clean compile (we haven't implemented `EmulatorBackend` yet, but the crate itself should build).

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add Cargo.toml crates/nes-tetanes-backend
git commit -m "tetanes-backend: crate scaffold"
```

---

### Task 12: TetanesBackend::new + load_rom

**Files:**
- Modify: `crates/nes-tetanes-backend/src/lib.rs`

**Read first**: the snippets below assume `tetanes-core::control_deck::ControlDeck` with a method `load_rom(name, &[u8]) -> Result<...>`. If the API you confirmed in Task 11 differs, **only** the adapter calls change — the test asserts on `RomInfo` from `nes-core`, which is unchanged.

- [ ] **Step 1: Write the failing test**

In `crates/nes-tetanes-backend/src/lib.rs`, append:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Reuses the synthetic NROM helper from nes-core's test code.
    /// We can't import it from cfg(test) of another crate, so duplicate
    /// the minimum here.
    fn minimal_nrom() -> Vec<u8> {
        let mut rom = Vec::with_capacity(16 + 16 * 1024 + 8 * 1024);
        rom.extend_from_slice(b"NES\x1A");
        rom.extend_from_slice(&[1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        rom.extend(std::iter::repeat(0u8).take(16 * 1024));
        rom.extend(std::iter::repeat(0u8).take(8 * 1024));
        rom
    }

    #[test]
    fn load_minimal_nrom_returns_rom_info() {
        let mut be = TetanesBackend::new();
        let info = be.load_rom(&minimal_nrom()).unwrap();
        assert_eq!(info.mapper, 0);
        assert_eq!(info.prg_rom_size, 16 * 1024);
        assert_eq!(info.chr_rom_size, 8 * 1024);
    }
}
```

- [ ] **Step 2: Run the test (expect failure)**

Run: `cargo test -p nes-tetanes-backend`
Expected: FAIL — `TetanesBackend::new` does not exist.

- [ ] **Step 3: Implement `new` and `load_rom`**

Replace the body of `crates/nes-tetanes-backend/src/lib.rs` with:

```rust
//! `EmulatorBackend` implementation backed by the `tetanes` crate.

use nes_core::{ControllerState, EmulatorBackend, EmulatorError, Frame, RomInfo};
use tetanes_core::control_deck::ControlDeck;

pub struct TetanesBackend {
    deck: ControlDeck,
    frame: Frame,
    audio: Vec<f32>,
    loaded: Option<RomInfo>,
}

impl TetanesBackend {
    pub fn new() -> Self {
        Self {
            deck: ControlDeck::default(),
            frame: Frame::default(),
            audio: Vec::new(),
            loaded: None,
        }
    }
}

impl Default for TetanesBackend {
    fn default() -> Self { Self::new() }
}

impl EmulatorBackend for TetanesBackend {
    fn load_rom(&mut self, rom: &[u8]) -> Result<RomInfo, EmulatorError> {
        // Our parser produces RomInfo (the source of truth).
        let info = nes_core::parse_header(rom)?;
        // Hand the bytes to tetanes. The exact method name may differ —
        // adjust per Task 11 research notes.
        self.deck
            .load_rom("rom.nes", &mut std::io::Cursor::new(rom))
            .map_err(|e| EmulatorError::Backend(e.to_string()))?;
        self.loaded = Some(info);
        Ok(info)
    }

    fn step_frame(&mut self) { /* Task 13 */ }
    fn frame(&self) -> &Frame { &self.frame }
    fn submit_input(&mut self, _p1: ControllerState, _p2: ControllerState) { /* Task 14 */ }
    fn drain_audio(&mut self) -> &[f32] { &self.audio }
    fn reset(&mut self) { /* Task 16 */ }
}
```

> **Note**: if `ControlDeck::load_rom` has a different signature (e.g. takes `&[u8]` directly, or returns a different error type), tweak the call. The visible behavior — "given valid iNES bytes, return `Ok(RomInfo)`" — is unchanged.

- [ ] **Step 4: Run the test (expect pass)**

Run: `cargo test -p nes-tetanes-backend test::load_minimal_nrom_returns_rom_info`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/nes-tetanes-backend
git commit -m "tetanes-backend: load_rom"
```

---

### Task 13: TetanesBackend::step_frame + frame copy-out

**Files:**
- Modify: `crates/nes-tetanes-backend/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` module:

```rust
    #[test]
    fn step_frame_produces_a_full_frame_buffer() {
        let mut be = TetanesBackend::new();
        be.load_rom(&minimal_nrom()).unwrap();
        be.step_frame();
        let f = be.frame();
        assert_eq!(f.pixels.len(), nes_core::FRAME_BYTES);
        // The synthetic NROM has no real CPU code, so we don't assert on
        // contents here — just that we got a full-sized buffer back.
    }
```

- [ ] **Step 2: Run the test (expect failure)**

Run: `cargo test -p nes-tetanes-backend test::step_frame_produces_a_full_frame_buffer`
Expected: FAIL or PASS-but-meaningless. If the buffer is still all zeros from `Frame::default()` and the test passes, that's fine — we're checking length, not content. If something panics, fix `step_frame` next.

- [ ] **Step 3: Implement `step_frame`**

Replace the `step_frame` and `frame` methods in `impl EmulatorBackend`:

```rust
    fn step_frame(&mut self) {
        // Clock tetanes for one full frame.
        // The exact method name may be `clock_frame` or `frame`; verify in Task 11.
        let _ = self.deck.clock_frame();

        // Copy tetanes's RGB pixel buffer into our Frame.
        // tetanes's frame_buffer() typically returns &[u8] of length 256*240*3
        // in the same RGB order we use. Confirm in Task 11.
        let src = self.deck.frame_buffer();
        debug_assert_eq!(src.len(), nes_core::FRAME_BYTES,
            "tetanes frame buffer length mismatch (expected {}, got {})",
            nes_core::FRAME_BYTES, src.len());
        self.frame.pixels.copy_from_slice(&src[..nes_core::FRAME_BYTES]);

        // Drain audio for this frame into our buffer (replaces previous frame's).
        self.audio.clear();
        // Some tetanes versions expose audio_samples() returning &[f32]; others
        // require a callback set on the deck. If callback-based, set it up
        // in Task 11/12 by storing samples in an Arc<Mutex<Vec<f32>>>.
        self.audio.extend_from_slice(self.deck.audio_samples());
    }
```

> **API uncertainty note**: if tetanes uses an audio callback rather than a pull API, set up the callback in `new()` to write into an internal buffer, then drain that buffer here.

- [ ] **Step 4: Run the test (expect pass)**

Run: `cargo test -p nes-tetanes-backend`
Expected: 2 tests PASS (load + step_frame).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/nes-tetanes-backend
git commit -m "tetanes-backend: step_frame + frame buffer copy"
```

---

### Task 14: TetanesBackend::submit_input

**Files:**
- Modify: `crates/nes-tetanes-backend/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` module:

```rust
    #[test]
    fn submit_input_does_not_panic_for_either_player() {
        let mut be = TetanesBackend::new();
        be.load_rom(&minimal_nrom()).unwrap();

        let mut p1 = ControllerState::empty();
        p1.press(ControllerState::A);
        p1.press(ControllerState::START);
        let p2 = ControllerState::empty();
        be.submit_input(p1, p2);
        be.step_frame();
        // No assertions on emulator state — synthetic NROM has no logic.
        // We're proving the call path doesn't panic and the bits map.
    }
```

- [ ] **Step 2: Run the test (expect failure)**

Run: `cargo test -p nes-tetanes-backend test::submit_input_does_not_panic_for_either_player`
Expected: PASS trivially (the empty `submit_input` doesn't panic). That's fine — we're upgrading the impl next so it actually wires bits through.

- [ ] **Step 3: Implement `submit_input` properly**

Replace `submit_input` in the `impl EmulatorBackend` block:

```rust
    fn submit_input(&mut self, p1: ControllerState, p2: ControllerState) {
        // Map our 8-bit ControllerState to tetanes's joypad. tetanes typically
        // exposes `joypad_mut(Slot::One)` returning a struct with per-button
        // setters or a `set_buttons(u8)` method. Confirm in Task 11.
        Self::apply_joypad(&mut self.deck, tetanes_core::input::Slot::One, p1);
        Self::apply_joypad(&mut self.deck, tetanes_core::input::Slot::Two, p2);
    }
```

Add a helper above the trait impl:

```rust
impl TetanesBackend {
    fn apply_joypad(deck: &mut ControlDeck, slot: tetanes_core::input::Slot, s: ControllerState) {
        // Pseudo-code — replace with the verified tetanes API.
        let pad = deck.joypad_mut(slot);
        pad.a      = s.pressed(ControllerState::A);
        pad.b      = s.pressed(ControllerState::B);
        pad.select = s.pressed(ControllerState::SELECT);
        pad.start  = s.pressed(ControllerState::START);
        pad.up     = s.pressed(ControllerState::UP);
        pad.down   = s.pressed(ControllerState::DOWN);
        pad.left   = s.pressed(ControllerState::LEFT);
        pad.right  = s.pressed(ControllerState::RIGHT);
    }
}
```

> If tetanes only takes a packed `u8`, the helper collapses to `deck.set_joypad(slot, s.0)`. Same observable behavior either way; tests below are agnostic.

- [ ] **Step 4: Run the test (expect pass)**

Run: `cargo test -p nes-tetanes-backend`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/nes-tetanes-backend
git commit -m "tetanes-backend: wire ControllerState to tetanes joypad"
```

---

### Task 15: TetanesBackend::drain_audio

**Files:**
- Modify: `crates/nes-tetanes-backend/src/lib.rs`

`drain_audio` already returns `&self.audio` (filled by `step_frame` in Task 13). This task adds a behavior assertion: after each `step_frame`, `drain_audio` returns a *non-empty* sample slice (synthetic NROM still produces silence samples — they're just zeros — but the buffer length should be > 0).

- [ ] **Step 1: Write the failing test**

Append to the `tests` module:

```rust
    #[test]
    fn drain_audio_yields_samples_after_frame() {
        let mut be = TetanesBackend::new();
        be.load_rom(&minimal_nrom()).unwrap();
        be.step_frame();
        let samples = be.drain_audio();
        assert!(!samples.is_empty(),
            "expected at least one audio sample per frame, got 0");
    }
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p nes-tetanes-backend test::drain_audio_yields_samples_after_frame`
Expected:
- PASS if tetanes's `audio_samples()` (or callback) is producing samples — implementation in Task 13 already drained into `self.audio`.
- FAIL if Task 13's audio path was a no-op stub; in that case revisit Task 13's tetanes audio path. The fix lives there, not in a new code path here.

- [ ] **Step 3: If needed, fix the audio path in `step_frame`**

If tetanes uses a callback instead of a pull API:
1. In `TetanesBackend::new()`, build `let buf = Arc::new(Mutex::new(Vec::<f32>::new()));`.
2. Register a callback on the `ControlDeck` that pushes into `buf.lock().unwrap()`.
3. In `step_frame`, drain `buf.lock().unwrap().drain(..)` into `self.audio`.
4. In `drain_audio`, return `&self.audio`. The trait contract — "samples produced this frame" — is preserved.

- [ ] **Step 4: Run all tests**

Run: `cargo test -p nes-tetanes-backend`
Expected: 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/nes-tetanes-backend
git commit -m "tetanes-backend: drain_audio yields per-frame samples"
```

---

### Task 16: TetanesBackend::reset

**Files:**
- Modify: `crates/nes-tetanes-backend/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` module:

```rust
    #[test]
    fn reset_clears_audio_and_keeps_rom_loaded() {
        let mut be = TetanesBackend::new();
        be.load_rom(&minimal_nrom()).unwrap();
        be.step_frame();
        assert!(!be.drain_audio().is_empty());
        be.reset();
        // After reset, the audio buffer from previous frames is gone.
        assert!(be.drain_audio().is_empty());
        // Re-stepping should still work (ROM stays loaded).
        be.step_frame();
        assert!(!be.drain_audio().is_empty());
    }
```

- [ ] **Step 2: Run the test (expect failure)**

Run: `cargo test -p nes-tetanes-backend test::reset_clears_audio_and_keeps_rom_loaded`
Expected: FAIL — `reset()` is a no-op.

- [ ] **Step 3: Implement `reset`**

Replace `reset` in the trait impl:

```rust
    fn reset(&mut self) {
        // Soft reset (equivalent to pressing the NES reset button).
        // tetanes API may be `reset(ResetKind::Soft)` or `power_cycle()`.
        // We want soft (keeps RAM, replays ROM): use the soft variant.
        self.deck.reset(tetanes_core::common::ResetKind::Soft);
        self.audio.clear();
        // Keep `self.frame` as-is until the next step_frame; some callers
        // may render once more between reset and step.
    }
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p nes-tetanes-backend test::reset_clears_audio_and_keeps_rom_loaded`
Expected: PASS.

- [ ] **Step 5: Run the full backend suite**

Run: `cargo test -p nes-tetanes-backend`
Expected: 5 tests PASS.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add crates/nes-tetanes-backend
git commit -m "tetanes-backend: implement reset"
```

---

### Task 17: Workspace lints + final integration check

**Files:**
- (no source changes expected unless lints trip)

- [ ] **Step 1: Clippy across the whole workspace**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: 0 warnings.

If anything trips: fix the warning where it lives. Common ones in this code:
- `clippy::needless_pass_by_value` on `submit_input`'s `ControllerState` — `ControllerState` is `Copy`, so this is a deliberate ergonomic choice; allow with `#[allow(clippy::needless_pass_by_value)]` on the method if necessary.
- `clippy::large_enum_variant` on `EmulatorError` — the `Io(io::Error)` variant is the biggest; harmless. Allow on the enum if it trips.

- [ ] **Step 2: fmt**

Run: `cargo fmt --all -- --check`
Expected: no diff.

- [ ] **Step 3: Full test run**

Run: `cargo test --workspace`
Expected: all tests PASS (≈ 16 tests across both crates).

- [ ] **Step 4: Commit any lint fixes**

```bash
git add -A
git diff --cached --quiet || git commit -m "chore: workspace clippy + fmt pass"
```

- [ ] **Step 5: Verification report**

Print to console (or save to `docs/qa-checklist.md` if it doesn't exist yet, append otherwise):

```
Stage 0 + 1.A complete:
- Workspace builds clean
- nes-core: Frame, ControllerState, EmulatorError, iNES parser, EmulatorBackend trait — all unit-tested
- nes-tetanes-backend: TetanesBackend implements EmulatorBackend
- Integration: synthetic NROM loads + steps + produces audio + resets
- Tests passing: cargo test --workspace
- Lints clean: cargo clippy --workspace --all-targets -- -D warnings
```

---

## Self-Review Notes

**Spec coverage** (against `2026-05-05-glyph8-cli-nes-emulator-design.md`):

| Spec section | Covered by |
|---|---|
| §3 workspace layout (`nes-core`, `nes-tetanes-backend`) | Tasks 1, 2, 11 |
| §4 Frame / ControllerState / RomInfo / EmulatorError | Tasks 3, 4, 5, 6 |
| §4 EmulatorBackend trait | Task 8 |
| §4 iNES parser | Tasks 6, 7 |
| §4 thiserror for errors | Task 5 |
| §8 (stage 2 internals) | Out of scope — separate plan |
| §5–§7 (renderer / input / audio crates) | Out of scope — separate plan (stage 1.B–1.F) |

**Deviations from spec** (reiterated here for the reviewer):

- Frame is RGB rather than palette indices (rationale at top of plan).
- Renderer no longer takes a `Palette` parameter (consequence of above).

**Type consistency check**: `Frame::pixels`, `ControllerState::A..RIGHT`, `RomInfo` field names, and `EmulatorError` variants are referenced consistently in Tasks 3–16.

**Out-of-scope items deferred to next plan**:

- `nes-render` (halfblock / braille / ascii)
- `nes-audio` (cpal)
- `nes-input` (crossterm)
- `glyph8-cli` binary
- Stage 2 milestones (CPU / PPU / APU / mappers)

The next plan (stage 1.B onward) will start where this one ends: `cargo test --workspace` green, `TetanesBackend` ready to be driven by a CLI front-end.
