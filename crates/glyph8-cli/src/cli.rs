use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "glyph8", version, about = "CLI NES emulator (terminal)")]
pub struct Args {
    /// Path to a .nes ROM file.
    pub rom: PathBuf,

    /// Run N frames headless and print a blake3 hash of the final frame buffer.
    #[arg(long)]
    pub headless: bool,

    /// Number of frames to step in headless mode. Ignored without --headless.
    #[arg(long, default_value_t = 60)]
    pub frames: u32,
}
