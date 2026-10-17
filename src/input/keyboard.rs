/*!
 * Translate crossterm keyboard events into the game's input state.
 */

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use super::InputState;

#[derive(Default)]
pub(super) struct Keyboard {
    // the keyboard doesn't retain "held down" state - so we must do that
    // ourselves by retaining this object and updating it for every call to
    // update()
    state: InputState,
}

impl Keyboard {
    pub fn update(&mut self, events: &[KeyEvent]) -> InputState {
        self.state.clear_pressed();

        for key in events {
            let is_down = key.kind != KeyEventKind::Release;
            let is_pressed = key.kind == KeyEventKind::Press;

            match key.code {
                KeyCode::Left | KeyCode::Char('a' | 'A') => {
                    self.state.arrow_pressed |= is_pressed;
                    self.state.left = is_down
                }
                KeyCode::Right | KeyCode::Char('d' | 'D') => {
                    self.state.arrow_pressed |= is_pressed;
                    self.state.right = is_down
                }
                KeyCode::Up | KeyCode::Down if is_pressed => {
                    self.state.arrow_pressed = true;
                }
                KeyCode::Char(' ') => {
                    if is_pressed {
                        self.state.jump_pressed = true;
                    }
                    self.state.jump = is_down;
                }
                KeyCode::Char('r') if is_pressed => {
                    self.state.restart_pressed = true;
                }
                KeyCode::Esc | KeyCode::Char('q' | 'Q') => {
                    self.state.quit = is_down
                }
                KeyCode::Enter => {
                    if is_pressed {
                        self.state.confirm_pressed = true;
                    }
                    self.state.confirm = is_down;
                }
                KeyCode::Char('1') if is_pressed => {
                    self.state.effects_toggle_pressed = true;
                }
                KeyCode::Char('2') if is_pressed => {
                    self.state.screenshot_pressed = true;
                }
                KeyCode::Char('3') if is_pressed => {
                    self.state.palette_randomize_pressed = true;
                }
                KeyCode::Char('m') if is_pressed => {
                    self.state.mute_toggle_pressed = true;
                }
                KeyCode::Char('8') if is_pressed => {
                    self.state.volume_down_pressed = true;
                }
                KeyCode::Char('9') if is_pressed => {
                    self.state.volume_up_pressed = true;
                }
                KeyCode::Char('-' | '_') if is_pressed => {
                    self.state.scale_down_pressed = true;
                }
                KeyCode::Char('+' | '=') if is_pressed => {
                    self.state.scale_up_pressed = true;
                }
                _ => {}
            }
        }

        self.state
    }
}
