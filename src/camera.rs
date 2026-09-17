/*!
 * Camera position and world-to-screen translation
 */

use crate::player::Player;

// center of the camera's dead zone - the levels are *mostly* left-to-right so
// we bias seeing more on the right
const PLAYER_SCREEN_X: f32 = 0.40;
const PLAYER_SCREEN_Y: f32 = 0.65;

// dead zone size in pixels
const DEAD_ZONE_HALF_WIDTH: f32 = 24.0;
const DEAD_ZONE_HALF_HEIGHT: f32 = 40.0;

// this seems fast enough
const FOLLOW_SPEED: f32 = 10.0;

#[derive(Default)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
}

impl Camera {
    pub fn snap_to(
        &mut self,
        player: &Player,
        level_width: f32,
        level_height: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        self.x = (player.x - viewport_width * PLAYER_SCREEN_X)
            .clamp(0.0, (level_width - viewport_width).max(0.0));
        self.y = (player.y - viewport_height * PLAYER_SCREEN_Y)
            .clamp(0.0, (level_height - viewport_height).max(0.0));
    }

    pub fn follow(
        &mut self,
        player: &Player,
        level_width: f32,
        level_height: f32,
        viewport_width: f32,
        viewport_height: f32,
        delta: f32,
    ) {
        let center_x = viewport_width * PLAYER_SCREEN_X;
        let center_y = viewport_height * PLAYER_SCREEN_Y;
        let left = center_x - DEAD_ZONE_HALF_WIDTH;
        let right = center_x + DEAD_ZONE_HALF_WIDTH;
        let top = center_y - DEAD_ZONE_HALF_HEIGHT;
        let bottom = center_y + DEAD_ZONE_HALF_HEIGHT;

        let player_x = player.x - self.x;
        let player_y = player.y - self.y;

        let target_x = if player_x < left {
            player.x - left
        } else if player_x > right {
            player.x - right
        } else {
            self.x
        };

        let target_y = if player_y < top {
            player.y - top
        } else if player_y > bottom {
            player.y - bottom
        } else {
            self.y
        };

        let max_x = (level_width - viewport_width).max(0.0);
        let max_y = (level_height - viewport_height).max(0.0);
        let target_x = target_x.clamp(0.0, max_x);
        let target_y = target_y.clamp(0.0, max_y);

        // smooth it out
        let amount = 1.0 - (-FOLLOW_SPEED * delta).exp();
        self.x += (target_x - self.x) * amount;
        self.y += (target_y - self.y) * amount;
    }

    pub fn world_to_screen(&self, x: f32, y: f32) -> (i32, i32) {
        (
            x.round() as i32 - self.x.round() as i32,
            y.round() as i32 - self.y.round() as i32,
        )
    }
}
