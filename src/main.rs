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
mod audio;
mod camera;
mod color;
mod input;
mod level;
mod player;
mod render;
mod screen;
mod stats;
mod terminal;

use audio::SoundEffect;
use screen::{LevelScreen, Screen};
use stats::Stats;
use terminal::Terminal;

const LEVELS: &[&str] = &[
    include_str!("../levels/level1.txt"),
    include_str!("../levels/level2.txt"),
    include_str!("../levels/level3.txt"),
];

// internal width and height of the game viewport (rendered internally and then
// stretched)
const WIDTH: usize = 360;
const HEIGHT: usize = 270;

// number of pixels below the floor of the level where a death is considered
const DEATH_PLANE: f32 = 70.0;

// how long a message should stay on screen
const MESSAGE_DURATION: Duration = Duration::from_secs(3);

fn load_level(index: usize) -> Result<LevelScreen> {
    let source = LEVELS.get(index).context("unknown level")?;

    let mut level_screen = LevelScreen::load(index, source)
        .with_context(|| format!("could not load level {}", index + 1))?;

    // camera starts at spawn (then follows player)
    let level_width = (level_screen.level.width * level::TILE_SIZE) as f32;
    let level_height = (level_screen.level.height * level::TILE_SIZE) as f32;
    level_screen.camera.snap_to(
        &level_screen.player,
        level_width,
        level_height,
        WIDTH as f32,
        HEIGHT as f32,
    );

    Ok(level_screen)
}

fn main() -> Result<()> {
    // parse args
    let args = args::parse();
    let start_level = args.level as usize - 1;

    // load the first level behind the title screen
    let level_screen = load_level(start_level)?;
    let mut stats = Stats::new(start_level, LEVELS)?;

    // start the title music unless audio was disabled
    let mut audio = if args.no_audio {
        None
    } else {
        Some(
            audio::Audio::new("reverie-amidst-ruins", args.volume)
                .context("could not initialize audio")?,
        )
    };

    // print the start message
    args::print_help();

    // hold this handle so it'll go out of scope and de-init when done
    let mut term_handle = Terminal::init()?;
    let mut input = input::Input::new();
    let mut screen = Screen::Title(level_screen);

    // target FPS
    let frame_duration = Duration::from_secs_f32(1.0 / args.fps as f32);
    let mut next_frame = Instant::now();

    // "HUD"
    let mut message = None;
    let mut effects_enabled = true;
    let mut debug_enabled = args.debug;
    let mut fps_timer = Instant::now();
    let mut fps_frame_count = 0;
    let mut fps = 0.0;
    let mut bytes_this_second = 0;
    let mut kib_per_second = 0.0;

    // game loop
    let mut last_time = Instant::now();
    let mut finished = false;
    while !input.quit {
        let now = Instant::now();
        let delta = now.duration_since(last_time);
        last_time = now;

        // read all pending terminal input
        let events = term_handle.read_events()?;
        input.update(&events);

        if input.effects_toggle_pressed {
            effects_enabled = !effects_enabled;
            let state = if effects_enabled { "on" } else { "off" };
            message = Some((
                format!("effects {state}"),
                Instant::now() + MESSAGE_DURATION,
            ));
        }
        if input.palette_randomize_pressed {
            let palette = color::random_palette();
            match &mut screen {
                Screen::Title(s) => s.level.palette = palette,
                Screen::Level(s) => s.level.palette = palette,
                Screen::End(s) => s.level.palette = palette,
            };
            message = Some((
                "palette randomized".into(),
                Instant::now() + MESSAGE_DURATION,
            ));
        }
        if input.debug_toggle_pressed {
            debug_enabled = !debug_enabled;
        }
        if input.mute_toggle_pressed {
            let state = if let Some(audio) = &mut audio {
                if audio.toggle_mute() { "muted" } else { "unmuted" }
            } else {
                "disabled"
            };
            message = Some((
                format!("audio {state}"),
                Instant::now() + MESSAGE_DURATION,
            ));
        }
        if input.volume_down_pressed || input.volume_up_pressed {
            let num = if input.volume_down_pressed { -5 } else { 5 };

            let status = match &mut audio {
                Some(audio) => format!("volume {}%", audio.adjust_volume(num)),
                None => "audio disabled".to_string(),
            };

            message = Some((status, Instant::now() + MESSAGE_DURATION));
        }
        if input.scale_down_pressed || input.scale_up_pressed {
            let num = if input.scale_down_pressed { -1 } else { 1 };
            term_handle.adjust_image_width(num, WIDTH, HEIGHT);
        }
        let restarted = if input.restart_pressed {
            if let Screen::Level(level_screen) = &mut screen {
                level_screen.player.reset();
                level_screen.remaining_coins = level_screen.level.coins.clone();
                level_screen.coin_count = 0;
                stats.deaths += 1;
                true
            } else {
                false
            }
        } else {
            false
        };
        // remove the title and spawn the player into the active level
        if input.confirm_pressed {
            screen = match screen {
                Screen::Title(level_screen) => {
                    if let Some(audio) = &audio {
                        audio.play_music(&level_screen.level.music)?;
                    }
                    Screen::Level(level_screen)
                }
                screen => screen,
            };
        }

        // update the active screen
        let mut next_level = None;
        let mut jumped = false;
        let mut collected_coin = false;
        let mut died = restarted;
        match &mut screen {
            Screen::Title(level_screen) | Screen::End(level_screen) => {
                level_screen.elapsed += delta.as_secs_f32();
            }
            Screen::Level(level_screen) => {
                stats.playtime += delta;
                level_screen.elapsed += delta.as_secs_f32();

                let level = &level_screen.level;
                let player = &mut level_screen.player;

                let level_width = (level.width * level::TILE_SIZE) as f32;

                let player_update = player.update(
                    &input,
                    delta.as_secs_f32(),
                    &level.blocks,
                    level_width - player::HITBOX_WIDTH,
                );
                jumped = player_update.jumped;

                // death: reset after falling below the level
                let level_height = (level.height * level::TILE_SIZE) as f32;
                if player.y > level_height + DEATH_PLANE {
                    player.reset();
                    level_screen.remaining_coins = level.coins.clone();
                    level_screen.coin_count = 0;
                    stats.deaths += 1;
                    died = true;
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
                            collected_coin = true;
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
                    if goal {
                        stats.levels_beaten += 1;
                        stats.coins_collected += level_screen.coin_count;

                        if level_screen.index + 1 < LEVELS.len() {
                            next_level = Some(level_screen.index + 1);
                        } else {
                            finished = true;
                        }
                    }
                }

                level_screen.camera.follow(
                    player,
                    level_width,
                    level_height,
                    WIDTH as f32,
                    HEIGHT as f32,
                    delta.as_secs_f32(),
                );
            }
        }

        if let Some(index) = next_level {
            let level_screen = load_level(index)?;
            if let Some(audio) = &audio {
                audio.play_music(&level_screen.level.music)?;
            }
            screen = Screen::Level(level_screen);
        } else if finished {
            screen = match screen {
                Screen::Level(level_screen) => Screen::End(level_screen),
                screen => screen,
            };
        }

        if let Some(audio) = &audio {
            if jumped {
                audio.play_effect(SoundEffect::Jump)?;
            }
            if collected_coin {
                audio.play_effect(SoundEffect::Coin)?;
            }
            if died {
                audio.play_effect(SoundEffect::Dead)?;
            }
        }
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
            Screen::End(level_screen) => {
                let mut frame = render::level(
                    &level_screen.level,
                    None,
                    &level_screen.camera,
                    &level_screen.remaining_coins,
                    level_screen.elapsed,
                    WIDTH as u32,
                    HEIGHT as u32,
                )
                .context("could not render end level")?;
                render::end(&mut frame)?;
                frame
            }
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
            let filename = format!("rar-screenshot-{timestamp}.png");

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

        // display the frame
        let placement = term_handle
            .image_placement(frame.width() as usize, frame.height() as usize);
        let frame_bytes = terminal::kitty_graphics::display_rgba(
            frame.as_raw(),
            frame.width() as usize,
            frame.height() as usize,
            placement,
        )?;
        let frame_kib = frame_bytes as f64 / 1024.0;
        bytes_this_second += frame_bytes;

        // format "HUD" - by which i mean the line of text we `print!` lmao

        // fps
        fps_frame_count += 1;
        if fps_timer.elapsed() >= Duration::from_secs(1) {
            // calculate the FPS every second
            let elapsed = fps_timer.elapsed().as_secs_f64();
            fps = fps_frame_count as f64 / elapsed;
            kib_per_second = bytes_this_second as f64 / 1024.0 / elapsed;

            fps_frame_count = 0;
            bytes_this_second = 0;

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
            Screen::End(_) => {
                stats.print_oneline();
                if !msg.is_empty() {
                    print!(" | {}", msg);
                }
                print!("\x1b[K\r");
            }
            Screen::Level(level_screen) => {
                print!("fps {:.1}", fps);

                if debug_enabled {
                    print!(
                        " | {:.1} KiB/frame | {:.1} KiB/s",
                        frame_kib, kib_per_second
                    );
                }

                print!(
                    " | coins {} | deaths {}",
                    level_screen.coin_count, stats.deaths
                );

                if !msg.is_empty() {
                    print!(" | {}", msg);
                }

                print!("\x1b[K\r");
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

    term_handle.deinit()?;

    if finished {
        println!("\n\nCongratulations - You Win!\n");
        stats.print_multiline();
        println!();
    }

    Ok(())
}
