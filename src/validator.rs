/*!
 * Validate that a level can be completed.
 *
 * The levels are hand-drawn ASCII, and nothing about the format guarantees one
 * is beatable: a platform can be one pixel too high, a gap one tile too wide, a
 * coin walled in. This module answers that with the game's own rules - it parses
 * with [`LevelDefinition::parse`] and steps the real [`Player::update`] - so a
 * level it rejects is one the game rejects, not a second approximation of one.
 *
 * The search is a macro-action walk: a **node** is a place the player came to
 * rest (pixel position plus the horizontal momentum it arrived with), and an
 * **arc** is one input program - direction, jump hold, and an optional mid-air
 * stop or reversal - simulated until it lands. That covers the moves a player
 * actually makes, including the mid-air correction a tight jump needs, without
 * searching every frame of every trajectory.
 */

use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::input::Input;
use crate::level::{LevelDefinition, TILE_SIZE};
use crate::player::{Player, HITBOX_HEIGHT, HITBOX_WIDTH};
use crate::DEATH_PLANE;

/// The fixed step the search simulates: 60 Hz is what the game's physics was
/// tuned for.
const STEP: f32 = 1.0 / 60.0;
/// How long one arc may run before it is abandoned (1.6 s at 60 Hz).
const MAX_FRAMES: usize = 96;
/// The most nodes one search explores before it reports itself truncated.
const DEFAULT_BUDGET: usize = 60_000;
/// Where in a landing's footprint an arc may take off from: a body may overhang
/// a ledge by its own width, and which end it stands on decides a marginal jump.
const TAKEOFFS: &[f32] = &[-8.0, 0.0, 8.0];

/// The verdict for one level.
#[derive(Debug)]
pub struct Report {
    /// Whether the goal was reached.
    pub goal: bool,
    /// How many of the level's coins the walk collected.
    pub coins: usize,
    /// How many coins the level has.
    pub total_coins: usize,
    /// How many cells the body occupied.
    pub cells: usize,
    /// How many cells the level has.
    pub total_cells: usize,
    /// How many nodes the search explored.
    pub nodes: usize,
    /// Whether the budget ran out before the walk finished.
    pub truncated: bool,
}

impl Report {
    /// Whether the level is beatable and every coin is reachable.
    pub fn playable(&self) -> bool {
        self.goal && self.coins == self.total_coins && !self.truncated
    }

    /// Why the level failed, or `None` when it did not.
    pub fn failure(&self) -> Option<String> {
        if self.truncated {
            return Some(format!(
                "the search hit its {DEFAULT_BUDGET}-node budget"
            ));
        }
        if !self.goal {
            return Some("no route to the goal".to_string());
        }
        if self.coins < self.total_coins {
            return Some(format!(
                "{} coin(s) unreachable",
                self.total_coins - self.coins
            ));
        }
        None
    }
}

/// One input program: the direction held from takeoff, how long jump is held,
/// and an optional mid-air change of direction.
#[derive(Clone, Copy)]
struct Policy {
    /// The direction held from takeoff: -1, 0 or 1.
    dir: i8,
    /// Frames the jump key is held; `-1` never jumps (a walk or a walk-off).
    hold: i8,
    /// The frame the direction changes (`-1` never).
    switch_at: i16,
    /// The direction after the change.
    switch_dir: i8,
}

/// Every program the search tries from every takeoff point.
///
/// The jump holds sample the climb from a tap to full height; the mid-air
/// changes are the ones that make a reversal or a mid-air stop reachable at all.
fn policies() -> Vec<Policy> {
    let mut out = Vec::new();
    for dir in [-1i8, 0, 1] {
        // Walks and walk-offs. Standing still is not a move, so only the two
        // directions have a walk of their own.
        if dir != 0 {
            out.push(Policy { dir, hold: -1, switch_at: -1, switch_dir: 0 });
            for (switch_at, switch_dir) in [(10i16, 0i8), (20, -dir)] {
                out.push(Policy { dir, hold: -1, switch_at, switch_dir });
            }
        }
        // Jumps at five holds, each with and without a mid-air change.
        for hold in [0i8, 6, 12, 18, 26] {
            out.push(Policy { dir, hold, switch_at: -1, switch_dir: 0 });
            for (switch_at, switch_dir) in [(8i16, 0i8), (20, -dir)] {
                out.push(Policy { dir, hold, switch_at, switch_dir });
            }
        }
    }
    out
}

/// Validate one level with the default budget.
pub fn validate(level: &LevelDefinition) -> Report {
    search(level, DEFAULT_BUDGET)
}

/// Validate one level file, or every built-in level when no path is given.
///
/// Prints one line per level and returns an error - and so a non-zero exit -
/// when any level is not fully playable. This is what `--validate` calls, and
/// what CI runs.
pub fn run(target: Option<&Path>, levels: &[&str]) -> Result<()> {
    let mut total = 0;
    let mut failures = 0;

    match target {
        Some(path) => {
            let source = std::fs::read_to_string(path).with_context(|| {
                format!("could not read {}", path.display())
            })?;
            total += 1;
            if !validate_one(&path.display().to_string(), &source) {
                failures += 1;
            }
        }
        None => {
            for (index, source) in levels.iter().enumerate() {
                total += 1;
                if !validate_one(&format!("level {}", index + 1), source) {
                    failures += 1;
                }
            }
        }
    }

    if failures > 0 {
        bail!("{failures} of {total} level(s) failed validation");
    }
    Ok(())
}

/// Parse, search and print one level. Returns whether it is fully playable.
fn validate_one(label: &str, source: &str) -> bool {
    let started = Instant::now();
    let level = match LevelDefinition::parse(source) {
        Ok(level) => level,
        Err(error) => {
            println!("{label}: INVALID - {error:#}");
            return false;
        }
    };
    let report = validate(&level);
    let elapsed = started.elapsed().as_secs_f32();
    let verdict = match report.failure() {
        None => "ok".to_string(),
        Some(reason) => format!("FAILED - {reason}"),
    };
    println!(
        "{label}: {verdict} ({} nodes in {elapsed:.1}s, {}/{} cells, {}/{} coins)",
        report.nodes,
        report.cells,
        report.total_cells,
        report.coins,
        report.total_coins,
    );
    report.playable()
}

/// Walk the level from the spawn, one landing at a time.
fn search(level: &LevelDefinition, budget: usize) -> Report {
    let width = level.width;
    let height = level.height;
    let cells = width * height;
    let max_x = width as f32 * TILE_SIZE as f32 - HITBOX_WIDTH;
    let death = height as f32 * TILE_SIZE as f32 + DEATH_PLANE;

    // One array index per lookup: the hot loop must not look a coin up by cell.
    let mut coin_at = vec![-1i32; cells];
    for (index, &(x, y)) in level.coins.iter().enumerate() {
        coin_at[y * width + x] = index as i32;
    }
    let goal_cell = (level.goal.1 * width + level.goal.0) as i32;

    let mut reached = vec![false; cells];
    let mut coin_hit = vec![false; level.coins.len()];
    let mut coins_left = level.coins.len();
    let mut goal = false;

    // The spawn is airborne: the game drops the player there. The first arcs are
    // the fall, with whatever air control each policy asks for.
    let spawn = Player::new(
        (level.spawn.0 * TILE_SIZE) as f32,
        (level.spawn.1 * TILE_SIZE) as f32,
    );
    let mut nodes: Vec<Player> = Vec::with_capacity(1024);
    nodes.push(spawn.clone());
    let mut seen: HashSet<u64> = HashSet::with_capacity(1024);
    seen.insert(node_key(&spawn));
    let programs = policies();
    let mut head = 0;
    let mut truncated = false;

    'walk: while head < nodes.len() {
        if goal && coins_left == 0 {
            break;
        }
        if nodes.len() >= budget {
            truncated = true;
            break;
        }
        // Take the landing out so the loop can push to `nodes`.
        let landing = nodes[head].clone();
        head += 1;

        // A landing may take off from anywhere along its footprint; a body in
        // the air only from where it is.
        let offsets: &[f32] = if landing.grounded { TAKEOFFS } else { &[0.0] };
        for &offset in offsets {
            let takeoff_x = (landing.x + offset).clamp(0.0, max_x);
            if landing.grounded && !standing(level, takeoff_x, landing.y) {
                continue;
            }
            let mut start = landing.clone();
            start.x = takeoff_x;
            for policy in &programs {
                let Some(rested) = simulate(
                    level,
                    &start,
                    policy,
                    max_x,
                    death,
                    &coin_at,
                    goal_cell,
                    &mut reached,
                    &mut coin_hit,
                    &mut coins_left,
                    &mut goal,
                ) else {
                    continue;
                };
                if seen.insert(node_key(&rested)) {
                    nodes.push(rested);
                    if nodes.len() >= budget {
                        truncated = true;
                        break 'walk;
                    }
                }
            }
        }
    }

    Report {
        goal,
        coins: level.coins.len() - coins_left,
        total_coins: level.coins.len(),
        cells: reached.iter().filter(|hit| **hit).count(),
        total_cells: cells,
        nodes: nodes.len(),
        truncated,
    }
}

/// Run one policy from `start` until it lands, marking what the body covered.
/// Returns the resting player, or `None` when the arc fell out of the level.
#[allow(clippy::too_many_arguments)]
fn simulate(
    level: &LevelDefinition,
    start: &Player,
    policy: &Policy,
    max_x: f32,
    death: f32,
    coin_at: &[i32],
    goal_cell: i32,
    reached: &mut [bool],
    coin_hit: &mut [bool],
    coins_left: &mut usize,
    goal: &mut bool,
) -> Option<Player> {
    let mut player = start.clone();
    let mut dir = policy.dir;
    let jumping = policy.hold >= 0;
    let mut left_ground = !player.grounded;
    // One input, refilled per frame: building one per frame allocates (and, in
    // a debug build, dominates the search).
    let mut input = Input::scripted(false, false, false, false);
    for frame in 0..MAX_FRAMES {
        if policy.switch_at >= 0 && frame as i16 == policy.switch_at {
            dir = policy.switch_dir;
        }
        input.left = dir < 0;
        input.right = dir > 0;
        input.jump = jumping && frame as i8 <= policy.hold;
        input.jump_pressed = jumping && frame == 0;
        player.update(&input, STEP, &level.blocks, max_x);
        if player.y > death {
            return None;
        }
        cover(
            &player, level, coin_at, goal_cell, reached, coin_hit, coins_left,
            goal,
        );
        if !player.grounded {
            left_ground = true;
        } else if left_ground {
            return Some(player);
        }
    }
    // A walk on flat ground never leaves it: the arc ends where the walk got to.
    if player.grounded && !left_ground {
        return Some(player);
    }
    None
}

/// Mark every tile the body overlaps. The game checks the same overlap after
/// each frame, so this is its own collision footprint, not a guess at it.
#[allow(clippy::too_many_arguments)]
fn cover(
    player: &Player,
    level: &LevelDefinition,
    coin_at: &[i32],
    goal_cell: i32,
    reached: &mut [bool],
    coin_hit: &mut [bool],
    coins_left: &mut usize,
    goal: &mut bool,
) {
    let tile = TILE_SIZE as f32;
    let left = (player.x / tile).floor() as i32;
    let right = ((player.x + HITBOX_WIDTH - 0.01) / tile).floor() as i32;
    let top = (player.y / tile).floor() as i32;
    let bottom = ((player.y + HITBOX_HEIGHT - 0.01) / tile).floor() as i32;
    for ty in top.max(0)..=bottom {
        if ty as usize >= level.height {
            continue;
        }
        for tx in left.max(0)..=right {
            if tx as usize >= level.width {
                continue;
            }
            let cell = ty as usize * level.width + tx as usize;
            reached[cell] = true;
            let coin = coin_at[cell];
            if coin >= 0 && !coin_hit[coin as usize] {
                coin_hit[coin as usize] = true;
                *coins_left -= 1;
            }
            if cell as i32 == goal_cell {
                *goal = true;
            }
        }
    }
}

/// Whether a body at `(x, y)` is clear of tiles and standing on one.
fn standing(level: &LevelDefinition, x: f32, y: f32) -> bool {
    let tile = TILE_SIZE as f32;
    let row = (y / tile).floor() as i32;
    let left = (x / tile).floor() as i32;
    let right = ((x + HITBOX_WIDTH - 0.01) / tile).floor() as i32;
    let mut supported = false;
    for tx in left..=right {
        if solid(level, tx, row) {
            return false;
        }
        if solid(level, tx, row + 1) {
            supported = true;
        }
    }
    supported
}

fn solid(level: &LevelDefinition, tx: i32, ty: i32) -> bool {
    if tx < 0
        || ty < 0
        || tx as usize >= level.width
        || ty as usize >= level.height
    {
        return false;
    }
    level.blocks[ty as usize][tx as usize]
}

/// The deduplication key: the landing position to 2 px, its row, and the
/// horizontal speed it arrived with (bucketed). Two arcs that rest at the same
/// place with the same momentum lead to the same places.
fn node_key(player: &Player) -> u64 {
    let x = (player.x.clamp(0.0, 8191.0) / 2.0) as u64;
    let row = (player.y.clamp(0.0, 2047.0) / 16.0) as u64;
    let vx = ((player.vx + 160.0) / 40.0).clamp(0.0, 7.0) as u64;
    x | (row << 13) | (vx << 24) | ((player.grounded as u64) << 27)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The metadata every synthetic level needs; the tests vary only geometry.
    const META: &str = "name: test\nmusic: x\nbackground: y\n\
                        palette: #000000, #111111, #222222, #333333\n\n";

    /// Parse a tiny level. These stay a handful of tiles across so the search
    /// finishes instantly in a dev build; the large hand-drawn levels are
    /// validated by the release `--validate` job.
    fn report(rows: &str) -> Report {
        let level = LevelDefinition::parse(&format!("{META}{rows}"))
            .expect("the level parses");
        validate(&level)
    }

    #[test]
    fn a_walk_across_the_level_is_playable() {
        let report = report("..........\nS.C.....F.\nGGGGGGGGGG\n");
        assert!(report.playable(), "{:?}", report.failure());
        assert_eq!(report.coins, 1);
    }

    #[test]
    fn a_wall_across_the_level_is_found() {
        // Four tiles tall: taller than the jump, or the player would clear it
        // by leaving the top of the level (out of bounds is not solid).
        let report = report(
            "....G.....\n....G.....\nS...G..F..\n....G.....\nGGGGGGGGGG\n",
        );
        assert!(!report.goal, "a full-height wall seals the goal off");
        assert_eq!(report.failure().as_deref(), Some("no route to the goal"));
    }

    #[test]
    fn a_sealed_coin_is_reported_unreachable() {
        // The coin sits in a one-cell pocket with a solid tile on every side.
        // The goal stays reachable by jumping over the whole structure.
        let report =
            report("............\n....G.......\nS..GCG..F...\nGGGGGGGGGGGG\n");
        assert!(report.goal, "the goal is reachable: {:?}", report.failure());
        assert_eq!(report.coins, 0);
        assert_eq!(report.failure().as_deref(), Some("1 coin(s) unreachable"));
    }

    #[test]
    #[ignore = "runs the full search; the release --validate CI job covers it"]
    fn every_builtin_level_is_playable() {
        for (index, source) in crate::LEVELS.iter().enumerate() {
            let level =
                LevelDefinition::parse(source).expect("the level parses");
            let report = validate(&level);
            assert!(
                report.playable(),
                "level {}: {:?}",
                index + 1,
                report.failure()
            );
        }
    }

    #[test]
    fn a_level_without_a_spawn_is_an_error() {
        let error = LevelDefinition::parse(&format!("{META}GGGG\n"))
            .expect_err("no spawn");
        assert!(format!("{error:#}").contains("no spawn"), "{error:#}");
    }
}
