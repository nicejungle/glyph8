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
