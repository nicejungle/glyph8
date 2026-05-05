//! A single rendered NES frame, stored as packed RGB.

/// NES native horizontal resolution.
pub const WIDTH: usize = 256;
/// NES native vertical resolution.
pub const HEIGHT: usize = 240;
/// Bytes per pixel (R, G, B — no alpha).
pub const BPP: usize = 3;
/// Total bytes in a frame's pixel buffer.
pub const FRAME_BYTES: usize = WIDTH * HEIGHT * BPP;

/// One rendered frame as packed RGB pixels in row-major order.
///
/// Pixel `(x, y)` lives at byte offset `(y * WIDTH + x) * BPP`.
#[derive(Clone)]
pub struct Frame {
    pub pixels: Box<[u8; FRAME_BYTES]>,
}

impl Frame {
    pub fn new() -> Self {
        Self {
            pixels: Box::new([0; FRAME_BYTES]),
        }
    }

    /// Sets the pixel at `(x, y)` to `rgb`.
    ///
    /// # Panics
    ///
    /// Panics if `x >= WIDTH` or `y >= HEIGHT`.
    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: [u8; 3]) {
        let off = (y * WIDTH + x) * BPP;
        self.pixels[off..off + 3].copy_from_slice(&rgb);
    }

    /// Returns the RGB triple at `(x, y)`.
    ///
    /// # Panics
    ///
    /// Panics if `x >= WIDTH` or `y >= HEIGHT`.
    pub fn get_pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let off = (y * WIDTH + x) * BPP;
        [self.pixels[off], self.pixels[off + 1], self.pixels[off + 2]]
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_dimensions_are_nes_native() {
        assert_eq!(WIDTH, 256);
        assert_eq!(HEIGHT, 240);
        assert_eq!(FRAME_BYTES, 256 * 240 * 3);
    }

    #[test]
    fn new_frame_is_all_zeros() {
        let f = Frame::new();
        assert!(f.pixels.iter().all(|&b| b == 0));
    }

    #[test]
    fn set_then_get_round_trips() {
        let mut f = Frame::new();
        f.set_pixel(10, 20, [0xAA, 0xBB, 0xCC]);
        assert_eq!(f.get_pixel(10, 20), [0xAA, 0xBB, 0xCC]);
        // Untouched pixel stays zero.
        assert_eq!(f.get_pixel(0, 0), [0, 0, 0]);
    }

    #[test]
    fn set_pixel_corners() {
        let mut f = Frame::new();
        f.set_pixel(0, 0, [1, 2, 3]);
        f.set_pixel(WIDTH - 1, HEIGHT - 1, [4, 5, 6]);
        assert_eq!(f.get_pixel(0, 0), [1, 2, 3]);
        assert_eq!(f.get_pixel(WIDTH - 1, HEIGHT - 1), [4, 5, 6]);
    }
}
