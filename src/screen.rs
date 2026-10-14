/*!
 * The different screens the game can display.
 */

use anyhow::Result;

use crate::camera::Camera;
use crate::editor::Editor;
use crate::level::{LevelDefinition, TILE_SIZE};
use crate::player::Player;

pub enum Screen {
    // title screen has a level underneath it
    Title(LevelScreen),
    Level(LevelScreen),
    // end screen has the final level underneath it
    End(LevelScreen),
    Editor(Editor),
}

// crumble blocks shake, then they're gone for a bit
#[derive(Clone, Copy)]
pub enum Crumble {
    Intact,
    Shaking(f32),
    Gone(f32),
}

pub struct LevelScreen {
    pub index: usize,
    pub level: LevelDefinition,
    pub player: Player,
    pub camera: Camera,
    pub remaining_coins: Vec<(usize, usize)>,
    pub springs: Vec<f32>,
    pub solid: Vec<Vec<bool>>,
    pub crumbles: Vec<Crumble>,
    pub mover_time: f32,
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
        let springs = vec![f32::INFINITY; level.springs.len()];
        let crumbles = vec![Crumble::Intact; level.crumbles.len()];
        let mut solid = level.blocks.clone();
        for &(x, y) in &level.crumbles {
            solid[y][x] = true;
        }

        Ok(Self {
            index,
            level,
            player,
            camera: Camera::default(),
            remaining_coins,
            springs,
            solid,
            crumbles,
            mover_time: 0.0,
            coin_count: 0,
            elapsed: 0.0,
        })
    }

    // dying puts all the crumble blocks back
    pub fn reset_crumbles(&mut self) {
        for (&(x, y), crumble) in
            self.level.crumbles.iter().zip(&mut self.crumbles)
        {
            *crumble = Crumble::Intact;
            self.solid[y][x] = true;
        }
    }
}
