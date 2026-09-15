/*!
 * A simple 2D platformer game to be played directly in the terminal.
 *
 * # Created
 * Author: Dave Eddy <ysap@daveeddy.com>
 * Date: September 13, 2026
 * License: MIT
 *
 * # Contributors
 * - Dave Eddy <ysap@daveeddy.com>
 */

use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

mod args;
mod camera;
mod input;
mod level;
mod player;
mod render;
mod terminal;

use camera::Camera;
use player::Player;
use terminal::Terminal;

const LEVEL1: &str = include_str!("../levels/level1.txt");

const WIDTH: usize = 480;
const HEIGHT: usize = 270;
const DEATH_PLANE: f32 = 70.0;

fn main() -> Result<()> {
    // parse args
    let args = args::parse();

    // get terminal width (default to full size)
    let display_width = match args.width {
        Some(width) => width,
        None => {
            crossterm::terminal::size()
                .context("could not read terminal size")?
                .0
        }
    };

    // parse the level
    let level = level::LevelDefinition::parse(LEVEL1)
        .context("could not parse level1")?;

    // hold this handle so it'll go out of scope and de-init when done
    let mut term_handle = Terminal::init()?;
    let mut input = input::Input::default();
    let mut player = Player::new(
        (level.spawn.0 * level::TILE_SIZE) as f32,
        (level.spawn.1 * level::TILE_SIZE) as f32,
    );
    let mut camera = Camera::default();
    let mut remaining_coins = level.coins.clone();
    let mut coin_count = 0;

    // target FPS
    let target_fps = 60.0;
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps);
    let mut fps_timer = Instant::now();
    let mut fps_frame_count = 0;
    let mut fps = 0.0;

    // game loop
    let mut last_time = Instant::now();
    let mut elapsed = 0.0;
    while !input.quit {
        let now = Instant::now();
        let delta = now.duration_since(last_time);
        last_time = now;
        elapsed += delta.as_secs_f32();

        // read all pending terminal input
        let events = term_handle.read_events()?;
        for event in events {
            input.handle_key(event);
        }

        // update the player
        let level_width = (level.width * level::TILE_SIZE) as f32;
        player.update(
            &input,
            delta.as_secs_f32(),
            &level.blocks,
            level_width - level::TILE_SIZE as f32,
        );
        input.jump_pressed = false;

        // death: reset after falling below the level
        let level_height = (level.height * level::TILE_SIZE) as f32;
        if player.y > level_height + DEATH_PLANE {
            // player died! reset the player and the level
            player.reset();
            remaining_coins = level.coins.clone();
            coin_count = 0;
        } else {
            // player still alive

            // check if we touched any coins
            remaining_coins.retain(|&(x, y)| {
                let collected = player.overlaps(
                    (x * level::TILE_SIZE) as f32,
                    (y * level::TILE_SIZE) as f32,
                    level::TILE_SIZE as f32,
                    level::TILE_SIZE as f32,
                );
                if collected {
                    coin_count += 1;
                }
                !collected
            });
        }

        // update the camera
        camera.follow(&player, level_width, WIDTH as f32);

        // render the frame
        let frame =
            render::level(&level, &player, &camera, &remaining_coins, elapsed)
                .context("could not render level1")?;

        // display the frame
        terminal::kitty_graphics::display_rgba(
            frame.as_raw(),
            WIDTH,
            HEIGHT,
            display_width,
        )?;

        // display FPS
        fps_frame_count += 1;
        if fps_timer.elapsed() >= Duration::from_secs(1) {
            // calculate the FPS every second
            fps = fps_frame_count as f64 / fps_timer.elapsed().as_secs_f64();
            fps_frame_count = 0;
            fps_timer = Instant::now();
        }
        println!("fps = {:.1} | coins = {}    \r", fps, coin_count);

        // naive: sleep to ensure the target fps
        let elapsed = now.elapsed();
        if elapsed < frame_duration {
            thread::sleep(frame_duration - elapsed);
        }
    }

    Ok(())
}
