mod cli;

use clap::Parser;

fn main() {
    let args = cli::Args::parse();
    eprintln!(
        "parsed: rom={}, headless={}, frames={}",
        args.rom.display(),
        args.headless,
        args.frames
    );
}
