use clap::Parser;
use pvox::config::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let config = args.load()?;
    pvox::run(config).await
}
