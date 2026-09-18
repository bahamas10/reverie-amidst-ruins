use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
    /// Disable music playback
    #[arg(long)]
    pub no_audio: bool,

    /// Level to start playing on
    #[arg(long, value_parser = clap::value_parser!(u16).range(1..), default_value_t = 1)]
    pub level: u16,
}

pub fn parse() -> Cli {
    Cli::parse()
}
