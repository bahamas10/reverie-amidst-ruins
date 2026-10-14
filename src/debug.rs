/*!
 * Debug menu (F4)
 */

use std::io::Write;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::terminal::ImagePlacement;

// menu size in terminal cells, 1 cell in so the HUD line still shows
pub const OFFSET: u16 = 1;
pub const WIDTH: u16 = 44;
pub const HEIGHT: u16 = 12;

const ROWS: usize = 8;
const RESULTS: usize = 6;

#[derive(Default)]
pub struct Overlays {
    pub stats: bool,
    pub hurtbox: bool,
    pub pickup: bool,
    pub blocks: bool,
    pub entities: bool,
}

enum Page {
    Main,
    Teleport { query: String, selected: usize },
}

pub struct DebugMenu {
    pub open: bool,
    pub overlays: Overlays,
    page: Page,
    cursor: usize,
}

impl DebugMenu {
    pub fn new(stats: bool) -> Self {
        let overlays = Overlays { stats, ..Default::default() };
        Self { open: false, overlays, page: Page::Main, cursor: 0 }
    }

    // returns a level index if we should teleport
    pub fn update(
        &mut self,
        events: &[KeyEvent],
        levels: &[String],
    ) -> Option<usize> {
        for key in events.iter().filter(|k| k.kind != KeyEventKind::Release) {
            if key.code == KeyCode::F(4) {
                self.open = !self.open;
                self.page = Page::Main;
                continue;
            }
            if !self.open {
                continue;
            }

            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            match &mut self.page {
                Page::Main => match key.code {
                    KeyCode::Esc => self.open = false,
                    KeyCode::Up | KeyCode::Char('w') => {
                        self.cursor = (self.cursor + ROWS - 1) % ROWS
                    }
                    KeyCode::Down | KeyCode::Char('s') => {
                        self.cursor = (self.cursor + 1) % ROWS
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => self.toggle(),
                    _ => {}
                },
                Page::Teleport { query, selected } => {
                    let found = matches(query, levels);
                    match key.code {
                        KeyCode::Esc => self.page = Page::Main,
                        KeyCode::Enter if !found.is_empty() => {
                            self.open = false;
                            return Some(found[*selected].0);
                        }
                        KeyCode::Up => *selected = selected.saturating_sub(1),
                        KeyCode::Char('p') if ctrl => {
                            *selected = selected.saturating_sub(1)
                        }
                        KeyCode::Down => {
                            *selected =
                                (*selected + 1).min(found.len().max(1) - 1)
                        }
                        KeyCode::Char('n') if ctrl => {
                            *selected =
                                (*selected + 1).min(found.len().max(1) - 1)
                        }
                        // results change, so jump back to the top
                        KeyCode::Char('u') if ctrl => {
                            query.clear();
                            *selected = 0;
                        }
                        KeyCode::Backspace => {
                            query.pop();
                            *selected = 0;
                        }
                        KeyCode::Char(c) if !ctrl => {
                            query.push(c);
                            *selected = 0;
                        }
                        _ => {}
                    }
                }
            }
        }

        None
    }

    // groups turn everything on, unless it's all on already
    fn toggle(&mut self) {
        let o = &mut self.overlays;
        match self.cursor {
            0 => o.stats = !o.stats,
            1 => {
                let on = !(o.hurtbox && o.pickup && o.blocks && o.entities);
                (o.hurtbox, o.pickup, o.blocks, o.entities) = (on, on, on, on);
            }
            2 => {
                let on = !(o.hurtbox && o.pickup);
                (o.hurtbox, o.pickup) = (on, on);
            }
            3 => o.hurtbox = !o.hurtbox,
            4 => o.pickup = !o.pickup,
            5 => o.blocks = !o.blocks,
            6 => o.entities = !o.entities,
            _ => {
                self.page = Page::Teleport { query: String::new(), selected: 0 }
            }
        }
    }

    // text goes on top of the image, \x1b[K wipes whatever was there before
    pub fn draw(&self, levels: &[String], placement: ImagePlacement) {
        let lines = match &self.page {
            Page::Main => self.main_lines(),
            Page::Teleport { query, selected } => {
                teleport_lines(query, *selected, levels)
            }
        };

        let column = placement.column + OFFSET + 1;
        let mut out = String::new();
        for i in 0..HEIGHT {
            let row = placement.row + OFFSET + i + 1;
            let line = lines.get(i as usize).map_or("", |s| s.as_str());
            out += &format!("\x1b[{row};{column}H{line}\x1b[0m\x1b[K");
        }

        print!("{out}");
        let _ = std::io::stdout().flush();
    }

    fn main_lines(&self) -> Vec<String> {
        let o = &self.overlays;
        let rows = [
            (check(&[o.stats]), "stats (frame bandwidth)"),
            (check(&[o.hurtbox, o.pickup, o.blocks, o.entities]), "hitboxes/"),
            (check(&[o.hurtbox, o.pickup]), "  player/"),
            (check(&[o.hurtbox]), "    hurtbox \x1b[31m■"),
            (check(&[o.pickup]), "    pickup \x1b[33m■"),
            (check(&[o.blocks]), "  blocks \x1b[34m■"),
            (check(&[o.entities]), "  entities \x1b[32m■"),
            ("   ", "teleport to level..."),
        ];

        let mut lines = vec!["\x1b[1mdebug menu".to_string(), String::new()];
        for (i, (check, label)) in rows.iter().enumerate() {
            lines.push(format!("{} {check} {label}", cursor(i == self.cursor)));
        }
        lines.push(String::new());
        lines.push("\x1b[2m↑↓ move  enter toggle  esc/f4 close".into());
        lines
    }
}

fn teleport_lines(
    query: &str,
    selected: usize,
    levels: &[String],
) -> Vec<String> {
    let found = matches(query, levels);
    let mut lines = vec![
        "\x1b[1mteleport to level".to_string(),
        String::new(),
        format!("\x1b[36m>\x1b[0m {query}\x1b[2m_"),
        format!("\x1b[2m  {}/{}", found.len(), levels.len()),
    ];

    // scroll so the selection stays on screen
    let start = selected.saturating_sub(RESULTS - 1);
    for (i, (index, hits)) in found.iter().enumerate().skip(start).take(RESULTS)
    {
        // highlight matched chars like fzf
        let name: String = levels[*index]
            .chars()
            .enumerate()
            .map(|(j, c)| {
                if hits.contains(&j) {
                    format!("\x1b[32m{c}\x1b[0m")
                } else {
                    c.to_string()
                }
            })
            .collect();
        lines.push(format!("{}{name}", cursor(i == selected)));
    }

    lines.resize(4 + RESULTS + 1, String::new());
    lines.push("\x1b[2mtype to search  ↑↓ pick  enter go  esc back".into());
    lines
}

fn cursor(selected: bool) -> &'static str {
    if selected { "\x1b[36m>\x1b[0m " } else { "  " }
}

fn check(values: &[bool]) -> &'static str {
    match (values.iter().all(|&v| v), values.iter().any(|&v| v)) {
        (true, _) => "[x]",
        (_, true) => "[~]",
        _ => "[ ]",
    }
}

// best match first, ties stay in level order
fn matches(query: &str, levels: &[String]) -> Vec<(usize, Vec<usize>)> {
    let mut found: Vec<_> = levels
        .iter()
        .enumerate()
        .filter_map(|(i, level)| {
            fuzzy(query, level).map(|(score, hits)| (score, i, hits))
        })
        .collect();
    found.sort_by_key(|(score, _, _)| -score);
    found.into_iter().map(|(_, i, hits)| (i, hits)).collect()
}

// fzf-ishd
fn fuzzy(query: &str, text: &str) -> Option<(i32, Vec<usize>)> {
    let text: Vec<char> =
        text.chars().map(|c| c.to_ascii_lowercase()).collect();
    let mut hits: Vec<usize> = vec![];
    let mut score = 0;

    for q in query.chars().filter(|c| !c.is_whitespace()) {
        let from = hits.last().map_or(0, |&i| i + 1);
        let i =
            (from..text.len()).find(|&i| text[i] == q.to_ascii_lowercase())?;

        score += match hits.last() {
            Some(&last) if i == last + 1 => 25,
            Some(&last) => 10 - (i - last - 1) as i32,
            None => 10 - i as i32,
        };
        if i == 0 || !text[i - 1].is_alphanumeric() {
            score += 8;
        }
        hits.push(i);
    }

    Some((score, hits))
}
