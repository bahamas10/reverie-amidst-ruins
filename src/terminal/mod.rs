/*!
 * Functions to use the kitty graphics protocol but simplified for just the
 * subset of stuff I need.
 *
 * Based on https://sw.kovidgoyal.net/kitty/graphics-protocol/
 */

use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{
    self, Event, KeyEvent, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::{cursor, execute, queue, terminal};

pub mod kitty_graphics;

#[derive(Clone, Copy)]
pub struct ImagePlacement {
    pub column: u16,
    pub row: u16,
    pub width: u16,
    pub height: u16,
}

/**
 * Abstraction over the terminal so we can hide the cursor and enter the
 * alternate screen and correctly (to the best of our ability) handle cleanup by
 * implementing `Drop`
 */
pub struct Terminal {
    active: bool,
    keyboard_flags: bool,
    size: terminal::WindowSize,

    // optional requested width by the user (used for scaling)
    image_width: Option<u16>,
}

impl Terminal {
    pub fn init() -> Result<Self> {
        let size =
            terminal::window_size().context("could not read terminal size")?;

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

        Ok(Self { active: true, keyboard_flags, size, image_width: None })
    }

    /// Adjust the displayed image width by a number of columns
    pub fn adjust_image_width(
        &mut self,
        amount: i16,
        image_width: usize,
        image_height: usize,
    ) {
        let max_width = self
            .fitted_size(image_width, image_height, self.size.columns.max(1))
            .0;
        let current_width =
            self.image_width.unwrap_or(max_width).min(max_width);
        let new_width =
            (current_width as i32 + amount as i32).clamp(1, max_width as i32);

        self.image_width = Some(new_width as u16);
        self.clear_screen().unwrap();
    }

    /// Figure out where to place the image at its current display width.
    pub fn image_placement(
        &self,
        image_width: usize,
        image_height: usize,
    ) -> ImagePlacement {
        let columns = self.size.columns.max(1);
        let rows = self.size.rows.max(1);
        let max_width = self.fitted_size(image_width, image_height, columns).0;
        let requested_width =
            self.image_width.unwrap_or(max_width).min(max_width);
        let (width, height) =
            self.fitted_size(image_width, image_height, requested_width);

        // center the image always
        ImagePlacement {
            column: (columns - width) / 2,
            row: (rows - height) / 2,
            width,
            height,
        }
    }

    fn fitted_size(
        &self,
        image_width: usize,
        image_height: usize,
        requested_width: u16,
    ) -> (u16, u16) {
        let columns = self.size.columns.max(1);
        let rows = self.size.rows.max(1);

        let cell_width = self.size.width.max(1) as f32 / columns as f32;
        let cell_height = self.size.height.max(1) as f32 / rows as f32;

        let mut width = requested_width.min(columns).max(1);
        let mut height = (width as f32 * image_height as f32 * cell_width
            / (image_width as f32 * cell_height))
            .ceil() as u16;

        if height > rows {
            height = rows;
            width = (height as f32 * image_width as f32 * cell_height
                / (image_height as f32 * cell_width))
                .floor() as u16;
        }

        // we could probably enforce something sane here tbh
        width = width.max(1);
        height = height.max(1);

        (width, height)
    }

    /// Read terminal events, react to some, and return all key events
    pub fn read_events(&mut self) -> Result<Vec<KeyEvent>> {
        let mut events = vec![];
        loop {
            let poll = event::poll(Duration::ZERO)
                .context("could not poll terminal input")?;

            if !poll {
                break;
            }

            let ev = event::read().context("could not read terminal input")?;
            match ev {
                Event::Key(key) => events.push(key),
                Event::Resize(_, _) => {
                    self.size = terminal::window_size()
                        .context("could not read terminal size")?;
                    self.image_width = None;
                    self.clear_screen()?;
                }
                _ => {}
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
        kitty_graphics::delete_image(&mut stdout)?;

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

    fn clear_screen(&self) -> Result<()> {
        let mut stdout = std::io::stdout();

        queue!(stdout, terminal::Clear(terminal::ClearType::All))
            .context("failed to clear screen")?;

        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.deinit();
    }
}
