/*!
 * Functions to use the kitty graphics protocol but simplified for just the
 * subset of stuff I need.
 *
 * Based on https://sw.kovidgoyal.net/kitty/graphics-protocol/
 */

use std::io::Write;

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use crossterm::{cursor, execute, terminal};
use flate2::{Compression, write::ZlibEncoder};

const APC_START: &[u8] = b"\x1b_G";
const APC_END: &[u8] = b"\x1b\\";
const MAX_PAYLOAD_SIZE: usize = 4096;
const IMAGE_ID: u32 = 1;
const PLACEMENT_ID: u32 = 1;

/**
 * Abstraction over the terminal so we can hide the cursor and enter the
 * alternate screen and correctly (to the best of our ability) handle cleanup by
 * implementing `Drop`
 */
pub struct Terminal {
    active: bool,
}

impl Terminal {
    pub fn init() -> Result<Self> {
        execute!(
            std::io::stdout(),
            terminal::EnterAlternateScreen,
            terminal::Clear(terminal::ClearType::All),
            cursor::Hide
        )
        .context("could not init terminal")?;
        Ok(Self { active: true })
    }

    pub fn deinit(&mut self) -> Result<()> {
        if !self.active {
            return Ok(());
        }

        let mut stdout = std::io::stdout();

        // delete the kitty image
        write!(stdout, "\x1b_Ga=d,d=I,i={IMAGE_ID},q=2;\x1b\\")?;

        execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)
            .context("could not deinit the terminal")?;

        self.active = false;

        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.deinit();
    }
}
