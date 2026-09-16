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

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

const LEVELS: &[&str] = &[
    include_str!("../levels/level1.txt"),
    include_str!("../levels/level2.txt"),
    include_str!("../levels/level3.txt"),
];

// internal width and height of the game viewport (rendered internall and then
// optionally stretched)
const WIDTH: usize = 360;
const HEIGHT: usize = 270;

// number of pixels below the floor of the level where a death is considered
const DEATH_PLANE: f32 = 70.0;

// how long a message should stay on screen
const MESSAGE_DURATION: Duration = Duration::from_secs(3);

fn load_level(index: usize) -> Result<LevelScreen> {
    let source = LEVELS.get(index).context("unknown level")?;

    LevelScreen::load(index, source)
        .with_context(|| format!("could not load level {}", index + 1))
}

fn main() -> Result<()> {
    // parse args
    let args = args::parse();

    // print the start message
    print!(include_str!("../assets/start.txt"));

    // load the first level behind the title screen
    let level_screen = load_level(2)?;

    // hold this handle so it'll go out of scope and de-init when done
    let mut term_handle = Terminal::init()?;
    let mut input = input::Input::default();
    let mut screen = Screen::Title(level_screen);

    // target FPS
    let target_fps = 60.0;
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps);
    let mut next_frame = Instant::now();

    // "HUD"
    let mut message = None;
    let mut effects_enabled = true;
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

        if input.effects_toggle_pressed {
            effects_enabled = !effects_enabled;
            let state = if effects_enabled { "on" } else { "off" };
            message = Some((
                format!("effects {state}"),
                Instant::now() + MESSAGE_DURATION,
            ));
        }
        input.effects_toggle_pressed = false;

        // remove the title and spawn the player into the active level
        if input.confirm_pressed {
            screen = match screen {
                Screen::Title(level_screen) => Screen::Level(level_screen),
                screen => screen,
            };
        }

        // update the active screen
        let mut next_level = None;
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

                    // touching the goal goes to the next level (if there is
                    // one)
                    let goal = player.overlaps(
                        (level.goal.0 * level::TILE_SIZE) as f32,
                        (level.goal.1 * level::TILE_SIZE) as f32,
                        level::TILE_SIZE as f32,
                        level::TILE_SIZE as f32,
                    );
                    if goal && level_screen.index + 1 < LEVELS.len() {
                        next_level = Some(level_screen.index + 1);
                    }
                }

                level_screen.camera.follow(player, level_width, WIDTH as f32);
            }
        }
        if let Some(index) = next_level {
            screen = Screen::Level(load_level(index)?);
        }
        input.jump_pressed = false;
        input.confirm_pressed = false;

        // render the frame
        let mut frame = match &mut screen {
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

        // apply full-screen effects to the final composed image
        if effects_enabled {
            render::post_process(&mut frame);
        }

        // save a screenshot (unscaled, directly write frame)
        if input.screenshot_pressed {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("system clock is bad")?
                .as_millis();
            let filename = format!("ram-screenshot-{timestamp}.png");

            // we could maybe check if the file exists before saving it? i think
            // it's fine to not bother since the name is unique enough?
            frame
                .save(&filename)
                .with_context(|| format!("could not save {filename}"))?;

            message = Some((
                format!("saved {filename}"),
                Instant::now() + MESSAGE_DURATION,
            ));
        }
        input.screenshot_pressed = false;

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

        // format "HUD" - by which i mean the line of text we `print!` lmao

        // fps
        fps_frame_count += 1;
        if fps_timer.elapsed() >= Duration::from_secs(1) {
            // calculate the FPS every second
            fps = fps_frame_count as f64 / fps_timer.elapsed().as_secs_f64();
            fps_frame_count = 0;
            fps_timer = Instant::now();
        }

        // optional message
        if message.as_ref().is_some_and(|(_, until)| Instant::now() >= *until) {
            message = None;
        }
        let msg = message.as_ref().map(|(text, _)| text.as_str()).unwrap_or("");

        match &screen {
            Screen::Title(_) => {
                if !msg.is_empty() {
                    print!("{}\x1b[K\r", msg);
                }
            }
            Screen::Level(level_screen) => {
                print!(
                    "fps = {:.1} | coins = {} | {}\x1b[K\r",
                    fps, level_screen.coin_count, msg
                );
            }
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
