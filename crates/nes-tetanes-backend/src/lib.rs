//! `EmulatorBackend` implementation backed by the `tetanes-core` crate.
//!
//! tetanes-core API verified against version 0.12.2 (last successful docs.rs build;
//! 0.13.0 and 0.14.x fail to build on docs.rs). Update this comment if bumped.
//!
//! ## Key API surface used (all on `ControlDeck`):
//!
//! - Constructor:         `ControlDeck::new() -> Self`
//! - Load ROM:            `load_rom<S: ToString, F: Read>(&mut self, name: S, rom: &mut F) -> Result<LoadedRom>`
//! - Step one frame:      `clock_frame(&mut self) -> Result<u64>`
//! - Frame buffer:        `frame_buffer(&mut self) -> &[u8]`
//!                         — RGBA, 4 bytes/pixel, 256×240 px (from `Ppu::WIDTH`/`Ppu::HEIGHT`)
//!                         — `frame_buffer_raw(&mut self) -> &[u16]` gives raw palette indices
//! - Audio samples:       `audio_samples(&self) -> &[f32]`  (pull, f32, interleaved stereo)
//! - Clear audio:         `clear_audio_samples(&mut self)`
//! - Set sample rate:     `set_sample_rate(&mut self, sample_rate: f32)`
//! - Joypad access:       `joypad_mut(&mut self, slot: Player) -> &mut Joypad`
//! - Set button:          `joypad.set_button(btn: JoypadBtn, pressed: bool)`
//!                         Buttons: A, B, TurboA, TurboB, Select, Start, Up, Down, Left, Right
//! - Reset:               `reset(&mut self, kind: ResetKind)`  (via `Reset` trait)
//!                         Variants: `ResetKind::Soft`, `ResetKind::Hard`
//! - Running check:       `is_running(&self) -> bool`

use nes_core::{ControllerState, EmulatorBackend, EmulatorError, Frame, RomInfo};

/// Stage-1 backend that delegates emulation to the `tetanes-core` crate.
pub struct TetanesBackend {
    // Fields filled in by Task 12.
}
