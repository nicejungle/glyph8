//! Errors that can arise from any [`crate::EmulatorBackend`] operation.

#[derive(thiserror::Error, Debug)]
pub enum EmulatorError {
    #[error("invalid iNES header")]
    InvalidINesHeader,
    #[error("rom too small ({0} bytes)")]
    RomTooSmall(usize),
    #[error("unsupported mapper {0}")]
    UnsupportedMapper(u8),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("backend error: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_are_useful() {
        let e = EmulatorError::InvalidINesHeader;
        assert_eq!(e.to_string(), "invalid iNES header");

        let e = EmulatorError::UnsupportedMapper(7);
        assert_eq!(e.to_string(), "unsupported mapper 7");

        let e = EmulatorError::RomTooSmall(15);
        assert_eq!(e.to_string(), "rom too small (15 bytes)");
    }

    #[test]
    fn io_error_converts_via_from() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "x");
        let e: EmulatorError = io.into();
        assert!(matches!(e, EmulatorError::Io(_)));
    }
}
