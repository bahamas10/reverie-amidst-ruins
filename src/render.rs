/*!
 * Basic renderer - this will generate an image from the game state (eventually)
 * that can then be displayed by *anything* that can display an image.  in our
 * case that will be the kitty terminal protocol.
 */

use anyhow::{Context, Result, bail};
use image::{RgbaImage, imageops};

use crate::camera::Camera;
use crate::level::{LevelDefinition, Palette, TILE_SIZE};
use crate::player::{HITBOX_HEIGHT, HITBOX_WIDTH, Player};

// animations
const COIN_ANIMATION_NUM: f32 = 7.0; // rotations per second

// 0 = stationary, 1 = moved with level
const BACKGROUND_PARALLAX_NUM: f32 = 0.35;

// full-screen post-processing
const VIGNETTE_START: f32 = 0.20;
const VIGNETTE_STRENGTH: f32 = 0.45;
const SCANLINE_BRIGHTNESS: f32 = 1.55;

// locations of the tiles in tileset PNG
const GROUND: (usize, usize) = (0, 0);
const GROUND_TOP: (usize, usize) = (16, 0);
const COIN_FRAMES: [(usize, usize); 4] = [(32, 0), (48, 0), (64, 0), (48, 0)];
const PLAYER: (usize, usize) = (80, 0);
const GOAL: (usize, usize) = (96, 0);

// images
const BACKGROUND_STORM: &[u8] =
    include_bytes!("../assets/backgrounds/storm.png");
const BACKGROUND_SNOW: &[u8] = include_bytes!("../assets/backgrounds/snow.png");
const BACKGROUND_NEBULA: &[u8] =
    include_bytes!("../assets/backgrounds/nebula.png");
const TITLE: &[u8] = include_bytes!("../assets/backgrounds/title.png");
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

pub fn title(frame: &mut RgbaImage) -> Result<()> {
    let title = image::load_from_memory(TITLE)
        .context("could not decode title.png")?
        .into_rgba8();

    imageops::overlay(frame, &title, 0, 0);

    Ok(())
}

/**
 * This is so cool - because our frames are just images we can apply CPU
 * "shaders" to them... basically just filters and image effects.
 *
 * Keep this one simple - add a vignette and some super generic scanlines.
 */
pub fn post_process(frame: &mut RgbaImage) {
    let width = frame.width().saturating_sub(1).max(1) as f32;
    let height = frame.height().saturating_sub(1).max(1) as f32;

    for (x, y, pixel) in frame.enumerate_pixels_mut() {
        // normalize values to be -1 <= value <= 1
        let x = x as f32 / width * 2.0 - 1.0;
        let y_normalized = y as f32 / height * 2.0 - 1.0;

        // vignette
        let distance =
            ((x * x + y_normalized * y_normalized) / 2.0).clamp(0.0, 1.0);
        let vignette = ((distance - VIGNETTE_START) / (1.0 - VIGNETTE_START))
            .clamp(0.0, 1.0);
        let mut brightness = 1.0 - vignette * VIGNETTE_STRENGTH;

        // scanlines
        if y % 2 == 1 {
            brightness *= SCANLINE_BRIGHTNESS;
        }

        // apply the change to the pixel
        for channel in &mut pixel.0[..3] {
            *channel = (*channel as f32 * brightness).round() as u8;
        }
    }
}

// convert the level to an image
pub fn level(
    level: &LevelDefinition,
    player: Option<&Player>,
    camera: &Camera,
    coins: &[(usize, usize)],
    elapsed: f32,
    width: u32,
    height: u32,
) -> Result<RgbaImage> {
    // load the background and get it ready to parallax
    let background_data = match level.background.as_str() {
        "snow" => BACKGROUND_SNOW,
        "nebula" => BACKGROUND_NEBULA,
        "storm" => BACKGROUND_STORM,
        name => bail!("unknown background: {name}"),
    };
    let background = image::load_from_memory(background_data)
        .with_context(|| {
            format!("could not decode background: {}", level.background)
        })?
        .into_rgba8();
    let background_width = background.width();
    let background_height = background.height();

    let background_x = (camera.x * BACKGROUND_PARALLAX_NUM)
        .round()
        .rem_euclid(background_width as f32) as u32;
    let background_y = (camera.y * BACKGROUND_PARALLAX_NUM)
        .round()
        .rem_euclid(background_height as f32) as u32;

    // construct the base frame using the background, parallaxed - this will
    // wrap the image in both x and y directions which is pretty fancy
    let mut frame = RgbaImage::from_fn(width, height, |x, y| {
        *background.get_pixel(
            (x + background_x) % background_width,
            (y + background_y) % background_height,
        )
    });

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
                false,
            );
        }
    }

    // loop through the coin sprites and animate them
    let coin = COIN_FRAMES
        [(elapsed * COIN_ANIMATION_NUM) as usize % COIN_FRAMES.len()];
    for &(x, y) in coins {
        sprite(
            &mut frame,
            &tileset,
            coin,
            camera.world_to_screen(
                (x * TILE_SIZE) as f32,
                (y * TILE_SIZE) as f32,
            ),
            &level.palette,
            false,
        );
    }

    // draw goal
    sprite(
        &mut frame,
        &tileset,
        GOAL,
        camera.world_to_screen(
            (level.goal.0 * TILE_SIZE) as f32,
            (level.goal.1 * TILE_SIZE) as f32,
        ),
        &level.palette,
        false,
    );

    // draw player - it's smaller
    if let Some(player) = player {
        sprite_sized(
            &mut frame,
            &tileset,
            PLAYER,
            camera.world_to_screen(player.x, player.y),
            &SOURCE_PALETTE,
            player.facing.facing_left(),
            (HITBOX_WIDTH as u32, HITBOX_HEIGHT as u32),
        );
    }

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
    flip_x: bool,
) {
    sprite_sized(
        frame,
        tileset,
        source,
        destination,
        palette,
        flip_x,
        (TILE_SIZE as u32, TILE_SIZE as u32),
    )
}

fn sprite_sized(
    frame: &mut RgbaImage,
    tileset: &RgbaImage,
    source: (usize, usize),
    destination: (i32, i32),
    palette: &Palette,
    flip_x: bool,
    size: (u32, u32),
) {
    // get the tile from the tileset
    let mut tile = imageops::crop_imm(
        tileset,
        source.0 as u32,
        source.1 as u32,
        size.0,
        size.1,
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

    if flip_x {
        imageops::flip_horizontal_in_place(&mut tile);
    }

    imageops::overlay(frame, &tile, destination.0.into(), destination.1.into());
}
