/*!
 * The different screens the game can display.
 */

use anyhow::Result;

use crate::camera::Camera;
use crate::level::{LevelDefinition, TILE_SIZE};
use crate::player::Player;

pub enum Screen {
    // title screen has a level underneath it
    Title(LevelScreen),
    Level(LevelScreen),
}

pub struct LevelScreen {
    pub index: usize,
    pub level: LevelDefinition,
    pub player: Player,
    pub camera: Camera,
    pub remaining_coins: Vec<(usize, usize)>,
    pub coin_count: usize,
    pub elapsed: f32,
}

impl LevelScreen {
    pub fn load(index: usize, source: &str) -> Result<Self> {
        let level = LevelDefinition::parse(source)?;
        let player = Player::new(
            (level.spawn.0 * TILE_SIZE) as f32,
            (level.spawn.1 * TILE_SIZE) as f32,
        );
        let remaining_coins = level.coins.clone();

        Ok(Self {
            index,
            level,
            player,
            camera: Camera::default(),
            remaining_coins,
            coin_count: 0,
            elapsed: 0.0,
        })
    }
}
