/*!
 * Parse a text level definiton
 */

use std::collections::HashMap;

use anyhow::{Context, Result, bail, ensure};

use crate::color::Palette;

pub const TILE_SIZE: usize = 16;

pub const SPRING_HEIGHT: usize = 5;

// only the base of the spikes hurts, grazing the tips is fine
pub const SPIKE_HEIGHT: usize = 4;

// which way the spikes point, figured out from what they're stuck to
#[derive(Debug, Clone, Copy)]
pub enum Spike {
    Up,
    Down,
    Left,
    Right,
}

impl Spike {
    // (x, y, width, height) inside the tile
    pub fn hitbox(self) -> (usize, usize, usize, usize) {
        let (t, h) = (TILE_SIZE, SPIKE_HEIGHT);
        match self {
            Spike::Up => (0, t - h, t, h),
            Spike::Down => (0, 0, t, h),
            Spike::Right => (0, 0, h, t),
            Spike::Left => (t - h, 0, h, t),
        }
    }

    // celeste reference ig (doesnt kill you when you're moving in their direction)
    pub fn kills(self, vx: f32, vy: f32) -> bool {
        match self {
            Spike::Up => vy >= 0.0,
            Spike::Down => vy <= 0.0,
            Spike::Right => vx <= 0.0,
            Spike::Left => vx >= 0.0,
        }
    }
}

#[derive(Debug)]
#[allow(unused)]
pub struct LevelDefinition {
    pub name: String,
    pub music: String,
    pub background: String,
    pub palette: Palette,
    pub width: usize,
    pub height: usize,
    pub spawn: (usize, usize),
    pub goal: Option<(usize, usize)>,
    pub blocks: Vec<Vec<bool>>,
    pub jumpthrus: Vec<Vec<bool>>,
    pub coins: Vec<(usize, usize)>,
    pub springs: Vec<(usize, usize)>,
    pub spikes: Vec<((usize, usize), Spike)>,
}

impl LevelDefinition {
    pub fn parse(text: &str) -> Result<Self> {
        let mut lines = text.lines().enumerate();
        let mut metadata = HashMap::new();

        for (index, line) in lines.by_ref() {
            let line_number = index + 1;

            // stop here if we hit the blank line
            if line.trim().is_empty() {
                break;
            }

            // parse metadata
            let (key, value) = line.split_once(':').with_context(|| {
                format!("line {}: expected metadata", line_number)
            })?;

            let (key, value) = (key.trim(), value.trim());
            metadata.insert(key, value);
        }

        let name = metadata["name"].to_string();
        let music = metadata["music"].to_string();
        let background = metadata["background"].to_string();
        let palette = parse_palette(metadata["palette"])?;

        let mut blocks: Vec<Vec<bool>> = Vec::new();
        let mut jumpthrus = Vec::new();
        let mut coins = Vec::new();
        let mut springs = Vec::new();
        let mut spikes = Vec::new();
        let mut spawn = None;
        let mut goal = None;

        // parse geometry - loop each row
        for (_, line) in lines {
            // loop each tile
            let mut row = Vec::new();
            let mut jumpthru_row = Vec::new();
            for (column, tile) in line.chars().enumerate() {
                let position = (column, blocks.len());
                jumpthru_row.push(tile == '-');

                // return true for collider, false for no collision
                row.push(match tile {
                    '.' | ' ' => {
                        // allow both period and empty space to be "air"
                        false
                    }
                    'C' => {
                        // C for coin lol
                        coins.push(position);
                        false
                    }
                    '-' => {
                        // jump thru, only solid from above (player handles it)
                        false
                    }
                    '^' => {
                        // boing
                        springs.push(position);
                        false
                    }
                    'X' => {
                        // spikes
                        spikes.push(position);
                        false
                    }
                    'S' => {
                        // spawn
                        ensure!(spawn.is_none(), "multiple spawns");
                        spawn = Some(position);
                        false
                    }
                    'F' => {
                        // goal
                        ensure!(goal.is_none(), "multiple finish");
                        goal = Some(position);
                        false
                    }
                    'P' | 'G' => {
                        // platform and ground are solid
                        true
                    }
                    c => {
                        // uh oh lol
                        bail!("unknown char in level: {}", c);
                    }
                });
            }

            blocks.push(row);
            jumpthrus.push(jumpthru_row);
        }

        let (width, height) = (blocks[0].len(), blocks.len());
        let spawn = spawn.context("no spawn found")?;

        // point spikes away from the block they're on
        let solid = |x: usize, y: usize| {
            blocks.get(y).and_then(|row| row.get(x)).copied().unwrap_or(false)
        };
        let spikes = spikes
            .into_iter()
            .map(|(x, y)| {
                let spike = if solid(x, y + 1) {
                    Spike::Up
                } else if y > 0 && solid(x, y - 1) {
                    Spike::Down
                } else if x > 0 && solid(x - 1, y) {
                    Spike::Right
                } else if solid(x + 1, y) {
                    Spike::Left
                } else {
                    Spike::Up
                };
                ((x, y), spike)
            })
            .collect();

        Ok(Self {
            name,
            music,
            background,
            palette,
            width,
            height,
            spawn,
            goal,
            blocks,
            jumpthrus,
            coins,
            springs,
            spikes,
        })
    }
}

fn parse_palette(value: &str) -> Result<Palette> {
    let colors = value
        .split(',')
        .map(str::trim)
        .map(|color| {
            ensure!(
                color.len() == 7 && color.starts_with('#'),
                "colors must use #rrggbb notation"
            );

            // parse the whole color string as hex, chopping off '#'
            let value = u32::from_str_radix(&color[1..], 16)
                .context("colors must contain hexadecimal digits")?;

            Ok([
                ((value >> 16) & 0xff) as u8,
                ((value >> 8) & 0xff) as u8,
                (value & 0xff) as u8,
            ])
        })
        .collect::<Result<Vec<_>>>()?;

    ensure!(colors.len() == 4, "expected exactly 4 colors");

    Ok(colors.try_into().unwrap())
}
