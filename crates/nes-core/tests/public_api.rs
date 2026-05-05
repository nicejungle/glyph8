//! Verifies the public API surface of nes-core.

use nes_core::{
    parse_header, ControllerState, EmulatorBackend, EmulatorError, Frame, Mirroring, RomInfo,
    FRAME_BYTES, HEIGHT, WIDTH,
};

fn _trait_is_object_safe(_: Box<dyn EmulatorBackend>) {}

#[test]
fn dimensions_are_exported() {
    assert_eq!(WIDTH, 256);
    assert_eq!(HEIGHT, 240);
    assert_eq!(FRAME_BYTES, 256 * 240 * 3);
}

#[test]
fn types_can_be_constructed_externally() {
    let _f = Frame::default();
    let _c = ControllerState::empty();
    let _m = Mirroring::Horizontal;
    let info = RomInfo {
        mapper: 0,
        prg_rom_size: 16 * 1024,
        chr_rom_size: 8 * 1024,
        mirroring: Mirroring::Horizontal,
        has_battery: false,
    };
    assert_eq!(info.mapper, 0);
}

#[test]
fn parse_header_is_callable() {
    let err = parse_header(b"too short").unwrap_err();
    assert!(matches!(err, EmulatorError::RomTooSmall(_)));
}
