mod cmd;
mod format;
mod fputil;
mod statement;
mod timer;
mod translate;
mod traverse;

use crate::cmd::convert;
use crate::cmd::convert::ConvertArgs;
use crate::timer::Timer;
use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    /// Adds files to myapp
    Convert(ConvertArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let _t = Timer::new("总");
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Convert(args) => convert::exec(args).await?,
    }
    Ok(())
}
