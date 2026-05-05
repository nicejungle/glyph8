mod cli;
mod fps;
mod headless;
mod input;
mod runloop;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    if args.headless {
        headless::run(&args.rom, args.frames)
    } else {
        runloop::run(&args.rom)
    }
}
