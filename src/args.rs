use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    long_about = styled_text(include_str!("../assets/help.txt"))

)]
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

pub fn print_help() {
    print!("{}", styled_text(include_str!("../assets/help.txt")));
}

fn styled_text(text: &str) -> String {
    let template = &[
        ("reset", "\x1b[0m"),
        ("bold", "\x1b[1m"),
        ("dim", "\x1b[2m"),
        ("c1", "\x1b[31m"),
        ("c2", "\x1b[32m"),
        ("c3", "\x1b[33m"),
        ("c4", "\x1b[34m"),
        ("c5", "\x1b[35m"),
        ("c6", "\x1b[36m"),
        ("c7", "\x1b[37m"),
        ("c8", "\x1b[38m"),
    ];

    let mut out = text.to_string();
    for (k, v) in template {
        let k = format!("$({})", k);
        out = out.replace(&k, v);
    }

    out
}
