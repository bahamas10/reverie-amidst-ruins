/*!
 * Handle input from the terminal and abstract it into a simple struct
 */

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

#[derive(Default)]
pub struct Input {
    // is holding / pressed left
    pub left: bool,

    // is holding / pressed right
    pub right: bool,

    // is holding / pressed jump
    pub jump: bool,

    // just pressed jump
    pub jump_pressed: bool,

    // is holding / pressed enter
    pub confirm: bool,

    // just pressed enter
    pub confirm_pressed: bool,

    // just pressed screenshot
    pub screenshot_pressed: bool,

    // just pressed effects toggle
    pub effects_toggle_pressed: bool,

    // just pressed palette randomize
    pub palette_randomize_pressed: bool,

    // just pressed level restart
    pub restart_pressed: bool,

    // just pressed mute toggle
    pub mute_toggle_pressed: bool,

    // just pressed volume down/up
    pub volume_down_pressed: bool,
    pub volume_up_pressed: bool,

    // just pressed image scale down/up
    pub scale_down_pressed: bool,
    pub scale_up_pressed: bool,

    // is holding / pressed quit
    pub quit: bool,
}

impl Input {
    pub fn handle_key(&mut self, key: KeyEvent) {
        let is_down = key.kind != KeyEventKind::Release;
        let is_pressed = key.kind == KeyEventKind::Press;

        match key.code {
            KeyCode::Left | KeyCode::Char('a' | 'A') => self.left = is_down,
            KeyCode::Right | KeyCode::Char('d' | 'D') => self.right = is_down,
            KeyCode::Char(' ') => {
                if is_pressed {
                    // we will *always* unset this after we update the player
                    self.jump_pressed = true;
                }
                self.jump = is_down;
            }
            KeyCode::Char('r') if is_pressed => {
                self.restart_pressed = true;
            }
            KeyCode::Esc | KeyCode::Char('q' | 'Q') => self.quit = is_down,
            KeyCode::Enter => {
                if is_pressed {
                    self.confirm_pressed = true;
                }
                self.confirm = is_down;
            }
            KeyCode::Char('1') if is_pressed => {
                self.effects_toggle_pressed = true;
            }
            KeyCode::Char('2') if is_pressed => {
                self.screenshot_pressed = true;
            }
            KeyCode::Char('3') if is_pressed => {
                self.palette_randomize_pressed = true;
            }
            KeyCode::Char('m') if is_pressed => {
                self.mute_toggle_pressed = true;
            }
            KeyCode::Char('8') if is_pressed => {
                self.volume_down_pressed = true;
            }
            KeyCode::Char('9') if is_pressed => {
                self.volume_up_pressed = true;
            }
            KeyCode::Char('-' | '_') if is_pressed => {
                self.scale_down_pressed = true;
            }
            KeyCode::Char('+' | '=') if is_pressed => {
                self.scale_up_pressed = true;
            }
            _ => {}
        }
    }
}
