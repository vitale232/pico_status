mod cli;
mod http;
mod oauth;
mod status;

use cli::{Cli, Parser};
use tracing::Level;

#[macro_use]
extern crate serde;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::fmt()
        .with_level(true)
        .with_thread_ids(true)
        .with_target(true)
        .with_max_level(Level::DEBUG)
        .with_line_number(true)
        .with_file(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Cli::parse();
    tracing::info!("CLI: {:?}", args);

    cli::run(args).await?;

    Ok(())
}
