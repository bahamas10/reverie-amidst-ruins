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
mod input;
mod level;
mod render;
mod terminal;

use terminal::Terminal;

const LEVEL1: &str = include_str!("../levels/level1.txt");

const WIDTH: usize = 480;
const HEIGHT: usize = 270;

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

        // render the frame
        let frame = render::level(&level, elapsed)
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
        println!("fps = {:.1}", fps);

        // naive: sleep to ensure the target fps
        let elapsed = now.elapsed();
        if elapsed < frame_duration {
            thread::sleep(frame_duration - elapsed);
        }
    }

    Ok(())
}
