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

use std::time::{Duration, Instant};

use anyhow::{Context, Result};

mod args;
mod camera;
mod input;
mod level;
mod player;
mod render;
mod screen;
mod terminal;

use screen::{LevelScreen, Screen};
use terminal::Terminal;

const LEVEL1: &str = include_str!("../levels/level1.txt");

// internal width and height of the game viewport (rendered internall and then
// optionally stretched)
const WIDTH: usize = 360;
const HEIGHT: usize = 270;

// number of pixels below the floor of the level where a death is considered
const DEATH_PLANE: f32 = 70.0;

fn main() -> Result<()> {
    // parse args
    let args = args::parse();

    // load the first level behind the title screen
    let level_screen =
        LevelScreen::load(LEVEL1).context("could not load level1")?;

    // hold this handle so it'll go out of scope and de-init when done
    let mut term_handle = Terminal::init()?;
    let mut input = input::Input::default();
    let mut screen = Screen::Title(level_screen);

    // target FPS
    let target_fps = 60.0;
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps);
    let mut next_frame = Instant::now();
    let mut fps_timer = Instant::now();
    let mut fps_frame_count = 0;
    let mut fps = 0.0;

    // game loop
    let mut last_time = Instant::now();
    while !input.quit {
        let now = Instant::now();
        let delta = now.duration_since(last_time);
        last_time = now;

        // read all pending terminal input
        let events = term_handle.read_events()?;
        for event in events {
            input.handle_key(event);
        }

        // remove the title and spawn the player into the active level
        if input.confirm_pressed {
            screen = match screen {
                Screen::Title(level_screen) => Screen::Level(level_screen),
                screen => screen,
            };
        }

        // update the active screen
        match &mut screen {
            Screen::Title(level_screen) => {
                level_screen.elapsed += delta.as_secs_f32();
            }
            Screen::Level(level_screen) => {
                level_screen.elapsed += delta.as_secs_f32();

                let level = &level_screen.level;
                let player = &mut level_screen.player;

                let level_width = (level.width * level::TILE_SIZE) as f32;
                player.update(
                    &input,
                    delta.as_secs_f32(),
                    &level.blocks,
                    level_width - player::HITBOX_WIDTH,
                );

                // death: reset after falling below the level
                let level_height = (level.height * level::TILE_SIZE) as f32;
                if player.y > level_height + DEATH_PLANE {
                    player.reset();
                    level_screen.remaining_coins = level.coins.clone();
                    level_screen.coin_count = 0;
                } else {
                    // check if we touched any coins
                    level_screen.remaining_coins.retain(|&(x, y)| {
                        let collected = player.overlaps(
                            (x * level::TILE_SIZE) as f32,
                            (y * level::TILE_SIZE) as f32,
                            level::TILE_SIZE as f32,
                            level::TILE_SIZE as f32,
                        );
                        if collected {
                            level_screen.coin_count += 1;
                        }
                        !collected
                    });
                }

                level_screen.camera.follow(player, level_width, WIDTH as f32);
            }
        }
        input.jump_pressed = false;
        input.confirm_pressed = false;

        // render the frame
        let frame = match &mut screen {
            Screen::Title(level_screen) => {
                let mut frame = render::level(
                    &level_screen.level,
                    None,
                    &level_screen.camera,
                    &level_screen.remaining_coins,
                    level_screen.elapsed,
                    WIDTH as u32,
                    HEIGHT as u32,
                )
                .context("could not render title level")?;
                render::title(&mut frame)?;
                frame
            }
            Screen::Level(level_screen) => render::level(
                &level_screen.level,
                Some(&level_screen.player),
                &level_screen.camera,
                &level_screen.remaining_coins,
                level_screen.elapsed,
                WIDTH as u32,
                HEIGHT as u32,
            )
            .context("could not render level")?,
        };

        // display the frame
        let placement = term_handle.image_placement(
            frame.width() as usize,
            frame.height() as usize,
            args.width,
        );
        terminal::kitty_graphics::display_rgba(
            frame.as_raw(),
            frame.width() as usize,
            frame.height() as usize,
            placement,
        )?;

        // display FPS
        fps_frame_count += 1;
        if fps_timer.elapsed() >= Duration::from_secs(1) {
            // calculate the FPS every second
            fps = fps_frame_count as f64 / fps_timer.elapsed().as_secs_f64();
            fps_frame_count = 0;
            fps_timer = Instant::now();
        }
        if let Screen::Level(level_screen) = &screen {
            print!(
                "fps = {:.1} | coins = {}    \r",
                fps, level_screen.coin_count
            );
        }

        // sleep until the next fixed deadline - keeping an absolute deadline
        // prevents small sleeps from going oven the target
        next_frame += frame_duration;
        if Instant::now() < next_frame {
            spin_sleep::sleep_until(next_frame);
        } else {
            // rendering took longer than a frame uh oh - should we warn?
            next_frame = Instant::now();
        }
    }

    Ok(())
}
