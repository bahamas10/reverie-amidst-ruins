/*!
 * Level editor (not much here yet lol)
 */

use image::{Rgba, RgbaImage};

use crate::render;

pub struct Editor;

impl Editor {
    pub fn render(&self, width: u32, height: u32) -> RgbaImage {
        let mut frame =
            RgbaImage::from_pixel(width, height, Rgba([7, 24, 33, 255]));
        render::text(&mut frame, "TODO", height as i32 / 2 - 28, 8, [255; 3]);
        frame
    }
}
