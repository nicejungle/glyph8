//! Core abstractions for the Glyph8 NES emulator.

pub mod controller;
pub mod error;
pub mod frame;

pub use controller::ControllerState;
pub use error::EmulatorError;
pub use frame::{Frame, BPP, FRAME_BYTES, HEIGHT, WIDTH};
