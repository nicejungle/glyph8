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
        fn frame(&self) -> &Frame {
            &self.frame
        }
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
