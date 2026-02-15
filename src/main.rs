mod cli;
mod commands;
mod near;
mod utils;
mod bot;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let cli = Cli::parse();

    if let Err(e) = commands::run(cli).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
