/*!
 * Level editor - pick a world from the list and look around it
 */

use anyhow::Result;
use crossterm::event::ModifierKeyCode::{
    LeftControl, LeftShift, RightControl, RightShift,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use image::RgbaImage;

use crate::level::{LevelDefinition, TILE_SIZE};
use crate::render;
use crate::screen::LevelScreen;
use crate::{HEIGHT, WIDTH};

// pixels per second
const SCROLL_SPEED: f32 = 240.0;

const WHITE: [u8; 3] = [0xff; 3];
const GRAY: [u8; 3] = [0x80; 3];

pub struct Editor {
    names: Vec<String>,
    selected: usize,
    // the selected world, also drawn behind the list
    world: LevelScreen,
    // looking around the world instead of picking one
    open: bool,
    // left, right, up, down, shift, ctrl
    held: [bool; 6],
}

impl Editor {
    pub fn new(worlds: &[&str]) -> Result<Self> {
        let mut names = vec![];
        for (i, source) in worlds.iter().enumerate() {
            let name = LevelDefinition::parse(source)?.name;
            // basically Dave named all the levels TODO
            let todo = name == "TODO";
            names.push(if todo { format!("level {}", i + 1) } else { name });
        }
        let world = load(0, worlds)?;
        Ok(Self { names, selected: 0, world, open: false, held: [false; 6] })
    }

    // true when esc on the list, so we go back to the title
    pub fn update(
        &mut self,
        events: &[KeyEvent],
        worlds: &[&str],
        dt: f32,
    ) -> Result<bool> {
        for key in events {
            let arrow = match key.code {
                KeyCode::Left | KeyCode::Char('a' | 'A') => Some(0),
                KeyCode::Right | KeyCode::Char('d' | 'D') => Some(1),
                KeyCode::Up | KeyCode::Char('w' | 'W') => Some(2),
                KeyCode::Down | KeyCode::Char('s' | 'S') => Some(3),
                KeyCode::Modifier(LeftShift | RightShift) => Some(4),
                KeyCode::Modifier(LeftControl | RightControl) => Some(5),
                _ => None,
            };
            if let Some(i) = arrow {
                self.held[i] = key.kind != KeyEventKind::Release;
            }
            if key.kind == KeyEventKind::Release {
                continue;
            }

            match (self.open, key.code) {
                (true, KeyCode::Esc) => self.open = false,
                (false, KeyCode::Esc) => return Ok(true),
                (false, KeyCode::Enter) => self.open = true,
                (false, _) if matches!(arrow, Some(2 | 3)) => {
                    let n = worlds.len();
                    let step = if arrow == Some(2) { n - 1 } else { 1 };
                    self.selected = (self.selected + step) % n;
                    self.world = load(self.selected, worlds)?;
                }
                _ => {}
            }
        }

        // process
        self.world.elapsed += dt;

        if self.open {
            let [left, right, up, down, shift, ctrl] = self.held; // (panning)
            let axis = |a: bool, b: bool| b as u8 as f32 - a as u8 as f32;
            let (w, h) = size(&self.world.level);
            let max_x = (w - WIDTH as f32).max(0.0);
            let max_y = (h - HEIGHT as f32).max(0.0);
            let camera = &mut self.world.camera;
            // shift 2x, ctrl 3x
            let boost = if ctrl { 3.0 } else if shift { 2.0 } else { 1.0 };
            let speed = SCROLL_SPEED * boost * dt;
            camera.x = (camera.x + axis(left, right) * speed).clamp(0.0, max_x);
            camera.y = (camera.y + axis(up, down) * speed).clamp(0.0, max_y);
        }

        Ok(false)
    }

    pub fn render(&self) -> Result<RgbaImage> {
        let (width, height) = (WIDTH as u32, HEIGHT as u32);
        let mut frame = render::level(&self.world, None, width, height)?;

        if self.open {
            render::darken(&mut frame, (0, height - 13), (width, 13));
            let hint = "arrows scroll  esc back";
            render::text(&mut frame, hint, height as i32 - 10, 1, GRAY);
            return Ok(frame);
        }

        render::darken(&mut frame, (0, 0), (width, height));
        render::text(&mut frame, "worlds", 40, 2, WHITE);
        for (i, name) in self.names.iter().enumerate() {
            let y = 80 + i as i32 * 13;
            if i == self.selected {
                render::text(&mut frame, &format!("> {name} <"), y, 1, WHITE);
            } else {
                render::text(&mut frame, name, y, 1, GRAY);
            }
        }
        let hint = "enter open  esc back";
        render::text(&mut frame, hint, height as i32 - 20, 1, GRAY);

        Ok(frame)
    }
}

// camera starts where the player would
fn load(index: usize, worlds: &[&str]) -> Result<LevelScreen> {
    let mut world = LevelScreen::load(index, worlds[index])?;
    let (w, h) = size(&world.level);
    world.camera.snap_to(&world.player, w, h, WIDTH as f32, HEIGHT as f32);
    Ok(world)
}

// in pixels
fn size(level: &LevelDefinition) -> (f32, f32) {
    ((level.width * TILE_SIZE) as f32, (level.height * TILE_SIZE) as f32)
}
