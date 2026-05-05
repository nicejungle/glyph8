//! End-to-end: run `glyph8 --headless --frames=60` on bundled nestest.nes
//! and assert that the emitted blake3 hash is reproducible across runs.
//!
//! We don't pin a specific hash value here — tetanes-core could change its
//! frame output between releases. Instead we run twice and assert equality,
//! which is a stronger property: the emulator must be deterministic.

use std::process::Command;

fn rom_path() -> String {
    // Workspace-root-relative — `cargo test` runs with CWD at the crate root,
    // so we go up two levels.
    let manifest = env!("CARGO_MANIFEST_DIR"); // .../crates/glyph8-cli
    format!("{}/../../tests/roms/nestest.nes", manifest)
}

#[test]
fn headless_run_is_deterministic() {
    let bin = env!("CARGO_BIN_EXE_glyph8");
    let run = || -> String {
        let out = Command::new(bin)
            .args(["--headless", "--frames=60", &rom_path()])
            .output()
            .expect("failed to run glyph8");
        assert!(
            out.status.success(),
            "glyph8 exited non-zero: stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };
    let h1 = run();
    let h2 = run();
    assert_eq!(
        h1, h2,
        "headless run must be deterministic across invocations"
    );
    // Hash should be 64 hex chars (blake3 default = 256 bits).
    assert_eq!(
        h1.len(),
        64,
        "blake3 hash should be 64 hex chars, got {:?}",
        h1
    );
}
