mod cli;
mod http;
mod oauth;
mod status;

use cli::{Cli, Parser};
use tokio::signal;

#[macro_use]
extern crate serde;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    cli::init_tracing(&args).expect("Could not initialize tracing infrastructure!");
    tracing::info!("CLI: {:?}", args);

    let client = http::build_durable_client();
    let pico_ip = args.get_pico_ip();

    // `tokio::select!` proc macro will concurrently execute/poll the futures.
    // The first to return or error will stop the listeners and execute the
    // "callback". Based on how the error occurred, it will be handled below.
    let is_graceful_shutdown = tokio::select! {
        // An error from `cli::run` means we've exceeded the error threshold
        // and have encountered a fatal error
        err = cli::run(args, &client) => {
            tracing::error!("Fatal error: {:?}", err);
            false
        },
        // A value here means the CLI caller has attempted to cancel the program
        // so we'll print a message to the Pico and exit.
        _ = signal::ctrl_c() => {
            tracing::info!("Graceful shutdown...");
            true
        }
    };

    if is_graceful_shutdown {
        status::set_graceful_shutdown(&client, &pico_ip).await?;
    } else {
        status::set_fatal_error(&client, &pico_ip).await?;
    }

    Ok(())
}
