/*!
 * Parse a text level definiton
 */

use std::collections::HashMap;

use anyhow::{Context, Result, bail, ensure};

use crate::color::Palette;

pub const TILE_SIZE: usize = 16;

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
    pub coins: Vec<(usize, usize)>,
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
        let mut coins = Vec::new();
        let mut spawn = None;
        let mut goal = None;

        // parse geometry - loop each row
        for (_, line) in lines {
            // loop each tile
            let mut row = Vec::new();
            for (column, tile) in line.chars().enumerate() {
                let position = (column, blocks.len());

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
        }

        let (width, height) = (blocks[0].len(), blocks.len());
        let spawn = spawn.context("no spawn found")?;

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
            coins,
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
