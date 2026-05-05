//! The [`EmulatorBackend`] trait — the abstraction the CLI frontend talks to.
//!
//! Both `nes-tetanes-backend` (stage 1) and `nes-native` (stage 2) implement this.

use crate::{ControllerState, EmulatorError, Frame, RomInfo};

pub trait EmulatorBackend: Send {
    fn load_rom(&mut self, rom: &[u8]) -> Result<RomInfo, EmulatorError>;
    /// Advances the emulator by exactly one frame.
    ///
    /// On error, the backend's internal state is left undefined; callers
    /// should typically [`reset`] before calling `step_frame` again.
    ///
    /// [`reset`]: Self::reset
    fn step_frame(&mut self) -> Result<(), EmulatorError>;
    fn frame(&self) -> &Frame;
    fn submit_input(&mut self, p1: ControllerState, p2: ControllerState);
    /// Returns audio samples produced during the most recent [`step_frame`].
    ///
    /// The slice is overwritten by each call to [`step_frame`]; calling
    /// `drain_audio` multiple times between frames returns the same samples.
    /// Implementations are expected to clear the buffer at the start of the
    /// next [`step_frame`], not at the end of `drain_audio`.
    ///
    /// [`step_frame`]: Self::step_frame
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
        fn step_frame(&mut self) -> Result<(), EmulatorError> {
            self.audio.push(0.0);
            Ok(())
        }
        fn frame(&self) -> &Frame {
            &self.frame
        }
        fn submit_input(&mut self, _p1: ControllerState, _p2: ControllerState) {}
        fn drain_audio(&mut self) -> &[f32] {
            // This mock accumulates samples without draining; real backends should clear.
            &self.audio[..]
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
        be.step_frame().unwrap();
        assert_eq!(be.frame().pixels.len(), crate::FRAME_BYTES);
        be.reset();
    }
}
