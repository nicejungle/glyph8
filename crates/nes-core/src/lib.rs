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
