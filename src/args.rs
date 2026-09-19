use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
    /// Disable audio playback completely
    #[arg(long)]
    pub no_audio: bool,

    /// Master audio volume from 0 to 100
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=100), default_value_t = 100)]
    pub volume: u8,

    /// Level to start playing on
    #[arg(long, value_parser = clap::value_parser!(u16).range(1..), default_value_t = 1)]
    pub level: u16,

    /// FPS to target
    #[arg(long, value_parser = clap::value_parser!(u16).range(1..), default_value_t = 60)]
    pub fps: u16,
}

pub fn parse() -> Cli {
    Cli::parse()
}
