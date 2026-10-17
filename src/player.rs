/*!
 * Player state and movement
 */

use crate::input::Input;
use crate::level::TILE_SIZE;

const SPEED: f32 = 120.0;
const GROUND_ACCELERATION: f32 = 1200.0;
const AIR_ACCELERATION: f32 = 650.0;
const GRAVITY: f32 = 800.0;
const JUMP_SPEED: f32 = 300.0;
const JUMP_RELEASE_SPEED: f32 = 100.0;
const MAX_FALL_SPEED: f32 = 450.0;
/// How long a jump press is remembered while the player is airborne, in
/// seconds.
const JUMP_BUFFER: f32 = 0.1;
const TILE_SIZE_F32: f32 = TILE_SIZE as f32;
pub const HITBOX_WIDTH: f32 = 12.0;
pub const HITBOX_HEIGHT: f32 = 15.0;

pub struct PlayerUpdate {
    pub jumped: bool,
}

pub enum PlayerFacing {
    Left,
    Right,
}

impl PlayerFacing {
    pub fn facing_left(&self) -> bool {
        matches!(self, Self::Left)
    }

    pub fn _facing_right(&self) -> bool {
        matches!(self, Self::Right)
    }
}

pub struct Player {
    pub x: f32,
    pub y: f32,
    pub vy: f32,
    pub vx: f32,
    pub grounded: bool,
    pub facing: PlayerFacing,
    pub spawn_x: f32,
    pub spawn_y: f32,
    jump_buffer: f32,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            vy: 0.0,
            vx: 0.0,
            grounded: false,
            facing: PlayerFacing::Right,
            spawn_x: x,
            spawn_y: y,
            jump_buffer: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.x = self.spawn_x;
        self.y = self.spawn_y;
        self.vx = 0.0;
        self.vy = 0.0;
        self.grounded = false;
        self.jump_buffer = 0.0;
    }

    pub fn overlaps(&self, x: f32, y: f32, width: f32, height: f32) -> bool {
        self.x < x + width
            && self.x + HITBOX_WIDTH > x
            && self.y < y + height
            && self.y + HITBOX_HEIGHT > y
    }

    pub fn update(
        &mut self,
        input: &Input,
        delta: f32,
        blocks: &[Vec<bool>],
        max_x: f32,
    ) -> PlayerUpdate {
        let direction = match (input.left, input.right) {
            (true, false) => {
                self.facing = PlayerFacing::Left;
                -1
            }
            (false, true) => {
                self.facing = PlayerFacing::Right;
                1
            }
            _ => 0,
        };

        // handle horizontal move first
        let target_speed = direction as f32 * SPEED;
        let acceleration =
            if self.grounded { GROUND_ACCELERATION } else { AIR_ACCELERATION };
        self.vx = approach(self.vx, target_speed, acceleration * delta);
        self.move_x(self.vx * delta, blocks, max_x);

        // Handle jump. A press is remembered for a short window: the collision
        // skin means that above ~200 fps the floor is only re-detected every
        // *other* frame, so a one-frame press often lands on a frame where
        // `grounded` is momentarily false. Buffering the press makes it fire on
        // the next grounded frame instead of being silently dropped.
        if input.jump_pressed {
            self.jump_buffer = JUMP_BUFFER;
        }
        let jumped = self.grounded && self.jump_buffer > 0.0;
        if jumped {
            self.vy = -JUMP_SPEED;
            self.grounded = false;
            self.jump_buffer = 0.0;
        }
        self.jump_buffer = (self.jump_buffer - delta).max(0.0);

        // releasing jump early cuts the remaining upward speed
        if !input.jump && self.vy < -JUMP_RELEASE_SPEED {
            self.vy = -JUMP_RELEASE_SPEED;
        }

        // handle vertical movement
        self.vy = (self.vy + GRAVITY * delta).min(MAX_FALL_SPEED);
        self.grounded = false;
        self.move_y(self.vy * delta, blocks);

        PlayerUpdate { jumped }
    }

    fn move_x(&mut self, amount: f32, blocks: &[Vec<bool>], max_x: f32) {
        self.x += amount;

        if hits_solid(self.x, self.y, blocks) {
            self.x = resolve_collision(self.x, amount, HITBOX_WIDTH);
            self.vx = 0.0;
        }

        let clamped_x = self.x.clamp(0.0, max_x);
        if clamped_x != self.x {
            self.x = clamped_x;
            self.vx = 0.0;
        }
    }

    fn move_y(&mut self, amount: f32, blocks: &[Vec<bool>]) {
        self.y += amount;

        if hits_solid(self.x, self.y, blocks) {
            self.y = resolve_collision(self.y, amount, HITBOX_HEIGHT);
            self.grounded = amount > 0.0;
            self.vy = 0.0;
        }
    }
}

// get to "target" from "current" changing by "amount"
fn approach(current: f32, target: f32, amount: f32) -> f32 {
    if current < target {
        (current + amount).min(target)
    } else {
        (current - amount).max(target)
    }
}

// reset the player based on what it collided with
fn resolve_collision(position: f32, amount: f32, player_size: f32) -> f32 {
    if amount > 0.0 {
        ((position + player_size) / TILE_SIZE_F32).floor() * TILE_SIZE_F32
            - player_size
            - 0.01
    } else if amount < 0.0 {
        (position / TILE_SIZE_F32).floor() * TILE_SIZE_F32
            + TILE_SIZE_F32
            + 0.01
    } else {
        position
    }
}

// check if the player hits a solid (should we store this?)
fn hits_solid(x: f32, y: f32, blocks: &[Vec<bool>]) -> bool {
    // find all blocks *around* the player
    let left = (x / TILE_SIZE_F32).floor() as i32;
    let right = ((x + HITBOX_WIDTH - 0.01) / TILE_SIZE_F32).floor() as i32;
    let top = (y / TILE_SIZE_F32).floor() as i32;
    let bottom = ((y + HITBOX_HEIGHT - 0.01) / TILE_SIZE_F32).floor() as i32;

    // check all blocks *around* the player
    (top.max(0)..=bottom).any(|y| {
        (left.max(0)..=right).any(|x| {
            // figure out if the block is solid - if we are out of bounds we
            // will get None at somepoint so just treat it as false.  if we make
            // it true then we can make the player stuck in the level
            blocks
                .get(y as usize)
                .and_then(|row| row.get(x as usize))
                .copied()
                .unwrap_or(false)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 10x4 level with only its bottom row solid.
    fn floor() -> Vec<Vec<bool>> {
        let mut blocks = vec![vec![false; 10]; 4];
        blocks[3].fill(true);
        blocks
    }

    #[test]
    fn a_jump_press_always_jumps_at_high_frame_rates() {
        // Above ~200 fps the collision skin means the floor is only
        // re-detected every *other* frame, so a one-frame jump press lands on
        // a non-grounded frame half the time. The press must still jump.
        let blocks = floor();
        let max_x = 10.0 * TILE_SIZE as f32 - HITBOX_WIDTH;
        let delta = 1.0 / 240.0;
        let idle = Input::default();
        let mut holding = Input::default();
        holding.jump = true;
        let mut press = Input::default();
        press.jump = true;
        press.jump_pressed = true;

        // Try both parities of the frame the press lands on: the bug depends
        // on it, since the floor is re-detected on alternating frames.
        for phase in 0..2 {
            let mut player = Player::new(0.0, 0.0);
            for _ in 0..1000 {
                player.update(&idle, delta, &blocks, max_x);
                if player.grounded {
                    break;
                }
            }
            assert!(player.grounded, "phase {phase}: the player lands");
            let rest = player.y;

            for _ in 0..phase {
                player.update(&idle, delta, &blocks, max_x);
            }
            player.update(&press, delta, &blocks, max_x);

            let mut peak = player.y;
            for _ in 0..24 {
                player.update(&holding, delta, &blocks, max_x);
                peak = peak.min(player.y);
            }

            assert!(
                peak < rest - 4.0,
                "phase {phase}: the press was dropped (rest {rest}, peak {peak})"
            );
        }
    }
}
