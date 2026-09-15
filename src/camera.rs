/*!
 * Camera position and world-to-screen translation
 */

use crate::player::Player;

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
        player_width: f32,
    ) {
        self.x = (player.x + player_width / 2.0 - viewport_width / 2.0)
            .clamp(0.0, (level_width - viewport_width).max(0.0));
    }

    pub fn world_to_screen(&self, x: f32, y: f32) -> (i32, i32) {
        (
            x.round() as i32 - self.x.round() as i32,
            y.round() as i32 - self.y.round() as i32,
        )
    }
}
