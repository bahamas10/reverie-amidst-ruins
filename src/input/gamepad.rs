/*!
 * Translate gilrs standard gamepad layout into the game's input state.
 */

use gilrs::{Button, Event, EventType, GamepadId, Gilrs};

use super::InputState;

pub(super) struct Gamepad {
    gilrs: Option<Gilrs>,

    // if none (it currently always is at init) we keep track of the first
    // gamepad that talks
    active: Option<GamepadId>,
}

impl Default for Gamepad {
    fn default() -> Self {
        // gamepad support is option / best-effort so just throw away any init
        // errors and store it as an option
        Self { gilrs: Gilrs::new().ok(), active: None }
    }
}

impl Gamepad {
    pub fn update(&mut self) -> InputState {
        let mut state = InputState::default();

        let Some(gilrs) = &mut self.gilrs else {
            return state;
        };

        // look at all gamepads: todo only allow 1? this is meant to be a cute
        // terminal project so i don't need to get *too* fancy lol
        while let Some(Event { event, id, .. }) = gilrs.next_event() {
            // keep track of this gamepad if it's the first one we've seen
            if self.active.is_none() {
                self.active = Some(id);
            }

            // ensure we are only looking at the gamepad we care about
            let active = self.active.unwrap();
            if id != active {
                continue;
            }

            // keep track of one-shot buttons
            match event {
                EventType::ButtonPressed(Button::South, _) => {
                    state.jump_pressed = true;
                    state.confirm_pressed = true;
                }
                EventType::ButtonPressed(Button::Start, _) => {
                    state.confirm_pressed = true;
                }
                EventType::ButtonPressed(Button::West, _) => {
                    state.restart_pressed = true;
                }
                EventType::Disconnected => {
                    self.active = None;
                }
                _ => {}
            }
        }

        // look for buttons being held down
        if let Some(active) = self.active {
            let gamepad = gilrs.gamepad(active);

            if gamepad.is_connected() {
                state.left = gamepad.is_pressed(Button::DPadLeft);
                state.right = gamepad.is_pressed(Button::DPadRight);
                state.jump = gamepad.is_pressed(Button::South);
                state.confirm = state.jump || gamepad.is_pressed(Button::Start);
            } else {
                self.active = None;
            }
        }

        state
    }
}
