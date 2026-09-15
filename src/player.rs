/*!
 * Player state and movement
 */

use crate::input::Input;

const SPEED: f32 = 120.0;

pub struct Player {
    pub x: f32,
    pub y: f32,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn update(&mut self, input: &Input, delta: f32, max_x: f32) {
        let direction = match (input.left, input.right) {
            (true, false) => -1,
            (false, true) => 1,
            _ => 0,
        };

        self.x += direction as f32 * SPEED * delta;

        // should this be here? i think when we add colision this might change?
        self.x = self.x.clamp(0.0, max_x);
    }
}
