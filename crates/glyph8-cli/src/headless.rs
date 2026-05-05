use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use nes_core::EmulatorBackend;
use nes_tetanes_backend::TetanesBackend;

pub fn run(rom_path: &Path, frames: u32) -> Result<()> {
    let bytes =
        fs::read(rom_path).with_context(|| format!("reading ROM {}", rom_path.display()))?;
    let mut backend = TetanesBackend::new();
    backend.load_rom(&bytes)?;
    for _ in 0..frames {
        backend.step_frame()?;
    }
    let hash = blake3::hash(backend.frame().pixels.as_ref());
    println!("{}", hash.to_hex());
    Ok(())
}
