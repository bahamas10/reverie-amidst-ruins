/*!
 * Level editor - pick a world from the list (or make a new one), then move a
 * cursor around it, place tiles, test it and save it to levels/
 */

mod draw;

use std::path::PathBuf;

use anyhow::Result;
use crossterm::event::ModifierKeyCode::{
    LeftControl, LeftShift, RightControl, RightShift,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::level::{LevelDefinition, TILE_SIZE};
use crate::screen::LevelScreen;
use crate::{HEIGHT, WIDTH};

// the editor works on the level files in the repo, not the ones baked into
// the binary
const LEVELS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/levels");

// what you can place, (level char, name)
const BRUSHES: &[(char, &str)] = &[
    ('G', "ground"),
    ('C', "coin"),
    ('^', "spring"),
    ('-', "jump thru"),
    ('X', "spikes"),
    ('%', "crumble block"),
    ('=', "moving platform"),
    ('*', "platform target"),
    ('S', "spawn"),
    ('F', "finish"),
];

// what b and m cycle through, same names render.rs and audio.rs use
const BACKGROUNDS: &[&str] = &["nebula", "snow", "storm"];
const SONGS: &[&str] = &[
    "reverie-amidst-ruins",
    "echoes-of-solace",
    "colors",
    "the-final-storm",
    "blueprints",
];

// longest name you can type, and what the font can draw
const NAME_LENGTH: usize = 24;
const NAME_CHARS: &str = " :-.,()<>*+/";

// holding an arrow moves again after this long, then every REPEAT
const REPEAT_DELAY: f32 = 0.25;
const REPEAT: f32 = 0.07;

// camera keeps the cursor this many tiles from the edge
const MARGIN: f32 = 3.0;
const FOLLOW_SPEED: f32 = 12.0;

// pixels covered by the status bar and the hotbar
const TOP: u32 = 12;
const BOTTOM: u32 = 42;

// how long "saved" etc stays up
const FLASH: f32 = 2.0;

// what main should do after an update
pub enum Exit {
    Title,
    Test(Box<LevelScreen>),
    Music(String),
}

struct Entry {
    path: PathBuf,
    name: String,
    // edits that parse end up here, saved is what's on disk
    source: String,
    saved: String,
}

pub struct Editor {
    entries: Vec<Entry>,
    // the last one past the entries is "new world"
    selected: usize,
    // the selected world as the game sees it (last version that parsed)
    world: Box<LevelScreen>,
    // editing instead of picking a world
    open: bool,
    // metadata lines and tiles of the world being edited
    header: String,
    grid: Vec<Vec<char>>,
    // what `world` was built from, so edits that don't parse still show
    valid: Vec<Vec<char>>,
    error: Option<String>,
    // message, is it bad, time left
    flash: Option<(String, bool, f32)>,
    cursor: (usize, usize),
    brush: usize,
    repeat: f32,
    // left, right, up, down, shift, ctrl, space
    held: [bool; 7],
    // the name being typed after pressing n
    naming: Option<String>,
    // playing the world's song instead of the editor's
    preview: bool,
}

impl Editor {
    pub fn new(worlds: &[(String, &str)]) -> Result<Self> {
        let mut files = vec![];
        match std::fs::read_dir(LEVELS_DIR) {
            Ok(dir) => {
                for file in dir.flatten() {
                    let path = file.path();
                    if path.extension().is_some_and(|e| e == "txt") {
                        let source = std::fs::read_to_string(&path)?;
                        files.push((path, source));
                    }
                }
            }
            Err(_) => {
                for (file, source) in worlds {
                    let path = PathBuf::from(LEVELS_DIR).join(file);
                    files.push((path, source.to_string()));
                }
            }
        }
        // level1, level2, ... first, then the rest
        files.sort_by_key(|(path, _)| {
            let stem = stem(path);
            (!stem.starts_with("level"), stem)
        });

        let mut entries = vec![];
        for (path, source) in files {
            // skip anything that isn't a level
            let Ok(level) = LevelDefinition::parse(&source) else { continue };
            // the real levels are all called TODO lol
            let todo = level.name == "TODO";
            let name = if todo { stem(&path) } else { level.name };
            entries.push(Entry { path, name, saved: source.clone(), source });
        }

        let world = load(0, &entries[0].source)?;
        Ok(Self {
            entries,
            selected: 0,
            world,
            open: false,
            header: String::new(),
            grid: vec![],
            valid: vec![],
            error: None,
            flash: None,
            cursor: (0, 0),
            brush: 0,
            repeat: 0.0,
            held: [false; 7],
            naming: None,
            preview: false,
        })
    }

    // main lets q quit unless we're typing
    pub fn typing(&self) -> bool {
        self.naming.is_some()
    }

    pub fn update(
        &mut self,
        events: &[KeyEvent],
        dt: f32,
    ) -> Result<Option<Exit>> {
        for key in events {
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            let shift = key.modifiers.contains(KeyModifiers::SHIFT);

            // typing a name eats every key
            if let Some(name) = &mut self.naming {
                if key.kind == KeyEventKind::Release {
                    continue;
                }
                match key.code {
                    KeyCode::Esc => self.naming = None,
                    KeyCode::Enter => {
                        let name = self.naming.take().unwrap_or_default();
                        self.rename(name.trim());
                    }
                    KeyCode::Backspace => {
                        name.pop();
                    }
                    KeyCode::Char(c) if !ctrl && name.len() < NAME_LENGTH => {
                        let c = if shift { c.to_ascii_uppercase() } else { c };
                        if c.is_ascii_alphanumeric() || NAME_CHARS.contains(c) {
                            name.push(c);
                        }
                    }
                    _ => {}
                }
                continue;
            }

            let held = match key.code {
                KeyCode::Left | KeyCode::Char('a' | 'A') => Some(0),
                KeyCode::Right | KeyCode::Char('d' | 'D') => Some(1),
                KeyCode::Up | KeyCode::Char('w' | 'W') => Some(2),
                // ctrl+s saves instead
                KeyCode::Down => Some(3),
                KeyCode::Char('s' | 'S') if !ctrl => Some(3),
                KeyCode::Modifier(LeftShift | RightShift) => Some(4),
                KeyCode::Modifier(LeftControl | RightControl) => Some(5),
                KeyCode::Char(' ') => Some(6),
                _ => None,
            };
            if let Some(i) = held {
                self.held[i] = key.kind != KeyEventKind::Release;
            }
            if key.kind == KeyEventKind::Release {
                continue;
            }
            let press = key.kind == KeyEventKind::Press;

            if !self.open {
                match key.code {
                    KeyCode::Esc => return Ok(Some(Exit::Title)),
                    KeyCode::Enter if press => {
                        if self.selected == self.entries.len() {
                            self.new_world();
                        }
                        self.edit();
                    }
                    _ if matches!(held, Some(2 | 3)) => {
                        let n = self.entries.len() + 1;
                        let step = if held == Some(2) { n - 1 } else { 1 };
                        self.selected = (self.selected + step) % n;
                        let source = match self.entries.get(self.selected) {
                            Some(entry) => entry.source.clone(),
                            None => template(&self.new_name()),
                        };
                        self.world = load(self.selected, &source)?;
                    }
                    _ => {}
                }
                continue;
            }

            let (n, brush) = (BRUSHES.len(), BRUSHES[self.brush].0);
            let back = shift;
            match key.code {
                KeyCode::Esc => {
                    self.open = false;
                    // back to the editor's song
                    if std::mem::take(&mut self.preview) {
                        return Ok(Some(Exit::Music("blueprints".into())));
                    }
                }
                KeyCode::Char('n' | 'N') if press => {
                    let name = self.get("name").to_string();
                    self.naming = Some(name);
                }
                KeyCode::Char('b' | 'B') => {
                    let bg = cycle(BACKGROUNDS, self.get("background"), back);
                    self.set("background", bg);
                    self.flash(&format!("background: {bg}"), false);
                }
                KeyCode::Char('m' | 'M') => {
                    let song = cycle(SONGS, self.get("music"), back);
                    self.set("music", song);
                    self.flash(&format!("music: {song}"), false);
                    self.preview = true;
                    return Ok(Some(Exit::Music(song.into())));
                }
                KeyCode::Enter if press => match &self.error {
                    Some(_) => self.flash("fix the error first", true),
                    None => {
                        let source = &self.entries[self.selected].source;
                        let world = load(self.selected, source)?;
                        return Ok(Some(Exit::Test(world)));
                    }
                },
                KeyCode::Char('s' | 'S') if ctrl && press => self.save(),
                KeyCode::BackTab => self.brush = (self.brush + n - 1) % n,
                KeyCode::Tab if back => self.brush = (self.brush + n - 1) % n,
                KeyCode::Tab => self.brush = (self.brush + 1) % n,
                KeyCode::Char(' ') if press => self.place(brush),
                KeyCode::Char('x' | 'X') => self.place('.'),
                // first step right away, holding repeats in update below
                _ if press && matches!(held, Some(0..4)) => {
                    let dirs = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                    let (dx, dy) = dirs[held.unwrap()];
                    self.step(dx, dy);
                    self.repeat = REPEAT_DELAY;
                }
                _ => {}
            }
        }

        // coins etc keep spinning
        self.world.elapsed += dt;
        if let Some((_, _, left)) = &mut self.flash {
            *left -= dt;
        }
        if !self.open {
            return Ok(None);
        }

        // our own key repeat so shift (2x) and ctrl (3x) can speed it up
        let [left, right, up, down, shift, ctrl, _] = self.held;
        if left || right || up || down {
            let axis = |a: bool, b: bool| b as i32 - a as i32;
            let boost = if ctrl { 3.0 } else if shift { 2.0 } else { 1.0 };
            self.repeat -= dt * boost;
            while self.repeat <= 0.0 {
                self.step(axis(left, right), axis(up, down));
                self.repeat += REPEAT;
            }
        }

        self.follow(dt);
        Ok(None)
    }

    // world-1.txt, world-2.txt, ... whatever's free
    fn new_name(&self) -> String {
        let taken = |n: &usize| {
            let name = format!("world-{n}");
            self.entries.iter().any(|e| stem(&e.path) == name)
        };
        let n = (1..).find(|n| !taken(n)).unwrap();
        format!("world-{n}")
    }

    // not on disk until it's saved
    fn new_world(&mut self) {
        let name = self.new_name();
        let path = PathBuf::from(LEVELS_DIR).join(format!("{name}.txt"));
        let source = template(&name);
        let saved = String::new();
        self.entries.push(Entry { path, name, source, saved });
    }

    // start editing the selected world
    fn edit(&mut self) {
        let source = &self.entries[self.selected].source;
        let (header, tiles) = source.split_once("\n\n").unwrap_or((source, ""));
        self.header = header.to_string();
        self.grid = tiles.lines().map(|line| line.chars().collect()).collect();
        let width = self.grid.iter().map(Vec::len).max().unwrap_or(0);
        self.grid.iter_mut().for_each(|row| row.resize(width, '.'));
        self.valid = self.grid.clone();
        self.cursor = self.world.level.spawn;
        self.error = None;
        self.open = true;
    }

    fn save(&mut self) {
        if let Some(error) = &self.error {
            let message = format!("can't save: {error}");
            return self.flash(&message, true);
        }
        let entry = &mut self.entries[self.selected];
        let result = std::fs::create_dir_all(LEVELS_DIR)
            .and_then(|_| std::fs::write(&entry.path, &entry.source));
        let file = entry.path.file_name().unwrap().to_string_lossy();
        let (message, bad) = match result {
            Ok(_) => {
                entry.saved = entry.source.clone();
                (format!("saved levels/{file}"), false)
            }
            Err(err) => (format!("can't save {file}: {err}"), true),
        };
        self.flash(&message, bad);
    }

    fn flash(&mut self, message: &str, bad: bool) {
        self.flash = Some((message.to_string(), bad, FLASH));
    }

    fn step(&mut self, dx: i32, dy: i32) {
        // can go one past the right and bottom edge, placing there grows it
        let (w, h) = (self.grid[0].len() as i32, self.grid.len() as i32);
        let x = (self.cursor.0 as i32 + dx).clamp(0, w) as usize;
        let y = (self.cursor.1 as i32 + dy).clamp(0, h) as usize;
        if (x, y) == self.cursor {
            return;
        }
        self.cursor = (x, y);

        // holding space paints
        if self.held[6] {
            self.place(BRUSHES[self.brush].0);
        }
    }

    fn place(&mut self, c: char) {
        let (x, y) = self.cursor;
        let inside = y < self.grid.len() && x < self.grid[0].len();
        if inside && self.grid[y][x] == c || !inside && c == '.' {
            return;
        }

        if x == self.grid[0].len() {
            self.grid.iter_mut().for_each(|row| row.push('.'));
        }
        if y == self.grid.len() {
            self.grid.push(vec!['.'; self.grid[0].len()]);
        }

        // only one spawn and finish, placing moves it
        if c == 'S' || c == 'F' {
            let old = self.grid.iter_mut().flatten().filter(|t| **t == c);
            old.for_each(|t| *t = '.');
        }
        self.grid[y][x] = c;
        self.reload();
    }

    // the game's own parser decides if it's ok
    fn reload(&mut self) {
        let text = self.text();
        match LevelScreen::load(self.selected, &text) {
            Ok(mut world) => {
                world.camera = std::mem::take(&mut self.world.camera);
                world.elapsed = self.world.elapsed;
                *self.world = world;
                self.valid = self.grid.clone();
                self.entries[self.selected].source = text;
                self.error = None;
            }
            Err(err) => self.error = Some(format!("{err:#}")),
        }
    }

    // a metadata line, like `music: colors`
    fn get(&self, key: &str) -> &str {
        let line = self.header.lines().find_map(|line| {
            let (k, v) = line.split_once(':')?;
            (k.trim() == key).then(|| v.trim())
        });
        line.unwrap_or_default()
    }

    fn set(&mut self, key: &str, value: &str) {
        let lines: Vec<String> = self
            .header
            .lines()
            .map(|line| match line.split_once(':') {
                Some((k, _)) if k.trim() == key => format!("{key}: {value}"),
                _ => line.to_string(),
            })
            .collect();
        self.header = lines.join("\n");
        self.reload();
    }

    fn rename(&mut self, name: &str) {
        if name.is_empty() {
            return self.flash("name can't be empty", true);
        }
        self.set("name", name);
        self.entries[self.selected].name = name.to_string();
        self.flash(&format!("renamed to {name}"), false);
    }

    // back to the level file format
    fn text(&self) -> String {
        let rows: Vec<String> =
            self.grid.iter().map(|row| row.iter().collect()).collect();
        format!("{}\n\n{}\n", self.header, rows.join("\n"))
    }

    // keep the cursor on screen, clear of the bars
    fn follow(&mut self, dt: f32) {
        let t = TILE_SIZE as f32;
        let (cx, cy) = (self.cursor.0 as f32 * t, self.cursor.1 as f32 * t);
        // the world plus the column/row past the edge
        let w = (self.grid[0].len() + 1) as f32 * t;
        let h = (self.grid.len() + 1) as f32 * t;
        let (top, bottom) = (TOP as f32, BOTTOM as f32);
        let (vw, vh) = (WIDTH as f32, HEIGHT as f32);
        let m = MARGIN * t;

        let camera = &mut self.world.camera;
        let x = camera.x.clamp(cx + t + m - vw, cx - m);
        let y = camera.y.clamp(cy + t + m + bottom - vh, cy - m - top);
        let x = x.clamp(0.0, (w - vw).max(0.0));
        let y = y.clamp(-top, (h + bottom - vh).max(-top));

        let amount = 1.0 - (-FOLLOW_SPEED * dt).exp();
        camera.x += (x - camera.x) * amount;
        camera.y += (y - camera.y) * amount;
    }
}

// next (or previous) one after current
fn cycle<'a>(list: &[&'a str], current: &str, back: bool) -> &'a str {
    let i = list.iter().position(|s| *s == current).unwrap_or(0);
    let step = if back { list.len() - 1 } else { 1 };
    list[(i + step) % list.len()]
}

// a small room to start from
fn template(name: &str) -> String {
    let mut rows = vec![".".repeat(24); 14];
    rows[11].replace_range(2..3, "S");
    rows[12] = "G".repeat(24);
    rows[13] = "G".repeat(24);
    format!(
        "name: {name}\nmusic: colors\nbackground: nebula\n\
         palette: #071A1D, #174B45, #2F8F5B, #A8D96F\n\n{}\n",
        rows.join("\n")
    )
}

// camera starts where the player would (boxed, LevelScreen is big)
fn load(index: usize, source: &str) -> Result<Box<LevelScreen>> {
    let mut world = LevelScreen::load(index, source)?;
    let (w, h) = size(&world.level);
    world.camera.snap_to(&world.player, w, h, WIDTH as f32, HEIGHT as f32);
    Ok(Box::new(world))
}

// in pixels
fn size(level: &LevelDefinition) -> (f32, f32) {
    ((level.width * TILE_SIZE) as f32, (level.height * TILE_SIZE) as f32)
}

fn stem(path: &std::path::Path) -> String {
    path.file_stem().unwrap_or_default().to_string_lossy().into_owned()
}
