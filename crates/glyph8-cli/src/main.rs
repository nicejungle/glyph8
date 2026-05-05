mod cli;
mod fps;
mod headless;
mod input;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    if args.headless {
        headless::run(&args.rom, args.frames)
    } else {
        // Interactive runloop wired in Task 10.
        anyhow::bail!("interactive mode not yet implemented (use --headless for now)");
    }
}
