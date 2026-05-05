use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::poll as event_poll;
use crossterm::event::read as event_read;

use nes_core::{ControllerState, EmulatorBackend};
use nes_render::halfblock::{HalfblockRenderer, NATIVE_COLS, NATIVE_ROW_PAIRS};
use nes_render::Renderer;
use nes_tetanes_backend::TetanesBackend;

use crate::fps::FpsMeter;
use crate::input::{Input, PollOutcome};

/// NTSC frame period: ~16.639 ms.
const FRAME_DUR: Duration = Duration::from_nanos(16_639_267);

/// Minimum terminal size we'll bother rendering into. Below this, the picture
/// is too small to be useful and we bail with a friendly error.
/// 64 cols × 33 rows = 64×32 cell canvas + 1 status row → ~1/4 native scale.
const MIN_TERM_COLS: u16 = 64;
const MIN_TERM_ROWS: u16 = 33;

/// Returns `Err` if the terminal is below the minimum useful size.
fn check_terminal_size(cols: u16, rows: u16) -> Result<()> {
    if cols < MIN_TERM_COLS || rows < MIN_TERM_ROWS {
        anyhow::bail!(
            "terminal too small: {}×{} cells, need at least {}×{}.\n\
             Resize your terminal (or shrink the font), \
             or use --headless --frames=N for non-interactive testing.",
            cols,
            rows,
            MIN_TERM_COLS,
            MIN_TERM_ROWS
        );
    }
    Ok(())
}

/// Pick the largest aspect-preserving canvas size that fits the available
/// terminal cells without upscaling beyond native NES resolution.
///
/// `term_cols` is total terminal width in cells; `avail_row_pairs` is total
/// terminal height in cells *minus the status row*. Returns the chosen
/// canvas size in cells: `(cols, row_pairs)`.
fn compute_canvas(term_cols: u16, avail_row_pairs: u16) -> (u16, u16) {
    let scale_x = term_cols as f32 / NATIVE_COLS as f32;
    let scale_y = avail_row_pairs as f32 / NATIVE_ROW_PAIRS as f32;
    let scale = scale_x.min(scale_y).min(1.0);
    let cols = ((NATIVE_COLS as f32) * scale).floor() as u16;
    let row_pairs = ((NATIVE_ROW_PAIRS as f32) * scale).floor() as u16;
    (cols.max(1), row_pairs.max(1))
}

pub fn run(rom_path: &Path) -> Result<()> {
    let (term_cols, term_rows) = crossterm::terminal::size().context("querying terminal size")?;
    check_terminal_size(term_cols, term_rows)?;
    // Reserve one row at the bottom for the status line.
    let avail_row_pairs = term_rows.saturating_sub(1);
    let (canvas_cols, canvas_row_pairs) = compute_canvas(term_cols, avail_row_pairs);

    let rom_bytes =
        fs::read(rom_path).with_context(|| format!("reading ROM {}", rom_path.display()))?;
    let rom_label = rom_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("rom")
        .to_string();

    let mut backend = TetanesBackend::new();
    backend.load_rom(&rom_bytes)?;

    let mut renderer = HalfblockRenderer::for_stdout_fitted(canvas_cols, canvas_row_pairs);
    let mut input = Input::new();
    let mut fps = FpsMeter::new(60);

    renderer.enter()?;
    let result = run_loop(
        &mut backend,
        &mut renderer,
        &mut input,
        &mut fps,
        &rom_label,
        canvas_row_pairs,
    );
    // Always restore terminal, even on error.
    let _ = renderer.leave();
    result
}

fn run_loop(
    backend: &mut TetanesBackend,
    renderer: &mut HalfblockRenderer<std::io::Stdout>,
    input: &mut Input,
    fps: &mut FpsMeter,
    rom_label: &str,
    canvas_row_pairs: u16,
) -> Result<()> {
    let status_row = canvas_row_pairs; // 0-indexed terminal row immediately below canvas
    let mut next = Instant::now() + FRAME_DUR;
    loop {
        // Drain all pending events without blocking.
        input.begin_frame();
        let mut control: Option<PollOutcome> = None;
        while event_poll(Duration::ZERO)? {
            let ev = event_read()?;
            if let Some(outcome) = input.handle_event(&ev) {
                control = Some(outcome);
                break;
            }
        }
        match control {
            Some(PollOutcome::Quit) => return Ok(()),
            Some(PollOutcome::Reset) => backend.reset(),
            None => {}
        }
        backend.submit_input(input.p1(), ControllerState::empty());
        backend.step_frame()?;
        renderer.draw(backend.frame())?;
        write_status_line(
            rom_label,
            fps.tick(),
            status_row,
            input.events_seen(),
            input.last_event(),
            input.p1(),
        )?;
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        }
        next += FRAME_DUR;
    }
}

fn write_status_line(
    rom_label: &str,
    fps: f32,
    row: u16,
    events: u64,
    last_event: &str,
    p1: ControllerState,
) -> std::io::Result<()> {
    use crossterm::cursor::MoveTo;
    use crossterm::queue;
    use crossterm::style::Print;
    use crossterm::terminal::{Clear, ClearType};

    let mut out = std::io::stdout();
    queue!(
        out,
        MoveTo(0, row),
        Clear(ClearType::CurrentLine),
        Print(format!(
            "{} FPS:{:.0} ev:{} last:{} p1:{:08b} ESC:quit R:reset",
            rom_label, fps, events, last_event, p1.0
        )),
    )?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_minimum_required_size() {
        assert!(check_terminal_size(MIN_TERM_COLS, MIN_TERM_ROWS).is_ok());
    }

    #[test]
    fn accepts_larger_size() {
        assert!(check_terminal_size(400, 200).is_ok());
    }

    #[test]
    fn rejects_too_few_columns() {
        let err = check_terminal_size(MIN_TERM_COLS - 1, MIN_TERM_ROWS).unwrap_err();
        assert!(err.to_string().contains("terminal too small"));
    }

    #[test]
    fn rejects_too_few_rows() {
        let err = check_terminal_size(MIN_TERM_COLS, MIN_TERM_ROWS - 1).unwrap_err();
        assert!(err.to_string().contains("terminal too small"));
    }

    #[test]
    fn error_includes_actual_and_required_dimensions() {
        let err = check_terminal_size(40, 20).unwrap_err().to_string();
        assert!(err.contains("40"), "actual cols not in error: {}", err);
        assert!(err.contains("20"), "actual rows not in error: {}", err);
        assert!(err.contains("64"), "required cols not in error: {}", err);
        assert!(err.contains("33"), "required rows not in error: {}", err);
    }

    #[test]
    fn compute_canvas_native_when_terminal_meets_native_size() {
        let (cols, row_pairs) = compute_canvas(NATIVE_COLS, NATIVE_ROW_PAIRS);
        assert_eq!((cols, row_pairs), (NATIVE_COLS, NATIVE_ROW_PAIRS));
    }

    #[test]
    fn compute_canvas_native_when_terminal_exceeds_native() {
        // Should not upscale.
        let (cols, row_pairs) = compute_canvas(400, 200);
        assert_eq!((cols, row_pairs), (NATIVE_COLS, NATIVE_ROW_PAIRS));
    }

    #[test]
    fn compute_canvas_fits_user_reported_size() {
        // 210×64 avail row-pairs (i.e. 65 terminal rows minus 1 for status).
        // Expected: scale = min(210/256, 64/120) = 64/120 ≈ 0.533;
        //   cols = floor(256 * 0.533) = 136
        //   row_pairs = floor(120 * 0.533) = 64
        let (cols, row_pairs) = compute_canvas(210, 64);
        assert_eq!(cols, 136);
        assert_eq!(row_pairs, 64);
    }

    #[test]
    fn compute_canvas_aspect_preserved_on_wide_short_terminal() {
        // 400 cols but only 30 row-pairs available. Height-bound:
        //   scale = min(400/256, 30/120) = 30/120 = 0.25
        //   cols = 64, row_pairs = 30
        let (cols, row_pairs) = compute_canvas(400, 30);
        assert_eq!(cols, 64);
        assert_eq!(row_pairs, 30);
    }

    #[test]
    fn compute_canvas_minimum_one_when_input_is_zero() {
        // Defensive: never return (0, 0).
        let (cols, row_pairs) = compute_canvas(0, 0);
        assert!(cols >= 1 && row_pairs >= 1);
    }
}
