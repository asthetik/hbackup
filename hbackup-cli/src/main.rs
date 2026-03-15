use clap::Parser;

mod args;
mod commands;
mod constants;

pub type Result<T> = anyhow::Result<T>;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = args::Cli::parse();
    cli.command.execute().await?;
    Ok(())
}
