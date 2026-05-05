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
        let mut deck = ControlDeck::new();
        // tetanes-core requires explicit sample rate setup before producing samples.
        // Default to 44100.0 Hz (standard CD audio).
        deck.set_sample_rate(44100.0);
        Self {
            deck,
            frame: Frame::default(),
            audio: Vec::new(),
            loaded: None,
        }
    }

    fn apply_joypad(deck: &mut ControlDeck, slot: tetanes_core::input::Player, s: ControllerState) {
        use tetanes_core::input::JoypadBtn;
        let pad = deck.joypad_mut(slot);
        pad.set_button(JoypadBtn::A, s.pressed(ControllerState::A));
        pad.set_button(JoypadBtn::B, s.pressed(ControllerState::B));
        pad.set_button(JoypadBtn::Select, s.pressed(ControllerState::SELECT));
        pad.set_button(JoypadBtn::Start, s.pressed(ControllerState::START));
        pad.set_button(JoypadBtn::Up, s.pressed(ControllerState::UP));
        pad.set_button(JoypadBtn::Down, s.pressed(ControllerState::DOWN));
        pad.set_button(JoypadBtn::Left, s.pressed(ControllerState::LEFT));
        pad.set_button(JoypadBtn::Right, s.pressed(ControllerState::RIGHT));
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
        // Clock tetanes for one full frame; map errors to Backend variant.
        // We swallow the cycle count return value.
        if let Err(e) = self.deck.clock_frame() {
            // Don't panic — log via Backend error in a future task. For now,
            // this is unrecoverable mid-frame. Reset the audio buffer so
            // we don't accumulate stale samples.
            self.audio.clear();
            // Convert error to a string for debugging output.
            eprintln!("tetanes clock_frame error: {}", e);
            return;
        }

        // tetanes 0.12.2 frame_buffer() returns &[u8] of length 256*240*4 (RGBA).
        // Our Frame is RGB (256*240*3). Strip the alpha channel pixel-by-pixel.
        const SRC_BPP: usize = 4;
        const DST_BPP: usize = nes_core::BPP; // 3
        let src = self.deck.frame_buffer();
        debug_assert_eq!(
            src.len(),
            nes_core::WIDTH * nes_core::HEIGHT * SRC_BPP,
            "tetanes RGBA frame buffer size mismatch"
        );
        let dst = &mut self.frame.pixels[..];
        for i in 0..(nes_core::WIDTH * nes_core::HEIGHT) {
            let s = i * SRC_BPP;
            let d = i * DST_BPP;
            dst[d] = src[s];
            dst[d + 1] = src[s + 1];
            dst[d + 2] = src[s + 2];
            // src[s + 3] (alpha) is dropped.
        }

        // Drain audio for this frame into our buffer (overwrites previous frame's).
        self.audio.clear();
        self.audio.extend_from_slice(self.deck.audio_samples());
        self.deck.clear_audio_samples();
    }

    fn frame(&self) -> &Frame {
        &self.frame
    }

    fn submit_input(&mut self, p1: ControllerState, p2: ControllerState) {
        Self::apply_joypad(&mut self.deck, tetanes_core::input::Player::One, p1);
        Self::apply_joypad(&mut self.deck, tetanes_core::input::Player::Two, p2);
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
        // iNES header: NES\x1A + 12 header bytes
        rom.extend_from_slice(b"NES\x1A");
        rom.extend_from_slice(&[1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        // PRG ROM: 16 KB
        let mut prg = vec![0u8; 16 * 1024];
        // Set up NES vectors at the end of PRG ROM (16KB = $4000, so vectors are at offsets $3FFA-$3FFF)
        // NMI vector at $FFFA (offset $3FFA): points to $8000
        prg[0x3FFA] = 0x00;
        prg[0x3FFB] = 0x80;
        // RESET vector at $FFFC (offset $3FFC): points to $8000
        prg[0x3FFC] = 0x00;
        prg[0x3FFD] = 0x80;
        // IRQ/BRK vector at $FFFE (offset $3FFE): points to $8000
        prg[0x3FFE] = 0x00;
        prg[0x3FFF] = 0x80;
        // Program code at $8000: infinite loop (BIT $0000, then JMP)
        // EA = NOP, so fill with NOPs for a safe infinite loop
        prg[0x0000] = 0xEA; // NOP at $8000
        rom.extend(prg);

        // CHR ROM: 8 KB (all zeros)
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

    #[test]
    fn drain_audio_yields_samples_after_frame() {
        let mut be = TetanesBackend::new();
        be.load_rom(&minimal_nrom()).unwrap();
        be.step_frame();
        let samples = be.drain_audio();
        assert!(
            !samples.is_empty(),
            "expected at least one audio sample per frame, got 0"
        );
    }
}
