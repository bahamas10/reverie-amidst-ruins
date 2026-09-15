/*!
 * Player state and movement
 */

use crate::input::Input;
use crate::level::TILE_SIZE;

const SPEED: f32 = 120.0;
const GRAVITY: f32 = 800.0;
const JUMP_SPEED: f32 = 300.0;
const MAX_FALL_SPEED: f32 = 450.0;
const SIZE: f32 = TILE_SIZE as f32;

pub struct Player {
    pub x: f32,
    pub y: f32,
    pub vy: f32,
    pub vx: f32,
    pub grounded: bool,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y, vy: 0.0, vx: 0.0, grounded: false }
    }

    pub fn update(
        &mut self,
        input: &Input,
        delta: f32,
        blocks: &[Vec<bool>],
        max_x: f32,
    ) {
        let direction = match (input.left, input.right) {
            (true, false) => -1,
            (false, true) => 1,
            _ => 0,
        };

        // handle horizontal move first
        self.move_x(direction as f32 * SPEED * delta, blocks, max_x);

        // handle jump
        if input.jump_pressed && self.grounded {
            self.vy = -JUMP_SPEED;
            self.grounded = false;
        }

        // handle vertical movement
        self.vy = (self.vy + GRAVITY * delta).min(MAX_FALL_SPEED);
        self.grounded = false;
        self.move_y(self.vy * delta, blocks);
    }

    fn move_x(&mut self, amount: f32, blocks: &[Vec<bool>], max_x: f32) {
        self.x += amount;

        if hits_solid(self.x, self.y, blocks) {
            self.x = resolve_collision(self.x, amount);
        }

        self.x = self.x.clamp(0.0, max_x);
    }

    fn move_y(&mut self, amount: f32, blocks: &[Vec<bool>]) {
        self.y += amount;

        if hits_solid(self.x, self.y, blocks) {
            self.y = resolve_collision(self.y, amount);
            self.grounded = amount > 0.0;
            self.vy = 0.0;
        }
    }
}

fn resolve_collision(position: f32, amount: f32) -> f32 {
    // reset the player (quarter steps?)
    if amount > 0.0 {
        ((position + SIZE) / SIZE).floor() * SIZE - SIZE - 0.01
    } else if amount < 0.0 {
        (position / SIZE).floor() * SIZE + SIZE + 0.01
    } else {
        position
    }
}

fn hits_solid(x: f32, y: f32, blocks: &[Vec<bool>]) -> bool {
    // find all blocks *around* the player
    let left = (x / SIZE).floor() as i32;
    let right = ((x + SIZE - 0.01) / SIZE).floor() as i32;
    let top = (y / SIZE).floor() as i32;
    let bottom = ((y + SIZE - 0.01) / SIZE).floor() as i32;

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
