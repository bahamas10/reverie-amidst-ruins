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
const COYOTE_TIME: f32 = 0.1; // behaves the same at any fps (uses timers), ~6 frames at 60fps
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
    pub coyote_timer: f32,
    pub facing: PlayerFacing,
    pub spawn_x: f32,
    pub spawn_y: f32,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            vy: 0.0,
            vx: 0.0,
            grounded: false,
            coyote_timer: 0.0,
            facing: PlayerFacing::Right,
            spawn_x: x,
            spawn_y: y,
        }
    }

    pub fn reset(&mut self) {
        self.x = self.spawn_x;
        self.y = self.spawn_y;
        self.vx = 0.0;
        self.vy = 0.0;
        self.grounded = false;
        self.coyote_timer = 0.0;
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

        // handle jump & coyote
        let jumped = input.jump_pressed && self.coyote_timer > 0.0;
        if jumped {
            self.vy = -JUMP_SPEED;
            self.grounded = false;
            self.coyote_timer = 0.0;
        }

        // releasing jump early cuts the remaining upward speed
        if !input.jump && self.vy < -JUMP_RELEASE_SPEED {
            self.vy = -JUMP_RELEASE_SPEED;
        }

        // handle vertical movement
        self.vy = (self.vy + GRAVITY * delta).min(MAX_FALL_SPEED);
        self.grounded = false;
        self.move_y(self.vy * delta, blocks);

        // refill coyote time while grounded, drain it while in air
        self.coyote_timer = if self.grounded { COYOTE_TIME } else { (self.coyote_timer - delta).max(0.0) };

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
