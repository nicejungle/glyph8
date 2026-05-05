use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::poll as event_poll;
use crossterm::event::read as event_read;

use nes_core::{ControllerState, EmulatorBackend};
use nes_render::halfblock::HalfblockRenderer;
use nes_render::Renderer;
use nes_tetanes_backend::TetanesBackend;

use crate::fps::FpsMeter;
use crate::input::{Input, PollOutcome};

/// NTSC frame period: ~16.639 ms.
const FRAME_DUR: Duration = Duration::from_nanos(16_639_267);

/// Minimum terminal cells to fit the halfblock canvas (256×120) + 1 status row.
const MIN_COLS: u16 = 256;
const MIN_ROWS: u16 = 121;

/// Returns `Err` if the given cell dimensions can't fit the halfblock canvas
/// + status row. Pure function so we can test it without a real terminal.
fn check_terminal_size(cols: u16, rows: u16) -> Result<()> {
    if cols < MIN_COLS || rows < MIN_ROWS {
        anyhow::bail!(
            "terminal too small: {}×{} cells, need at least {}×{}.\n\
             Resize your terminal (or shrink the font), \
             or use --headless --frames=N for non-interactive testing.\n\
             (Adaptive smaller-terminal modes — braille / ASCII — land in Stage 1.E.)",
            cols,
            rows,
            MIN_COLS,
            MIN_ROWS
        );
    }
    Ok(())
}

pub fn run(rom_path: &Path) -> Result<()> {
    let (cols, rows) = crossterm::terminal::size().context("querying terminal size")?;
    check_terminal_size(cols, rows)?;

    let rom_bytes =
        fs::read(rom_path).with_context(|| format!("reading ROM {}", rom_path.display()))?;
    let rom_label = rom_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("rom")
        .to_string();

    let mut backend = TetanesBackend::new();
    backend.load_rom(&rom_bytes)?;

    let mut renderer = HalfblockRenderer::for_stdout();
    let mut input = Input::new();
    let mut fps = FpsMeter::new(60);

    renderer.enter()?;
    let result = run_loop(
        &mut backend,
        &mut renderer,
        &mut input,
        &mut fps,
        &rom_label,
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
) -> Result<()> {
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
        write_status_line(rom_label, fps.tick())?;
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        }
        next += FRAME_DUR;
    }
}

fn write_status_line(rom_label: &str, fps: f32) -> std::io::Result<()> {
    use crossterm::cursor::MoveTo;
    use crossterm::queue;
    use crossterm::style::Print;
    use crossterm::terminal::{Clear, ClearType};

    let mut out = std::io::stdout();
    // Status row is just below the 120 halfblock rows.
    queue!(
        out,
        MoveTo(0, 120),
        Clear(ClearType::CurrentLine),
        Print(format!(
            "{} | FPS: {:>5.1} | ESC: quit | R: reset",
            rom_label, fps
        )),
    )?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_minimum_required_size() {
        assert!(check_terminal_size(MIN_COLS, MIN_ROWS).is_ok());
    }

    #[test]
    fn accepts_larger_size() {
        assert!(check_terminal_size(400, 200).is_ok());
    }

    #[test]
    fn rejects_too_few_columns() {
        let err = check_terminal_size(MIN_COLS - 1, MIN_ROWS).unwrap_err();
        assert!(err.to_string().contains("terminal too small"));
    }

    #[test]
    fn rejects_too_few_rows() {
        let err = check_terminal_size(MIN_COLS, MIN_ROWS - 1).unwrap_err();
        assert!(err.to_string().contains("terminal too small"));
    }

    #[test]
    fn error_includes_actual_and_required_dimensions() {
        let err = check_terminal_size(80, 24).unwrap_err().to_string();
        assert!(err.contains("80"), "actual cols not in error: {}", err);
        assert!(err.contains("24"), "actual rows not in error: {}", err);
        assert!(err.contains("256"), "required cols not in error: {}", err);
        assert!(err.contains("121"), "required rows not in error: {}", err);
    }
}
