//! Core abstractions for the Glyph8 NES emulator.

pub mod controller;
pub mod frame;

pub use controller::ControllerState;
pub use frame::{Frame, BPP, FRAME_BYTES, HEIGHT, WIDTH};
