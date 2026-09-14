use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {}

pub fn parse() -> Cli {
    Cli::parse()
}
