//! Core abstractions for the Glyph8 NES emulator.
//!
//! This crate defines the [`EmulatorBackend`] trait and the value types
//! ([`Frame`], [`ControllerState`], [`RomInfo`]) shared between any backend
//! implementation (e.g. `nes-tetanes-backend`, `nes-native`) and the CLI
//! frontend.
