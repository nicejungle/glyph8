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
//!   (RGBA, 4 bytes/pixel, 256×240 px; `frame_buffer_raw` gives raw palette indices)
//! - Audio samples:       `audio_samples(&self) -> &[f32]`  (pull, f32, interleaved stereo)
//! - Clear audio:         `clear_audio_samples(&mut self)`
//! - Set sample rate:     `set_sample_rate(&mut self, sample_rate: f32)`
//! - Joypad access:       `joypad_mut(&mut self, slot: Player) -> &mut Joypad`
//! - Set button:          `joypad.set_button(btn: JoypadBtn, pressed: bool)`
//!   (Buttons: A, B, TurboA, TurboB, Select, Start, Up, Down, Left, Right)
//! - Reset:               `reset(&mut self, kind: ResetKind)` (via `Reset` trait)
//!   (Variants: `ResetKind::Soft`, `ResetKind::Hard`)
//! - Running check:       `is_running(&self) -> bool`

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
            deck: ControlDeck::new(),
            frame: Frame::default(),
            audio: Vec::new(),
            loaded: None,
        }
    }
}

impl Default for TetanesBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl EmulatorBackend for TetanesBackend {
    fn load_rom(&mut self, rom: &[u8]) -> Result<RomInfo, EmulatorError> {
        // Our parser produces RomInfo (the source of truth — fields like
        // mapper, mirroring, etc. are extracted from the iNES header by us,
        // not by tetanes).
        let info = nes_core::parse_header(rom)?;
        // Hand the bytes to tetanes via a Read impl (Cursor over the slice).
        let mut cursor = std::io::Cursor::new(rom);
        self.deck
            .load_rom("rom.nes", &mut cursor)
            .map_err(|e| EmulatorError::Backend(e.to_string()))?;
        self.loaded = Some(info);
        Ok(info)
    }

    fn step_frame(&mut self) {
        // Filled in by Task 13.
    }

    fn frame(&self) -> &Frame {
        &self.frame
    }

    fn submit_input(&mut self, _p1: ControllerState, _p2: ControllerState) {
        // Filled in by Task 14.
    }

    fn drain_audio(&mut self) -> &[f32] {
        &self.audio
    }

    fn reset(&mut self) {
        // Filled in by Task 16.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reuses the synthetic NROM helper concept from nes-core's test code.
    /// Re-implemented here because `pub(crate)` test helpers don't cross crate boundaries.
    fn minimal_nrom() -> Vec<u8> {
        let mut rom = Vec::with_capacity(16 + 16 * 1024 + 8 * 1024);
        rom.extend_from_slice(b"NES\x1A");
        rom.extend_from_slice(&[1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        rom.extend(std::iter::repeat_n(0u8, 16 * 1024));
        rom.extend(std::iter::repeat_n(0u8, 8 * 1024));
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
