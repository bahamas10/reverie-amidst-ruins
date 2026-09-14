use anyhow::Result;

mod args;

fn main() -> Result<()> {
    let _args = args::parse();

    Ok(())
}
