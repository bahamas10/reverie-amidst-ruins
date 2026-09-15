/*!
 * Basic renderer - this will generate an image from the game state (eventually)
 * that can then be displayed by *anything* that can display an image.  in our
 * case that will be the kitty terminal protocol.
 */

use anyhow::{Context, Result};
use image::{RgbaImage, imageops};

use crate::camera::Camera;
use crate::level::{LevelDefinition, Palette};
use crate::player::Player;

// size of the tiles in the tileset + the size we use to render them
pub const TILE_SIZE: usize = 16;

// animations
const COIN_ANIMATION_NUM: f32 = 8.0; // 8 rotations per second

// locations of the tiles in tileset PNG
const GROUND: (usize, usize) = (0, 0);
const GROUND_TOP: (usize, usize) = (16, 0);
const COIN_FRAMES: [(usize, usize); 4] = [(32, 0), (48, 0), (64, 0), (48, 0)];
const PLAYER: (usize, usize) = (80, 0);
const GOAL: (usize, usize) = (112, 0);

// images
const BACKGROUND: &[u8] = include_bytes!("../assets/backgrounds/snow.png");
const TILESET: &[u8] = include_bytes!("../assets/tiles/tileset.png");

// ok, i don't know how i feel about this currently, but this is the exact
// colors used in the tileset.  we find those colors and treat those as indices
// - this means we can do "palette swaps" without adding new graphics.  the
// palette data will be embedded in the level text
const SOURCE_PALETTE: Palette = [
    [0x07, 0x18, 0x21],
    [0x16, 0x4b, 0x63],
    [0x11, 0xd9, 0xc5],
    [0xf2, 0xff, 0x66],
];

// convert the level to an image
pub fn level(
    level: &LevelDefinition,
    player: &Player,
    camera: &Camera,
    elapsed: f32,
) -> Result<RgbaImage> {
    // the background *is* our frame - eventually this should probably be a
    // black image that we blit over
    let mut frame = image::load_from_memory(BACKGROUND)
        .context("could not decode snow.png")?
        .into_rgba8();

    let tileset = image::load_from_memory(TILESET)
        .context("could not decode tileset.png")?
        .into_rgba8();

    // loop all the blocks
    for (y, row) in level.blocks.iter().enumerate() {
        for (x, &solid) in row.iter().enumerate() {
            // don't render air
            if !solid {
                continue;
            }

            // idk if this is the best idea, but basically we have 2 tiles for
            // ground, and which one is used is determined in code and not in
            // the txt file
            let block = if y == 0 || !level.blocks[y - 1][x] {
                GROUND_TOP
            } else {
                GROUND
            };

            // draw the sprite onte the frame
            sprite(
                &mut frame,
                &tileset,
                block,
                camera.world_to_screen(
                    (x * TILE_SIZE) as f32,
                    (y * TILE_SIZE) as f32,
                ),
                &level.palette,
            );
        }
    }

    // loop through the coin sprites and animate them
    let coin = COIN_FRAMES
        [(elapsed * COIN_ANIMATION_NUM) as usize % COIN_FRAMES.len()];
    for &(x, y) in &level.coins {
        sprite(
            &mut frame,
            &tileset,
            coin,
            camera.world_to_screen(
                (x * TILE_SIZE) as f32,
                (y * TILE_SIZE) as f32,
            ),
            &level.palette,
        );
    }

    // draw player and spawn
    sprite(
        &mut frame,
        &tileset,
        GOAL,
        camera.world_to_screen(
            (level.goal.0 * TILE_SIZE) as f32,
            (level.goal.1 * TILE_SIZE) as f32,
        ),
        &level.palette,
    );
    sprite(
        &mut frame,
        &tileset,
        PLAYER,
        camera.world_to_screen(player.x, player.y),
        &level.palette,
    );

    Ok(frame)
}

/**
 * Helper to draw a single sprite to an image object
 */
fn sprite(
    frame: &mut RgbaImage,
    tileset: &RgbaImage,
    source: (usize, usize),
    destination: (i32, i32),
    palette: &Palette,
) {
    // get the tile from the tileset
    let mut tile = imageops::crop_imm(
        tileset,
        source.0 as u32,
        source.1 as u32,
        TILE_SIZE as u32,
        TILE_SIZE as u32,
    )
    .to_image();

    // copy it PIXEL BY PIXEL. yes actually, because we want to translate the
    // color palette
    // is this slow?
    for pixel in tile.pixels_mut() {
        // find the color in the "source" color palette to get the index
        let pos = SOURCE_PALETTE.iter().position(|c| pixel.0[..3] == c[..]);

        // translate the color, or make it transparent if it fails
        match pos {
            Some(idx) => pixel.0[..3].copy_from_slice(&palette[idx]),
            None => pixel.0 = [0, 0, 0, 0],
        };
    }

    imageops::overlay(frame, &tile, destination.0.into(), destination.1.into());
}
