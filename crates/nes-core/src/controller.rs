//! NES controller (Famicom standard pad) state.

/// Bit-packed state of one controller. `1` = pressed.
///
/// Bit layout matches the order the NES strobe protocol shifts out: A, B,
/// Select, Start, Up, Down, Left, Right.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct ControllerState(pub u8);

impl ControllerState {
    pub const A: u8 = 1 << 0;
    pub const B: u8 = 1 << 1;
    pub const SELECT: u8 = 1 << 2;
    pub const START: u8 = 1 << 3;
    pub const UP: u8 = 1 << 4;
    pub const DOWN: u8 = 1 << 5;
    pub const LEFT: u8 = 1 << 6;
    pub const RIGHT: u8 = 1 << 7;

    pub const fn empty() -> Self {
        Self(0)
    }

    pub fn pressed(self, mask: u8) -> bool {
        self.0 & mask != 0
    }

    pub fn press(&mut self, mask: u8) {
        self.0 |= mask;
    }
    pub fn release(&mut self, mask: u8) {
        self.0 &= !mask;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_no_buttons_pressed() {
        let s = ControllerState::empty();
        for mask in [
            ControllerState::A,
            ControllerState::B,
            ControllerState::SELECT,
            ControllerState::START,
            ControllerState::UP,
            ControllerState::DOWN,
            ControllerState::LEFT,
            ControllerState::RIGHT,
        ] {
            assert!(!s.pressed(mask));
        }
    }

    #[test]
    fn press_then_release() {
        let mut s = ControllerState::empty();
        s.press(ControllerState::A);
        assert!(s.pressed(ControllerState::A));
        assert!(!s.pressed(ControllerState::B));
        s.release(ControllerState::A);
        assert!(!s.pressed(ControllerState::A));
    }

    #[test]
    fn bit_layout_is_strobe_order() {
        // A is LSB, Right is MSB — required to match NES $4016 read order.
        assert_eq!(ControllerState::A, 0b0000_0001);
        assert_eq!(ControllerState::B, 0b0000_0010);
        assert_eq!(ControllerState::SELECT, 0b0000_0100);
        assert_eq!(ControllerState::START, 0b0000_1000);
        assert_eq!(ControllerState::UP, 0b0001_0000);
        assert_eq!(ControllerState::DOWN, 0b0010_0000);
        assert_eq!(ControllerState::LEFT, 0b0100_0000);
        assert_eq!(ControllerState::RIGHT, 0b1000_0000);
    }
}
