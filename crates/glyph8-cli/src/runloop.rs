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

pub fn run(rom_path: &Path) -> Result<()> {
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
        MoveTo(0, 121),
        Clear(ClearType::CurrentLine),
        Print(format!(
            "{} | FPS: {:>5.1} | ESC: quit | R: reset",
            rom_label, fps
        )),
    )?;
    out.flush()
}
