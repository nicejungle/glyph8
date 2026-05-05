//! Halfblock renderer — encodes NES frames as ANSI halfblock characters (▀, U+2580).
//!
//! Each terminal cell covers 2 vertical pixels of the rendered canvas: fg = top, bg = bottom.
//!
//! Native canvas is 256 cols × 120 row-pairs (= 256×240 NES pixels). On smaller terminals
//! the renderer can be constructed with a smaller canvas via the `*_fitted` constructors;
//! `draw` then nearest-neighbor downsamples each NES frame into the configured canvas.

use std::io::{self, Write};

use nes_core::{Frame, BPP, HEIGHT, WIDTH};

type Cell = ((u8, u8, u8), (u8, u8, u8)); // (fg=top, bg=bottom)

/// Native canvas dimensions: full NES resolution as halfblock cells.
pub const NATIVE_COLS: u16 = WIDTH as u16; // 256
pub const NATIVE_ROW_PAIRS: u16 = (HEIGHT / 2) as u16; // 120

/// A NES frame rendered as halfblock characters (▀, U+2580).
/// Each terminal cell encodes 2 vertical pixels: fg = top, bg = bottom.
pub struct HalfblockRenderer<W: Write> {
    out: W,
    /// Only `for_stdout*` sets this true; setting it true on a non-terminal writer
    /// will smash the calling process's terminal on `enter`.
    manage_terminal: bool,
    /// Canvas size in cells. (cols, row_pairs).
    cols: u16,
    row_pairs: u16,
    /// Previous frame's cells, indexed by (ry * cols + x).
    /// `None` until first draw — first draw paints everything.
    prev: Option<Vec<Cell>>,
}

impl<W: Write> HalfblockRenderer<W> {
    /// For tests / non-terminal sinks at native (256×120) resolution.
    /// `enter`/`leave` will not touch terminal state.
    pub fn with_writer(out: W) -> Self {
        Self::with_writer_fitted(out, NATIVE_COLS, NATIVE_ROW_PAIRS)
    }

    /// For tests / non-terminal sinks at a custom canvas size.
    /// The renderer downsamples (nearest-neighbor) from native NES resolution
    /// into `cols × row_pairs` cells.
    pub fn with_writer_fitted(out: W, cols: u16, row_pairs: u16) -> Self {
        Self {
            out,
            manage_terminal: false,
            cols: cols.max(1),
            row_pairs: row_pairs.max(1),
            prev: None,
        }
    }

    /// Returns the configured canvas size in cells: `(cols, row_pairs)`.
    pub fn canvas_cells(&self) -> (u16, u16) {
        (self.cols, self.row_pairs)
    }
}

impl HalfblockRenderer<io::Stdout> {
    /// Constructor for production use at native (256×120) resolution: owns stdout,
    /// toggles terminal state on `enter`/`leave` (raw mode, alt screen, cursor hide/show).
    pub fn for_stdout() -> Self {
        Self::for_stdout_fitted(NATIVE_COLS, NATIVE_ROW_PAIRS)
    }

    /// Production constructor for terminals smaller than native. Renderer downsamples
    /// the NES frame nearest-neighbor into the configured canvas.
    pub fn for_stdout_fitted(cols: u16, row_pairs: u16) -> Self {
        Self {
            out: io::stdout(),
            manage_terminal: true,
            cols: cols.max(1),
            row_pairs: row_pairs.max(1),
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
        let cols = self.cols as usize;
        let row_pairs = self.row_pairs as usize;
        let cell_count = cols * row_pairs;

        // Build current cell vector by sampling the source frame.
        // For each output cell (x, ry), the top pixel maps to NES (src_x, src_y_top)
        // and the bottom pixel to NES (src_x, src_y_bot), where:
        //   src_x      = (x      * WIDTH ) / cols
        //   src_y_top  = (2*ry   * HEIGHT) / (row_pairs * 2)  =  (ry * HEIGHT) / row_pairs
        //   src_y_bot  = ((2*ry+1) * HEIGHT) / (row_pairs * 2)
        //
        // For native (cols=WIDTH, row_pairs=HEIGHT/2) this collapses to identity:
        //   src_x = x, src_y_top = 2*ry, src_y_bot = 2*ry+1.
        let mut current: Vec<Cell> = Vec::with_capacity(cell_count);
        for ry in 0..row_pairs {
            let src_y_top = (ry * HEIGHT) / row_pairs;
            let src_y_bot = ((2 * ry + 1) * HEIGHT) / (row_pairs * 2);
            for x in 0..cols {
                let src_x = (x * WIDTH) / cols;
                let top = (src_y_top * WIDTH + src_x) * BPP;
                let bot = (src_y_bot * WIDTH + src_x) * BPP;
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
            // Track whether the cursor is at the start of a run we want to draw.
            // Emit a cursor-position escape only when resuming after a skip.
            let mut last_drawn_x: Option<usize> = None;
            for x in 0..cols {
                let idx = ry * cols + x;
                let cur = current[idx];
                let changed = match prev_ref {
                    None => true, // first draw paints everything
                    Some(p) => p[idx] != cur,
                };
                if !changed {
                    last_drawn_x = None;
                    continue;
                }
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
        // Redrawing the same frame should emit < 5% the bytes
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

    // ---- Fitted (downsampled) canvas tests ----

    #[test]
    fn fitted_emits_one_halfblock_per_cell_at_custom_size() {
        let cols: u16 = 128;
        let row_pairs: u16 = 60;
        let mut r = HalfblockRenderer::with_writer_fitted(Vec::<u8>::new(), cols, row_pairs);
        let frame = Frame::default();
        r.draw(&frame).unwrap();
        let halfblocks = r.out.windows(3).filter(|w| *w == "▀".as_bytes()).count();
        assert_eq!(halfblocks, (cols as usize) * (row_pairs as usize));
    }

    #[test]
    fn fitted_canvas_cells_returns_configured_size() {
        let r = HalfblockRenderer::with_writer_fitted(Vec::<u8>::new(), 136, 64);
        assert_eq!(r.canvas_cells(), (136, 64));
    }

    #[test]
    fn fitted_minimum_size_is_clamped_to_one() {
        // Defensive: passing 0 mustn't divide-by-zero in draw.
        let mut r = HalfblockRenderer::with_writer_fitted(Vec::<u8>::new(), 0, 0);
        let frame = Frame::default();
        // Should produce a single 1×1 cell without panicking.
        r.draw(&frame).unwrap();
        assert_eq!(r.canvas_cells(), (1, 1));
        let halfblocks = r.out.windows(3).filter(|w| *w == "▀".as_bytes()).count();
        assert_eq!(halfblocks, 1);
    }

    #[test]
    fn fitted_half_size_samples_correct_source_pixels() {
        // At cols=128 (half native width), output cell x=0 samples src_x=0,
        // and x=1 samples src_x=2. Place a red pixel at NES (2, 0) and confirm
        // it appears in the second cell of the output (the SGR for x=1).
        let mut r = HalfblockRenderer::with_writer_fitted(Vec::<u8>::new(), 128, 60);
        let mut frame = Frame::default();
        frame.set_pixel(2, 0, [255, 0, 0]);
        r.draw(&frame).unwrap();
        let s = String::from_utf8_lossy(&r.out);
        assert!(
            s.contains("38;2;255;0;0"),
            "expected red SGR somewhere in the downsampled output"
        );
    }

    #[test]
    fn native_default_constructor_yields_native_canvas() {
        let r = HalfblockRenderer::with_writer(Vec::<u8>::new());
        assert_eq!(r.canvas_cells(), (NATIVE_COLS, NATIVE_ROW_PAIRS));
    }
}
