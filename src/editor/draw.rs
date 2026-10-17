/*!
 * Drawing the editor on top of the world
 */

use anyhow::Result;
use image::RgbaImage;

use super::{BOTTOM, BRUSHES, Editor, Entry, TOP};
use crate::level::TILE_SIZE;
use crate::render;
use crate::{HEIGHT, WIDTH};

const WHITE: [u8; 3] = [0xff; 3];
const GRAY: [u8; 3] = [0x80; 3];
const RED: [u8; 3] = [0xff, 0x50, 0x50];

impl Editor {
    pub fn render(&self) -> Result<RgbaImage> {
        let (width, height) = (WIDTH as u32, HEIGHT as u32);
        let mut frame = render::level(&self.world, None, width, height)?;
        if self.open {
            self.draw_edit(&mut frame)?;
        } else {
            self.draw_list(&mut frame);
        }
        Ok(frame)
    }

    fn draw_list(&self, frame: &mut RgbaImage) {
        render::darken(frame, (0, 0), (WIDTH as u32, HEIGHT as u32));
        render::text(frame, "worlds", 40, 2, WHITE);
        let names = self.entries.iter().map(|e| e.label());
        let names: Vec<_> = names.chain(["+ new world".into()]).collect();
        for (i, name) in names.iter().enumerate() {
            let y = 80 + i as i32 * 13;
            if i == self.selected {
                render::text(frame, &format!("> {name} <"), y, 1, WHITE);
            } else {
                render::text(frame, name, y, 1, GRAY);
            }
        }
        let hint = "enter open  esc back";
        render::text(frame, hint, HEIGHT as i32 - 20, 1, GRAY);
    }

    fn draw_edit(&self, frame: &mut RgbaImage) -> Result<()> {
        let t = TILE_SIZE as i32;
        let camera = &self.world.camera;
        let at = |x: usize, y: usize| {
            let (x, y) = ((x * TILE_SIZE) as f32, (y * TILE_SIZE) as f32);
            camera.world_to_screen(x, y)
        };

        // stuff the game doesn't draw
        let mut icons = vec![];
        for (y, row) in self.grid.iter().enumerate() {
            for (x, &c) in row.iter().enumerate() {
                let old = self.valid.get(y).and_then(|r| r.get(x));
                if old != Some(&c) {
                    dim(frame, at(x, y), (t, t));
                }
                if old != Some(&c) || c == 'S' || c == '*' {
                    icons.push((c, at(x, y)));
                }
            }
        }
        render::icons(frame, &icons, &self.world.level.palette)?;

        // where the world ends
        let (w, h) = (self.grid[0].len() as i32, self.grid.len() as i32);
        let (x, y) = at(0, 0);
        outline(frame, (x - 1, y - 1), (w * t + 2, h * t + 2), GRAY);

        // cursor, blinks a bit
        let (x, y) = at(self.cursor.0, self.cursor.1);
        let blink = (self.world.elapsed * 3.0) as i32 % 2 == 0;
        let color = if blink { WHITE } else { GRAY };
        outline(frame, (x - 1, y - 1), (t + 2, t + 2), color);

        // status bar, or what's wrong with the world
        render::darken(frame, (0, 0), (WIDTH as u32, TOP));
        let flash = self.flash.as_ref().filter(|(_, _, left)| *left > 0.0);
        if let Some(name) = &self.naming {
            let blink = (self.world.elapsed * 3.0) as i32 % 2 == 0;
            let cursor = if blink { "_" } else { " " };
            let text = format!("name: {name}{cursor}");
            render::text_at(frame, &text, (3, 3), 1, WHITE);
        } else if let Some((message, bad, _)) = flash {
            render::text(frame, message, 3, 1, if *bad { RED } else { WHITE });
        } else if let Some(error) = &self.error {
            render::text(frame, error, 3, 1, RED);
        } else {
            let name = self.entries[self.selected].label();
            render::text_at(frame, &name, (3, 3), 1, WHITE);
            render::text(frame, BRUSHES[self.brush].1, 3, 1, WHITE);
            let (cx, cy) = self.cursor;
            let info = format!("{w}x{h}  x{cx} y{cy}");
            let x = WIDTH as i32 - 3 - render::text_width(&info, 1);
            render::text_at(frame, &info, (x, 3), 1, GRAY);
        }

        // hotbar
        let top = HEIGHT as u32 - BOTTOM;
        render::darken(frame, (0, top), (WIDTH as u32, BOTTOM));
        let gap = t + 4;
        let x0 = (WIDTH as i32 - BRUSHES.len() as i32 * gap + 4) / 2;
        let y0 = top as i32 + 3;
        let slots: Vec<_> = BRUSHES
            .iter()
            .enumerate()
            .map(|(i, &(c, _))| (c, (x0 + i as i32 * gap, y0)))
            .collect();
        render::icons(frame, &slots, &self.world.level.palette)?;
        let x = x0 + self.brush as i32 * gap;
        outline(frame, (x - 2, y0 - 2), (t + 4, t + 4), WHITE);
        let hint = "tab tile  space put  x erase  enter test  ctrl-s save";
        render::text(frame, hint, HEIGHT as i32 - 19, 1, GRAY);
        let hint = match &self.naming {
            Some(_) => "type a name  enter ok  esc cancel",
            None => "n name  b background  m music",
        };
        render::text(frame, hint, HEIGHT as i32 - 10, 1, GRAY);

        Ok(())
    }
}

fn outline(
    frame: &mut RgbaImage,
    (x, y): (i32, i32),
    (w, h): (i32, i32),
    color: [u8; 3],
) {
    let mut dot = |px: i32, py: i32| {
        if let Some(p) = frame.get_pixel_mut_checked(px as u32, py as u32) {
            p.0 = [color[0], color[1], color[2], 255];
        }
    };
    for i in 0..w {
        dot(x + i, y);
        dot(x + i, y + h - 1);
    }
    for i in 0..h {
        dot(x, y + i);
        dot(x + w - 1, y + i);
    }
}

// render::darken but fine with being half off screen
fn dim(frame: &mut RgbaImage, (x, y): (i32, i32), (w, h): (i32, i32)) {
    for (px, py) in (x..x + w).flat_map(|px| (y..y + h).map(move |py| (px, py)))
    {
        if let Some(p) = frame.get_pixel_mut_checked(px as u32, py as u32) {
            p.0[..3].iter_mut().for_each(|c| *c /= 4);
        }
    }
}

impl Entry {
    // * when it has changes that aren't saved
    fn label(&self) -> String {
        let dirty = if self.source != self.saved { " *" } else { "" };
        format!("{}{dirty}", self.name)
    }
}
