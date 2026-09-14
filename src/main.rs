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
mod kitty_graphics;
mod level;

const IMAGE: &[u8] = include_bytes!("../assets/backgrounds/snow.png");
const LEVEL1: &str = include_str!("../levels/level1.txt");

fn main() -> Result<()> {
    let _args = args::parse();
    let level = level::LevelDefinition::parse(LEVEL1)
        .context("invalid bundled level1.txt")?;

    println!("loaded level:\n{:#?}", level);

    let image =
        image::load_from_memory(IMAGE).expect("invalid image").into_rgba8();

    let target_fps = 60.0;
    let mut fps_timer = Instant::now();
    let mut fps_frame_count = 0;

    let mut last_time = Instant::now();
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps);

    // game loop
    loop {
        let now = Instant::now();
        let _delta = now.duration_since(last_time);
        last_time = now;

        // display the image
        kitty_graphics::display_rgba(&image, 480, 270)?;
        println!("\n");

        // calculate FPS
        fps_frame_count += 1;
        if fps_timer.elapsed() >= Duration::from_secs(1) {
            let fps =
                fps_frame_count as f64 / fps_timer.elapsed().as_secs_f64();
            fps_frame_count = 0;
            fps_timer = Instant::now();
            println!("fps = {:.1}", fps);
        }

        // sleep if we finished faster than our target FPS
        let elapsed = now.elapsed();
        if elapsed < frame_duration {
            thread::sleep(frame_duration - elapsed);
        }

        // XXX: don't flood the screen lol
        thread::sleep(Duration::from_secs(1));
    }

    unreachable!()
}
