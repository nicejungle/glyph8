//! Halfblock renderer — encodes NES frames as ANSI halfblock characters (▀, U+2580).
//!
//! Each terminal cell covers 2 vertical NES pixels: fg = top pixel, bg = bottom pixel.

use std::io::{self, Write};

use nes_core::{Frame, BPP, HEIGHT, WIDTH};

/// A NES frame rendered as halfblock characters (▀, U+2580).
/// Each terminal cell encodes 2 vertical NES pixels: fg = top, bg = bottom.
pub struct HalfblockRenderer<W: Write> {
    out: W,
    #[allow(dead_code)] // used in Task 4 terminal lifecycle
    manage_terminal: bool,
}

impl<W: Write> HalfblockRenderer<W> {
    /// For tests / non-terminal sinks. `enter`/`leave` will not touch terminal state.
    pub fn with_writer(out: W) -> Self {
        Self {
            out,
            manage_terminal: false,
        }
    }
}

impl<W: Write> crate::Renderer for HalfblockRenderer<W> {
    fn enter(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn draw(&mut self, frame: &Frame) -> io::Result<()> {
        debug_assert_eq!(frame.pixels.len(), WIDTH * HEIGHT * BPP);

        let row_pairs = HEIGHT / 2;
        for ry in 0..row_pairs {
            // Cursor to row ry+1, col 1 (1-indexed in ANSI).
            write!(self.out, "\x1b[{};1H", ry + 1)?;
            for x in 0..WIDTH {
                // Top pixel: row 2*ry, col x. Bottom: row 2*ry+1, col x.
                let top = (2 * ry * WIDTH + x) * BPP;
                let bot = ((2 * ry + 1) * WIDTH + x) * BPP;
                let (tr, tg, tb) = (
                    frame.pixels[top],
                    frame.pixels[top + 1],
                    frame.pixels[top + 2],
                );
                let (br, bg, bb) = (
                    frame.pixels[bot],
                    frame.pixels[bot + 1],
                    frame.pixels[bot + 2],
                );
                // SGR: foreground 24-bit color, then background 24-bit color, then ▀.
                write!(
                    self.out,
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                    tr, tg, tb, br, bg, bb
                )?;
            }
        }
        // Reset attributes at end of frame.
        write!(self.out, "\x1b[0m")?;
        self.out.flush()?;
        Ok(())
    }

    fn leave(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Renderer;

    #[test]
    fn draw_emits_one_halfblock_char_per_cell() {
        let mut buf: Vec<u8> = Vec::new();
        let mut r = HalfblockRenderer::with_writer(&mut buf);
        let frame = Frame::default();
        r.draw(&frame).unwrap();
        // 256 cols × 120 row-pairs = 30,720 cells, each containing one ▀ (3 UTF-8 bytes).
        let halfblocks = buf.windows(3).filter(|w| *w == "▀".as_bytes()).count();
        assert_eq!(halfblocks, WIDTH * HEIGHT / 2);
    }

    #[test]
    fn draw_includes_24bit_color_escape_for_each_pixel() {
        // A frame with one bright-red top pixel at (0,0) should produce an SGR 38;2;255;0;0 escape.
        let mut buf: Vec<u8> = Vec::new();
        let mut r = HalfblockRenderer::with_writer(&mut buf);
        let mut frame = Frame::default();
        // Frame is RGB; pixel (0,0) → bytes [0..3]
        frame.pixels[0] = 255;
        frame.pixels[1] = 0;
        frame.pixels[2] = 0;
        r.draw(&frame).unwrap();
        let s = String::from_utf8_lossy(&buf);
        assert!(
            s.contains("38;2;255;0;0"),
            "expected fg escape for red pixel, got: {}",
            &s[..s.len().min(200)]
        );
    }
}
