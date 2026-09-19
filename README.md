Reverie Amidst Ruins
====================

A simple 2D platformer game to be played directly in the terminal.

How To Play
-----------

Make sure you have [Rust](https://rust-lang.org) installed to compile the source
code, then clone this repo and run it with:

    git clone https://github.com/bahamas10/reverie-amidst-ruins.git
    cd reverie-amidst-ruins
    cargo run --release

Controls
--------

Movement
- `wasd` / `arrows` -  Move player
- `space bar` - Jump
- `r` - Restart level (counts as a death)

Utility
- `num 1` - Toggle post-processing effects
- `num 2` - Save screenshot to current directory
- `num 3` - Randomize level color palette
- `num 8` - Lower master volume by 5%
- `num 9` - Raise master volume by 5%
- `m` - Mute audio
- `-` / `_` - Shrink image by one column
- `+` / `=` - Grow image by one column

How Does This Work?
-------------------

The concept is surprisingly simple - the game logic itself is handled in Rust to
update the player, handle collision, coin collecting, etc.  The "frames" are
then rendered onto an RGBA image completely in memory.  Then, the frame is
compressed is zlib compressed and then written to the terminal via the [Kitty
Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/).

- [args.rs](args.rs) - CLI arguments
- [audio.rs](audio.rs) - Audio Engine (music and SFX)
- [camera.rs](camera.rs) - Player-following camera
- [color.rs](color.rs) - Helpers to colorize tiles with custom palettes
- [input.rs](input.rs) - Handle and abstract user input
- [level.rs](level.rs) - Level processing and parsing (levels in [levels/](levels))
- [main.rs](main.rs) - Main entry point and game loop
- [player.rs](player.rs) - The player and its properties
- [render.rs](render.rs) - Render the game frame to an RGBA image
- [screen.rs](screen.rs) - Abstractions over "screens" (intro, level, end, etc.)
- [stats.rs](stats.rs) - Maintain stats during the game (deaths, playtime, etc.)
  - [terminal/kitty_graphics.rs](terminal/kitty_graphics.rs) - Helpers to output
      images to the terminal
  - [terminal/mod.rs](terminal/mod.rs) - Abstraction over the terminal to handle
      ANSI

Performance
-----------

On my M1 Macbook Pro:

- without `--release`: ~25 FPS
- with `--release`: comfortably runs at 144fps, can go up to ~500

You can test the FPS yourself with:

    cargo run --release -- --fps 30
    cargo run --release -- --fps 60
    cargo run --release -- --fps 144

License
-------

MIT License
