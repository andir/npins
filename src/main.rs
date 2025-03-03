use anyhow::Result;
use clap::Parser;

pub mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    let opts = cli::Opts::parse();

    env_logger::builder()
        .filter_level(if opts.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .format_timestamp(None)
        .format_target(false)
        .init();

    opts.run().await?;
    Ok(())
}
