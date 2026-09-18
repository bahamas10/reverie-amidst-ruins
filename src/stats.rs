/*!
 * Keep stats for the entire run
 */

use std::time::Duration;

use anyhow::Result;

use crate::level::LevelDefinition;

pub struct Stats {
    pub playtime: Duration,
    pub levels_beaten: usize,
    pub coins_collected: usize,
    pub coins_total: usize,
    pub deaths: usize,
}

impl Stats {
    pub fn new(start_level: usize, levels: &[&str]) -> Result<Self> {
        // figure out how many coins there are total for all levels
        let coins_total = levels[start_level..]
            .iter()
            .map(|source| LevelDefinition::parse(source))
            .try_fold(0, |total, level| {
                level.map(|level| total + level.coins.len())
            })?;

        Ok(Self {
            playtime: Duration::ZERO,
            levels_beaten: 0,
            coins_collected: 0,
            coins_total,
            deaths: 0,
        })
    }

    pub fn print_oneline(&self) {
        print!(
            "total playtime: {} | levels beaten: {} | coins collected total: {}/{} | deaths: {}",
            self.formatted_playtime(),
            self.levels_beaten,
            self.coins_collected,
            self.coins_total,
            self.deaths,
        );
    }

    pub fn print_multiline(&self) {
        println!("total playtime: {}", self.formatted_playtime());
        println!("levels beaten: {}", self.levels_beaten);
        println!(
            "coins collected: {}/{}",
            self.coins_collected, self.coins_total
        );
        println!("deaths: {}", self.deaths);
    }

    fn formatted_playtime(&self) -> String {
        let seconds = self.playtime.as_secs();
        let hours = seconds / 60 / 60;
        let minutes = seconds / 60 % 60;
        let seconds = seconds % 60;
        let milliseconds = self.playtime.subsec_millis();

        format!("{hours}:{minutes:02}:{seconds:02}.{milliseconds:03}")
    }
}
