/*!
 * Handle input from any source (keyboard or gamepad currently) and abstract it
 * into a simple struct
 */

use crossterm::event::KeyEvent;

use gamepad::Gamepad;
use keyboard::Keyboard;

mod gamepad;
mod keyboard;

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

    // just pressed debug toggle
    pub debug_toggle_pressed: bool,

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

    // all input methods are stored here
    keyboard: Keyboard,
    gamepad: Gamepad,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, key_events: &[KeyEvent]) {
        // update the keyboard using terminal events
        let keyboard = self.keyboard.update(key_events);

        // update the gamepad
        let gamepad = self.gamepad.update();

        // held buttons remain active while either input source is held
        self.left = keyboard.left || gamepad.left;
        self.right = keyboard.right || gamepad.right;
        self.jump = keyboard.jump || gamepad.jump;
        self.confirm = keyboard.confirm || gamepad.confirm;
        self.quit = keyboard.quit || gamepad.quit;

        // one-shot buttons are reset / recomputed every frame
        self.jump_pressed = keyboard.jump_pressed || gamepad.jump_pressed;
        self.confirm_pressed =
            keyboard.confirm_pressed || gamepad.confirm_pressed;
        self.screenshot_pressed =
            keyboard.screenshot_pressed || gamepad.screenshot_pressed;
        self.effects_toggle_pressed =
            keyboard.effects_toggle_pressed || gamepad.effects_toggle_pressed;
        self.palette_randomize_pressed = keyboard.palette_randomize_pressed
            || gamepad.palette_randomize_pressed;
        self.debug_toggle_pressed =
            keyboard.debug_toggle_pressed || gamepad.debug_toggle_pressed;
        self.restart_pressed =
            keyboard.restart_pressed || gamepad.restart_pressed;
        self.mute_toggle_pressed =
            keyboard.mute_toggle_pressed || gamepad.mute_toggle_pressed;
        self.volume_down_pressed =
            keyboard.volume_down_pressed || gamepad.volume_down_pressed;
        self.volume_up_pressed =
            keyboard.volume_up_pressed || gamepad.volume_up_pressed;
        self.scale_down_pressed =
            keyboard.scale_down_pressed || gamepad.scale_down_pressed;
        self.scale_up_pressed =
            keyboard.scale_up_pressed || gamepad.scale_up_pressed;
    }
}

/// This object is returned via oll of the input update() methosd
#[derive(Clone, Copy, Default)]
struct InputState {
    left: bool,
    right: bool,
    jump: bool,
    jump_pressed: bool,
    confirm: bool,
    confirm_pressed: bool,
    screenshot_pressed: bool,
    effects_toggle_pressed: bool,
    palette_randomize_pressed: bool,
    debug_toggle_pressed: bool,
    restart_pressed: bool,
    mute_toggle_pressed: bool,
    volume_down_pressed: bool,
    volume_up_pressed: bool,
    scale_down_pressed: bool,
    scale_up_pressed: bool,
    quit: bool,
}

impl InputState {
    /// Clear just the one-shot button presses
    fn clear_pressed(&mut self) {
        self.jump_pressed = false;
        self.confirm_pressed = false;
        self.screenshot_pressed = false;
        self.effects_toggle_pressed = false;
        self.palette_randomize_pressed = false;
        self.debug_toggle_pressed = false;
        self.restart_pressed = false;
        self.mute_toggle_pressed = false;
        self.volume_down_pressed = false;
        self.volume_up_pressed = false;
        self.scale_down_pressed = false;
        self.scale_up_pressed = false;
    }
}
