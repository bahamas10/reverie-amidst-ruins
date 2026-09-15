/*!
 * Functions to use the kitty graphics protocol but simplified for just the
 * subset of stuff I need.
 *
 * Based on https://sw.kovidgoyal.net/kitty/graphics-protocol/
 */

use std::io::Write;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{
    self, Event, KeyEvent, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::{cursor, execute, terminal};

pub mod kitty_graphics;

/**
 * Abstraction over the terminal so we can hide the cursor and enter the
 * alternate screen and correctly (to the best of our ability) handle cleanup by
 * implementing `Drop`
 */
pub struct Terminal {
    active: bool,
    keyboard_flags: bool,
}

impl Terminal {
    pub fn init() -> Result<Self> {
        terminal::enable_raw_mode().context("could not enable raw mode")?;

        let mut stdout = std::io::stdout();
        execute!(
            stdout,
            terminal::EnterAlternateScreen,
            terminal::Clear(terminal::ClearType::All),
            cursor::Hide
        )
        .context("could not init terminal")?;

        // enable terminal for geting keyboard events
        let keyboard_flags = execute!(
            stdout,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            )
        )
        .is_ok();

        Ok(Self { active: true, keyboard_flags })
    }

    pub fn read_events(&mut self) -> Result<Vec<KeyEvent>> {
        let mut events = vec![];
        loop {
            let poll = event::poll(Duration::ZERO)
                .context("could not poll terminal input")?;

            if !poll {
                break;
            }

            let ev = event::read().context("could not read terminal input")?;
            if let Event::Key(key) = ev {
                events.push(key);
            }
        }

        Ok(events)
    }

    pub fn deinit(&mut self) -> Result<()> {
        if !self.active {
            return Ok(());
        }

        let mut stdout = std::io::stdout();

        // delete the kitty image
        write!(
            stdout,
            "\x1b_Ga=d,d=I,i={},q=2;\x1b\\",
            kitty_graphics::IMAGE_ID
        )?;

        // restore the keyboard events if they were set
        if self.keyboard_flags {
            execute!(stdout, PopKeyboardEnhancementFlags)?;
        }

        execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)
            .context("could not deinit the terminal")?;
        terminal::disable_raw_mode().context("could not disable raw mode")?;

        self.active = false;

        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.deinit();
    }
}
