//! Halfblock renderer — encodes NES frames as ANSI halfblock characters (▀, U+2580).
//!
//! Each terminal cell covers 2 vertical NES pixels: fg = top pixel, bg = bottom pixel.

use std::io::{self, Write};

use nes_core::{Frame, BPP, HEIGHT, WIDTH};

type Cell = ((u8, u8, u8), (u8, u8, u8)); // (fg=top, bg=bottom)

/// A NES frame rendered as halfblock characters (▀, U+2580).
/// Each terminal cell encodes 2 vertical NES pixels: fg = top, bg = bottom.
pub struct HalfblockRenderer<W: Write> {
    out: W,
    /// Only `for_stdout()` sets this true; setting it true on a non-terminal writer
    /// will smash the calling process's terminal on `enter`.
    manage_terminal: bool,
    /// Previous frame's cells, indexed by (row_pair * WIDTH + x).
    /// `None` until first draw — first draw paints everything.
    prev: Option<Vec<Cell>>,
}

impl<W: Write> HalfblockRenderer<W> {
    /// For tests / non-terminal sinks. `enter`/`leave` will not touch terminal state.
    pub fn with_writer(out: W) -> Self {
        Self {
            out,
            manage_terminal: false,
            prev: None,
        }
    }
}

impl HalfblockRenderer<io::Stdout> {
    /// Constructor for production use: owns stdout, toggles terminal state
    /// on `enter`/`leave` (raw mode, alt screen, cursor hide/show).
    pub fn for_stdout() -> Self {
        Self {
            out: io::stdout(),
            manage_terminal: true,
            prev: None,
        }
    }
}

impl<W: Write> crate::Renderer for HalfblockRenderer<W> {
    fn enter(&mut self) -> io::Result<()> {
        if self.manage_terminal {
            crossterm::terminal::enable_raw_mode()?;
            crossterm::execute!(
                self.out,
                crossterm::terminal::EnterAlternateScreen,
                crossterm::cursor::Hide,
            )?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &Frame) -> io::Result<()> {
        let row_pairs = HEIGHT / 2;
        let cell_count = row_pairs * WIDTH;
        let mut current: Vec<Cell> = Vec::with_capacity(cell_count);
        for ry in 0..row_pairs {
            for x in 0..WIDTH {
                let top = (2 * ry * WIDTH + x) * BPP;
                let bot = ((2 * ry + 1) * WIDTH + x) * BPP;
                current.push((
                    (
                        frame.pixels[top],
                        frame.pixels[top + 1],
                        frame.pixels[top + 2],
                    ),
                    (
                        frame.pixels[bot],
                        frame.pixels[bot + 1],
                        frame.pixels[bot + 2],
                    ),
                ));
            }
        }

        let prev_ref = self.prev.as_deref();
        for ry in 0..row_pairs {
            // Track whether the cursor is currently at the start of run we want to draw.
            // We only emit a cursor-position escape when we hit a changed cell after a skip.
            let mut last_drawn_x: Option<usize> = None;
            for x in 0..WIDTH {
                let idx = ry * WIDTH + x;
                let cur = current[idx];
                let changed = match prev_ref {
                    None => true, // first draw paints everything
                    Some(p) => p[idx] != cur,
                };
                if !changed {
                    last_drawn_x = None;
                    continue;
                }
                // Position cursor only when we just resumed after skipping.
                let need_cursor = x == 0 || last_drawn_x != Some(x - 1);
                if need_cursor {
                    write!(self.out, "\x1b[{};{}H", ry + 1, x + 1)?;
                }
                let ((tr, tg, tb), (br, bg, bb)) = cur;
                write!(
                    self.out,
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                    tr, tg, tb, br, bg, bb
                )?;
                last_drawn_x = Some(x);
            }
        }
        write!(self.out, "\x1b[0m")?;
        self.out.flush()?;
        self.prev = Some(current);
        Ok(())
    }

    fn leave(&mut self) -> io::Result<()> {
        if self.manage_terminal {
            // Best-effort: try to restore everything even if some steps fail.
            let _ = crossterm::execute!(
                self.out,
                crossterm::cursor::Show,
                crossterm::terminal::LeaveAlternateScreen,
            );
            let _ = crossterm::terminal::disable_raw_mode();
            // Mark it done so a subsequent leave (e.g. via Drop) doesn't double-toggle.
            self.manage_terminal = false;
        }
        Ok(())
    }
}

impl<W: Write> Drop for HalfblockRenderer<W> {
    fn drop(&mut self) {
        // Best-effort terminal restoration on panic / unwind.
        let _ = <Self as crate::Renderer>::leave(self);
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
        drop(r); // release mutable borrow so we can read buf
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
        drop(r); // release mutable borrow so we can read buf
        let s = String::from_utf8_lossy(&buf);
        assert!(
            s.contains("38;2;255;0;0"),
            "expected fg escape for red pixel, got: {}",
            &s[..s.len().min(200)]
        );
    }

    #[test]
    fn bottom_pixel_maps_to_bg_color() {
        let mut buf: Vec<u8> = Vec::new();
        let mut r = HalfblockRenderer::with_writer(&mut buf);
        let mut frame = Frame::default();
        // Row 1 (second pixel row) = bottom half of the first terminal row.
        frame.set_pixel(0, 1, [0, 0, 255]);
        r.draw(&frame).unwrap();
        drop(r); // release mutable borrow so we can read buf
        let s = String::from_utf8_lossy(&buf);
        assert!(
            s.contains("48;2;0;0;255"),
            "expected bg escape for blue bottom pixel"
        );
    }

    #[test]
    fn second_draw_of_identical_frame_is_minimal() {
        let mut r = HalfblockRenderer::with_writer(Vec::<u8>::new());
        let frame = Frame::default();
        r.draw(&frame).unwrap();
        let bytes_first = r.out.len();
        r.out.clear();
        r.draw(&frame).unwrap();
        let bytes_second = r.out.len();
        // After Task 3, redrawing the same frame should emit < 5% the bytes
        // (only the SGR reset + maybe a cursor home), no per-cell escapes.
        assert!(
            bytes_second * 20 < bytes_first,
            "diff redraw too large: first={}, second={}",
            bytes_first,
            bytes_second
        );
    }

    #[test]
    fn diff_emits_changed_cell_only() {
        let mut r = HalfblockRenderer::with_writer(Vec::<u8>::new());
        let mut frame = Frame::default();
        r.draw(&frame).unwrap();
        r.out.clear();
        // Flip one top pixel to red.
        frame.pixels[0] = 255;
        r.draw(&frame).unwrap();
        let s = String::from_utf8_lossy(&r.out);
        assert!(
            s.contains("38;2;255;0;0"),
            "expected the changed cell's red fg in the diff output"
        );
        // The number of ▀ chars should be 1 (only the changed cell).
        let halfblocks = r.out.windows(3).filter(|w| *w == "▀".as_bytes()).count();
        assert_eq!(halfblocks, 1);
    }

    #[test]
    fn enter_leave_are_noop_for_writer_sink() {
        // Sanity: the test constructor must NOT actually toggle terminal state.
        let mut r = HalfblockRenderer::with_writer(Vec::<u8>::new());
        r.enter().unwrap();
        r.leave().unwrap();
        // Buffer should still be empty (no escapes for terminal toggling).
        assert!(r.out.is_empty());
    }
}
