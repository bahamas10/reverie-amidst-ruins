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
                self.jump_pressed = is_pressed;
                self.jump = is_down;
            }
            KeyCode::Esc | KeyCode::Char('q' | 'Q') => self.quit = is_down,
            KeyCode::Enter => self.confirm = is_down,
            _ => {}
        }
    }
}
