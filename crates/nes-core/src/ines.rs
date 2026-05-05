//! iNES (.nes) ROM header parser.
//!
//! See https://www.nesdev.org/wiki/INES for the format spec.

use crate::error::EmulatorError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RomInfo {
    pub mapper: u8,
    pub prg_rom_size: usize,
    pub chr_rom_size: usize,
    pub mirroring: Mirroring,
    pub has_battery: bool,
}

const HEADER_SIZE: usize = 16;
const MAGIC: &[u8; 4] = b"NES\x1A";
const PRG_BANK_SIZE: usize = 16 * 1024;
const CHR_BANK_SIZE: usize = 8 * 1024;

pub fn parse_header(rom: &[u8]) -> Result<RomInfo, EmulatorError> {
    if rom.len() < HEADER_SIZE {
        return Err(EmulatorError::RomTooSmall(rom.len()));
    }
    if &rom[0..4] != MAGIC {
        return Err(EmulatorError::InvalidINesHeader);
    }

    let prg_banks = rom[4] as usize;
    let chr_banks = rom[5] as usize;
    let flags6 = rom[6];
    let flags7 = rom[7];

    let prg_rom_size = prg_banks * PRG_BANK_SIZE;
    let chr_rom_size = chr_banks * CHR_BANK_SIZE;

    let mirroring = if flags6 & 0b0000_1000 != 0 {
        Mirroring::FourScreen
    } else if flags6 & 0b0000_0001 != 0 {
        Mirroring::Vertical
    } else {
        Mirroring::Horizontal
    };
    let has_battery = flags6 & 0b0000_0010 != 0;
    let mapper = (flags7 & 0b1111_0000) | (flags6 >> 4);

    let expected_min = HEADER_SIZE + prg_rom_size + chr_rom_size;
    if rom.len() < expected_min {
        return Err(EmulatorError::RomTooSmall(rom.len()));
    }

    Ok(RomInfo {
        mapper,
        prg_rom_size,
        chr_rom_size,
        mirroring,
        has_battery,
    })
}

#[cfg(test)]
pub(crate) fn make_minimal_nrom() -> Vec<u8> {
    let mut rom = Vec::with_capacity(16 + 16 * 1024 + 8 * 1024);
    rom.extend_from_slice(b"NES\x1A");
    rom.push(1); // 1 × 16 KB PRG
    rom.push(1); // 1 × 8 KB CHR
    rom.push(0); // flags 6: mapper 0, horizontal mirror, no battery
    rom.push(0); // flags 7
    rom.extend(std::iter::repeat(0u8).take(8)); // padding
    rom.extend(std::iter::repeat(0u8).take(16 * 1024));
    rom.extend(std::iter::repeat(0u8).take(8 * 1024));
    rom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_nrom() {
        let rom = make_minimal_nrom();
        let info = parse_header(&rom).unwrap();
        assert_eq!(info.mapper, 0);
        assert_eq!(info.prg_rom_size, 16 * 1024);
        assert_eq!(info.chr_rom_size, 8 * 1024);
        assert_eq!(info.mirroring, Mirroring::Horizontal);
        assert!(!info.has_battery);
    }

    #[test]
    fn vertical_mirror_flag() {
        let mut rom = make_minimal_nrom();
        rom[6] |= 0b0000_0001;
        let info = parse_header(&rom).unwrap();
        assert_eq!(info.mirroring, Mirroring::Vertical);
    }

    #[test]
    fn four_screen_mirror_flag() {
        let mut rom = make_minimal_nrom();
        rom[6] |= 0b0000_1000;
        let info = parse_header(&rom).unwrap();
        assert_eq!(info.mirroring, Mirroring::FourScreen);
    }

    #[test]
    fn battery_flag() {
        let mut rom = make_minimal_nrom();
        rom[6] |= 0b0000_0010;
        let info = parse_header(&rom).unwrap();
        assert!(info.has_battery);
    }

    #[test]
    fn mapper_id_split_across_flags6_and_flags7() {
        let mut rom = make_minimal_nrom();
        // mapper 0x4A: low nybble in flags6 high nybble, high nybble in flags7 high nybble
        rom[6] = 0xA0;
        rom[7] = 0x40;
        let info = parse_header(&rom).unwrap();
        assert_eq!(info.mapper, 0x4A);
    }

    #[test]
    fn rejects_too_short_for_header() {
        let rom = b"NES\x1A".to_vec(); // 4 bytes, less than HEADER_SIZE
        let err = parse_header(&rom).unwrap_err();
        assert!(matches!(err, EmulatorError::RomTooSmall(4)));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut rom = make_minimal_nrom();
        rom[0] = b'X';
        let err = parse_header(&rom).unwrap_err();
        assert!(matches!(err, EmulatorError::InvalidINesHeader));
    }

    #[test]
    fn rejects_truncated_prg() {
        let mut rom = make_minimal_nrom();
        // claim 2 PRG banks (32 KB) but only ship 1
        rom[4] = 2;
        let err = parse_header(&rom).unwrap_err();
        assert!(matches!(err, EmulatorError::RomTooSmall(_)));
    }
}
