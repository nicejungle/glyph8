#![allow(dead_code)]

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use nes_core::ControllerState;

/// Outcome of polling input for a frame.
#[derive(Debug, PartialEq)]
pub enum PollOutcome {
    Continue(ControllerState),
    Reset,
    Quit,
}

/// State carried across frames: which buttons are currently considered "held".
#[derive(Default)]
pub struct Input {
    p1: ControllerState,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a single crossterm event. Returns `Some(outcome)` if the event
    /// produced a runloop-level decision (Reset / Quit); otherwise `None`,
    /// meaning "state updated, keep looping".
    pub fn handle_event(&mut self, ev: &Event) -> Option<PollOutcome> {
        let Event::Key(KeyEvent {
            code,
            modifiers,
            kind,
            ..
        }) = ev
        else {
            return None;
        };
        // Most terminals only deliver `Press` (no repeat or release) without the
        // Kitty enhancement protocol. We treat every Press as both "press" and
        // "release at end of frame" — the runloop clears `self.p1` once per
        // frame before draining events. See spec §3.3 for the trade-off note.
        if !matches!(kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return None;
        }
        // Ctrl+C — quit regardless of code (covers Ctrl+C as char 'c').
        if modifiers.contains(KeyModifiers::CONTROL)
            && matches!(code, KeyCode::Char('c') | KeyCode::Char('C'))
        {
            return Some(PollOutcome::Quit);
        }
        match code {
            KeyCode::Esc => return Some(PollOutcome::Quit),
            KeyCode::Char('r') | KeyCode::Char('R') => return Some(PollOutcome::Reset),
            KeyCode::Up => self.p1.press(ControllerState::UP),
            KeyCode::Down => self.p1.press(ControllerState::DOWN),
            KeyCode::Left => self.p1.press(ControllerState::LEFT),
            KeyCode::Right => self.p1.press(ControllerState::RIGHT),
            KeyCode::Char('z') | KeyCode::Char('Z') => self.p1.press(ControllerState::B),
            KeyCode::Char('x') | KeyCode::Char('X') => self.p1.press(ControllerState::A),
            KeyCode::Enter => self.p1.press(ControllerState::START),
            // Right Shift detection: crossterm reports it as a SHIFT modifier on
            // an empty key — there's no dedicated keycode. As a beta compromise,
            // we accept the more reliable signal: Tab as Select.
            // (Mednafen-style RShift→Select needs Kitty protocol; that comes in 1.C.)
            KeyCode::Tab => self.p1.press(ControllerState::SELECT),
            _ => {}
        }
        None
    }

    /// Reset the held-button mask. Call once per frame *before* draining events.
    pub fn begin_frame(&mut self) {
        self.p1 = ControllerState::empty();
    }

    /// Snapshot the current pressed-button mask for submission to the backend.
    pub fn p1(&self) -> ControllerState {
        self.p1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn esc_quits() {
        let mut i = Input::new();
        let r = i.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(r, Some(PollOutcome::Quit));
    }

    #[test]
    fn ctrl_c_quits() {
        let mut i = Input::new();
        let r = i.handle_event(&key(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(r, Some(PollOutcome::Quit));
    }

    #[test]
    fn r_resets() {
        let mut i = Input::new();
        let r = i.handle_event(&key(KeyCode::Char('r'), KeyModifiers::NONE));
        assert_eq!(r, Some(PollOutcome::Reset));
    }

    #[test]
    fn z_presses_b() {
        let mut i = Input::new();
        i.handle_event(&key(KeyCode::Char('z'), KeyModifiers::NONE));
        assert!(i.p1().pressed(ControllerState::B));
        assert!(!i.p1().pressed(ControllerState::A));
    }

    #[test]
    fn arrows_press_dpad() {
        let mut i = Input::new();
        i.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
        i.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
        assert!(i.p1().pressed(ControllerState::UP));
        assert!(i.p1().pressed(ControllerState::RIGHT));
    }

    #[test]
    fn begin_frame_clears_held_buttons() {
        let mut i = Input::new();
        i.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
        assert!(i.p1().pressed(ControllerState::UP));
        i.begin_frame();
        assert_eq!(i.p1(), ControllerState::empty());
    }
}
