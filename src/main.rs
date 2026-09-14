/**
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
use anyhow::Result;

mod args;
mod kitty_graphics;

const IMAGE: &[u8] = include_bytes!("../assets/backgrounds/snow.png");

fn main() -> Result<()> {
    let _args = args::parse();

    let image =
        image::load_from_memory(IMAGE).expect("invalid image").into_rgba8();

    kitty_graphics::display_rgba(&image, 480, 270)?;
    println!("\n");

    Ok(())
}
