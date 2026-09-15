/*!
 * Camera position and world-to-screen translation
 */

use crate::player::Player;

// try to keep the player at 40% horizontally
const PLAYER_SCREEN_POSITION: f32 = 0.40;

#[derive(Default)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
}

impl Camera {
    pub fn follow(
        &mut self,
        player: &Player,
        level_width: f32,
        viewport_width: f32,
    ) {
        self.x = (player.x - viewport_width * PLAYER_SCREEN_POSITION)
            .clamp(0.0, (level_width - viewport_width).max(0.0));
    }

    pub fn world_to_screen(&self, x: f32, y: f32) -> (i32, i32) {
        (
            x.round() as i32 - self.x.round() as i32,
            y.round() as i32 - self.y.round() as i32,
        )
    }
}
