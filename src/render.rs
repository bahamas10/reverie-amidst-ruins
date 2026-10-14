/*!
 * Basic renderer - this will generate an image from the game state (eventually)
 * that can then be displayed by *anything* that can display an image.  in our
 * case that will be the kitty terminal protocol.
 */

use anyhow::{Context, Result, bail};
use image::{RgbaImage, imageops};

use crate::color::{Color, Palette};
use crate::debug::Overlays;
use crate::level::{SPRING_HEIGHT, Spike, TILE_SIZE};
use crate::player::{HITBOX_HEIGHT, HITBOX_WIDTH, Player};
use crate::screen::{Crumble, LevelScreen};

// animations
const COIN_ANIMATION_NUM: f32 = 7.0; // rotations per second

// 0 = stationary, 1 = moved with level
const BACKGROUND_PARALLAX_NUM: f32 = 0.35;

// full-screen post-processing
const VIGNETTE_START: f32 = 0.20;
const VIGNETTE_STRENGTH: f32 = 0.45;
const SCANLINE_BRIGHTNESS: f32 = 1.55;

// sprite drop shadows
const SHADOW_OFFSET: (i32, i32) = (3, 3);
const SHADOW_OPACITY: u8 = 50;

// debug hitboxes
const HITBOX_HURTBOX: Color = [0xff, 0x30, 0x30];
const HITBOX_PICKUP: Color = [0xff, 0xe0, 0x30];
const HITBOX_BLOCK: Color = [0x30, 0x80, 0xff];
const HITBOX_ENTITY: Color = [0x30, 0xff, 0x60];
const HITBOX_FILL_OPACITY: f32 = 0.25;

// locations of the tiles in tileset PNG
const GROUND: (usize, usize) = (0, 0);
const GROUND_TOP: (usize, usize) = (16, 0);
const COIN_FRAMES: [(usize, usize); 4] = [(32, 0), (48, 0), (64, 0), (48, 0)];
const PLAYER: (usize, usize) = (80, 0);
const GOAL: (usize, usize) = (96, 0);
const SPRING: (usize, usize) = (112, 0);
const JUMPTHRU_LEFT: (usize, usize) = (192, 0);
const JUMPTHRU: (usize, usize) = (208, 0);
const JUMPTHRU_RIGHT: (usize, usize) = (224, 0);
const SPIKE_UP: (usize, usize) = (240, 0);
const SPIKE_DOWN: (usize, usize) = (256, 0);
const SPIKE_RIGHT: (usize, usize) = (272, 0);
const SPIKE_LEFT: (usize, usize) = (288, 0);
const CRUMBLE: (usize, usize) = (304, 0);

// how tall the jump thru plank looks (only the top edge is solid)
const JUMPTHRU_HEIGHT: i32 = 5;

// spring shoots up then goes back down in steps, (frame, seconds)
const SPRING_FRAMES: [((usize, usize), f32); 4] =
    [((128, 0), 0.08), ((144, 0), 0.05), ((160, 0), 0.05), ((176, 0), 0.05)];

// images
const BACKGROUND_STORM: &[u8] =
    include_bytes!("../assets/backgrounds/storm.png");
const BACKGROUND_SNOW: &[u8] = include_bytes!("../assets/backgrounds/snow.png");
const BACKGROUND_NEBULA: &[u8] =
    include_bytes!("../assets/backgrounds/nebula.png");
const TITLE: &[u8] = include_bytes!("../assets/backgrounds/title.png");
const END: &[u8] = include_bytes!("../assets/backgrounds/end.png");
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

/// Render "title" screen
pub fn title(frame: &mut RgbaImage, selected: usize) -> Result<()> {
    overlay_image(frame, TITLE)?;
    for (i, option) in ["PLAY", "EDITOR"].iter().enumerate() {
        let y = 130 + i as i32 * 13;
        if i == selected {
            text(frame, &format!("> {option} <"), y, 1, [255; 3]);
        } else {
            text(frame, option, y, 1, [0x80; 3]);
        }
    }
    Ok(())
}

/// Render "end" screen
pub fn end(frame: &mut RgbaImage) -> Result<()> {
    overlay_image(frame, END)
}

fn overlay_image(frame: &mut RgbaImage, bytes: &[u8]) -> Result<()> {
    let img = image::load_from_memory(bytes)
        .context("could not decode image")?
        .into_rgba8();

    imageops::overlay(frame, &img, 0, 0);

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
    screen: &LevelScreen,
    player: Option<&Player>,
    width: u32,
    height: u32,
) -> Result<RgbaImage> {
    let (level, camera) = (&screen.level, &screen.camera);
    let (coins, springs) = (&screen.remaining_coins, &screen.springs);
    let elapsed = screen.elapsed;

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

    // jump thrus get end caps, same idea as the ground tops
    for (y, row) in level.jumpthrus.iter().enumerate() {
        for x in (0..row.len()).filter(|&x| row[x]) {
            let left = x > 0 && row[x - 1];
            let right = row.get(x + 1) == Some(&true);
            let tile = match (left, right) {
                (false, true) => JUMPTHRU_LEFT,
                (true, false) => JUMPTHRU_RIGHT,
                _ => JUMPTHRU,
            };
            sprite(
                &mut frame,
                &tileset,
                tile,
                camera.world_to_screen(
                    (x * TILE_SIZE) as f32,
                    (y * TILE_SIZE) as f32,
                ),
                &level.palette,
                false,
            );
        }
    }

    // crumble blocks wiggle while shaking, gone ones aren't drawn
    for (&(x, y), crumble) in level.crumbles.iter().zip(&screen.crumbles) {
        let shake = match crumble {
            Crumble::Intact => 0,
            Crumble::Shaking(time) => (time * 60.0) as i32 % 3 - 1,
            Crumble::Gone(_) => continue,
        };
        let (sx, sy) = camera
            .world_to_screen((x * TILE_SIZE) as f32, (y * TILE_SIZE) as f32);
        sprite(
            &mut frame,
            &tileset,
            CRUMBLE,
            (sx + shake, sy),
            &level.palette,
            false,
        );
    }

    // moving platforms use the jump thru planks
    for mover in &level.movers {
        let (mx, my) = mover.position(screen.mover_time);
        for i in 0..mover.width {
            let tile = match (i == 0, i + 1 == mover.width) {
                (true, false) => JUMPTHRU_LEFT,
                (false, true) => JUMPTHRU_RIGHT,
                _ => JUMPTHRU,
            };
            sprite(
                &mut frame,
                &tileset,
                tile,
                camera.world_to_screen(mx + (i * TILE_SIZE) as f32, my),
                &level.palette,
                false,
            );
        }
    }

    for &((x, y), spike) in &level.spikes {
        let tile = match spike {
            Spike::Up => SPIKE_UP,
            Spike::Down => SPIKE_DOWN,
            Spike::Right => SPIKE_RIGHT,
            Spike::Left => SPIKE_LEFT,
        };
        sprite(
            &mut frame,
            &tileset,
            tile,
            camera.world_to_screen(
                (x * TILE_SIZE) as f32,
                (y * TILE_SIZE) as f32,
            ),
            &level.palette,
            false,
        );
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

    for (&(x, y), &since) in level.springs.iter().zip(springs) {
        sprite(
            &mut frame,
            &tileset,
            spring_frame(since),
            camera.world_to_screen(
                (x * TILE_SIZE) as f32,
                (y * TILE_SIZE) as f32,
            ),
            &level.palette,
            false,
        );
    }

    // draw goal
    if let Some((x, y)) = level.goal {
        sprite(
            &mut frame,
            &tileset,
            GOAL,
            camera.world_to_screen(
                (x * TILE_SIZE) as f32,
                (y * TILE_SIZE) as f32,
            ),
            &level.palette,
            false,
        );
    }

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

fn spring_frame(mut since: f32) -> (usize, usize) {
    for (frame, time) in SPRING_FRAMES {
        if since < time {
            return frame;
        }
        since -= time;
    }
    SPRING
}

// debug hitboxes on top of the level
pub fn hitboxes(
    frame: &mut RgbaImage,
    screen: &LevelScreen,
    overlays: &Overlays,
) {
    let (level, player, camera) =
        (&screen.level, &screen.player, &screen.camera);
    let coins = &screen.remaining_coins;
    let tile = (TILE_SIZE as i32, TILE_SIZE as i32);
    let at = |(x, y): (usize, usize)| {
        camera.world_to_screen((x * TILE_SIZE) as f32, (y * TILE_SIZE) as f32)
    };

    // solid includes crumble blocks that are still there
    if overlays.blocks {
        for (y, row) in screen.solid.iter().enumerate() {
            for x in (0..row.len()).filter(|&x| row[x]) {
                hitbox(frame, at((x, y)), tile, HITBOX_BLOCK);
            }
        }
        for (y, row) in level.jumpthrus.iter().enumerate() {
            for x in (0..row.len()).filter(|&x| row[x]) {
                let size = (tile.0, JUMPTHRU_HEIGHT);
                hitbox(frame, at((x, y)), size, HITBOX_BLOCK);
            }
        }
        for mover in &level.movers {
            let (mx, my) = mover.position(screen.mover_time);
            let size = ((mover.width * TILE_SIZE) as i32, JUMPTHRU_HEIGHT);
            hitbox(frame, camera.world_to_screen(mx, my), size, HITBOX_BLOCK);
        }
    }

    // coins and the goal use the whole tile
    if overlays.entities {
        for &coin in coins.iter().chain(&level.goal) {
            hitbox(frame, at(coin), tile, HITBOX_ENTITY);
        }
    }

    if overlays.entities {
        let top = (TILE_SIZE - SPRING_HEIGHT) as i32;
        for &spring in &level.springs {
            let (x, y) = at(spring);
            hitbox(
                frame,
                (x, y + top),
                (tile.0, SPRING_HEIGHT as i32),
                HITBOX_ENTITY,
            );
        }
    }

    if overlays.entities {
        for &(spike_at, spike) in &level.spikes {
            let ((x, y), (sx, sy, w, h)) = (at(spike_at), spike.hitbox());
            let size = (w as i32, h as i32);
            hitbox(frame, (x + sx as i32, y + sy as i32), size, HITBOX_ENTITY);
        }
    }

    // there's only 1 player box right now so these are the same lol
    let player_at = camera.world_to_screen(player.x, player.y);
    let size = (HITBOX_WIDTH as i32, HITBOX_HEIGHT as i32);
    if overlays.hurtbox {
        hitbox(frame, player_at, size, HITBOX_HURTBOX);
    }
    if overlays.pickup {
        hitbox(frame, player_at, size, HITBOX_PICKUP);
    }
}

// solid outline, see-through inside
fn hitbox(
    frame: &mut RgbaImage,
    (x, y): (i32, i32),
    (w, h): (i32, i32),
    color: Color,
) {
    for py in y.max(0)..(y + h).min(frame.height() as i32) {
        for px in x.max(0)..(x + w).min(frame.width() as i32) {
            let edge = px == x || py == y || px == x + w - 1 || py == y + h - 1;
            let alpha = if edge { 1.0 } else { HITBOX_FILL_OPACITY };

            let pixel = frame.get_pixel_mut(px as u32, py as u32);
            for (channel, c) in pixel.0[..3].iter_mut().zip(color) {
                *channel =
                    (*channel as f32 * (1.0 - alpha) + c as f32 * alpha) as u8;
            }
        }
    }
}

// tiny 5x7 font
fn glyph(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        'A' => [0x0e, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        'B' => [0x1e, 0x11, 0x11, 0x1e, 0x11, 0x11, 0x1e],
        'C' => [0x0e, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0e],
        'D' => [0x1e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1e],
        'E' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x1f],
        'F' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x10],
        'G' => [0x0e, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0f],
        'H' => [0x11, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        'I' => [0x0e, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0e],
        'J' => [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0c],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1f],
        'M' => [0x11, 0x1b, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11],
        'O' => [0x0e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        'P' => [0x1e, 0x11, 0x11, 0x1e, 0x10, 0x10, 0x10],
        'Q' => [0x0e, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0d],
        'R' => [0x1e, 0x11, 0x11, 0x1e, 0x14, 0x12, 0x11],
        'S' => [0x0f, 0x10, 0x10, 0x0e, 0x01, 0x01, 0x1e],
        'T' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0a, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0a],
        'X' => [0x11, 0x11, 0x0a, 0x04, 0x0a, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0a, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1f],
        '0' => [0x0e, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0e],
        '1' => [0x04, 0x0c, 0x04, 0x04, 0x04, 0x04, 0x0e],
        '2' => [0x0e, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1f],
        '3' => [0x1f, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0e],
        '4' => [0x02, 0x06, 0x0a, 0x12, 0x1f, 0x02, 0x02],
        '5' => [0x1f, 0x10, 0x1e, 0x01, 0x01, 0x11, 0x0e],
        '6' => [0x06, 0x08, 0x10, 0x1e, 0x11, 0x11, 0x0e],
        '7' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0e, 0x11, 0x11, 0x0e, 0x11, 0x11, 0x0e],
        '9' => [0x0e, 0x11, 0x11, 0x0f, 0x01, 0x02, 0x0c],
        ':' => [0x00, 0x0c, 0x0c, 0x00, 0x0c, 0x0c, 0x00],
        '-' => [0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x0c],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x0c, 0x04, 0x08],
        '(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        '>' => [0x10, 0x08, 0x04, 0x02, 0x04, 0x08, 0x10],
        '<' => [0x01, 0x02, 0x04, 0x08, 0x04, 0x02, 0x01],
        _ => [0; 7],
    }
}

/// Draw text centered horizontally
pub fn text(
    frame: &mut RgbaImage,
    text: &str,
    y: i32,
    scale: i32,
    color: Color,
) {
    let width = (text.chars().count() as i32 * 6 - 1) * scale;
    let x = (frame.width() as i32 - width) / 2;
    for (i, c) in text.chars().enumerate() {
        for (row, bits) in glyph(c).iter().enumerate() {
            for col in (0..5).filter(|col| bits >> (4 - col) & 1 == 1) {
                let px = x + (i as i32 * 6 + col) * scale;
                let py = y + row as i32 * scale;
                for (dx, dy) in
                    (0..scale).flat_map(|dx| (0..scale).map(move |dy| (dx, dy)))
                {
                    let (px, py) = ((px + dx) as u32, (py + dy) as u32);
                    if let Some(p) = frame.get_pixel_mut_checked(px, py) {
                        p.0 = [color[0], color[1], color[2], 255];
                    }
                }
            }
        }
    }
}

// used behind the debug menu so the text is readable
pub fn darken(frame: &mut RgbaImage, (x, y): (u32, u32), (w, h): (u32, u32)) {
    for (px, py, pixel) in frame.enumerate_pixels_mut() {
        if (x..x + w).contains(&px) && (y..y + h).contains(&py) {
            pixel.0[..3].iter_mut().for_each(|c| *c /= 4);
        }
    }
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

    // make the shadow based on the final sprite
    let mut shadow = tile.clone();
    for pixel in shadow.pixels_mut() {
        pixel.0[..3].fill(0);
        pixel.0[3] = (pixel.0[3] as u16 * SHADOW_OPACITY as u16 / 255) as u8;
    }

    imageops::overlay(
        frame,
        &shadow,
        (destination.0 + SHADOW_OFFSET.0).into(),
        (destination.1 + SHADOW_OFFSET.1).into(),
    );
    imageops::overlay(frame, &tile, destination.0.into(), destination.1.into());
}
