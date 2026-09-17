use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
    /// Width of the game in terminal columns
    #[arg(long, value_parser = clap::value_parser!(u16).range(1..))]
    pub width: Option<u16>,

    /// Disable music playback
    #[arg(long)]
    pub no_audio: bool,
}

pub fn parse() -> Cli {
    Cli::parse()
}
