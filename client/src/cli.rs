use crate::{
    http::DurableClient,
    oauth::{self, OAuthConfiguration, SharedAccessToken},
    status,
};
pub use clap::Parser;
use tokio::time::Duration;
use tracing::Level;

pub fn init_tracing(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let log_level = if cli.is_verbose() {
        Level::TRACE
    } else {
        Level::INFO
    };
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_file(true)
        .with_target(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

#[derive(Debug, Parser)]
#[clap(version, about, long_about = None)]
pub struct Cli {
    #[clap(value_parser)]
    pico_ip: String,

    #[clap(value_parser)]
    client_id: String,

    #[clap(value_parser, default_value = "common")]
    tenant_id: String,

    #[clap(short, long, value_parser, default_value = "3")]
    auth_wait_for: u64,

    #[clap(short, long, value_parser, default_value = "60")]
    poll_after: u64,

    #[clap(short, long, value_parser, default_value = "120")]
    refresh_expiry_padding: u64,

    #[clap(
        short,
        long,
        value_parser,
        default_value = "Presence.Read Calendars.Read offline_access"
    )]
    scope: String,

    #[clap(short, long, action)]
    verbose: bool,
}

impl Cli {
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    pub fn get_pico_ip(&self) -> String {
        String::from(&self.pico_ip)
    }
}

#[tracing::instrument]
pub async fn run(cli: Cli, client: &DurableClient) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Received CLI Args: {:?}", cli);

    let config = OAuthConfiguration::new(&cli.client_id, &cli.tenant_id, &cli.scope);
    let token = oauth::flow(config.clone(), client, cli.auth_wait_for).await?;
    SharedAccessToken::autorefresh(
        token.clone(),
        client.clone(),
        config.clone(),
        cli.refresh_expiry_padding,
    );

    let err_tolerance = 5;
    let mut err_count = 0;
    loop {
        let status = match status::get_status(client, &token).await {
            Ok(status) => status,
            Err(err) => {
                tracing::warn!("An error occurred while fetching the status: {:#?}", err);
                err_count += 1;
                tracing::warn!(
                    "This is the {} err occurrence. Tolerates {}.",
                    err_count,
                    err_tolerance
                );
                status::debug_status(client, &token).await.unwrap_or(());
                if err_count > err_tolerance {
                    tracing::error!("Err number {} has occurred! This means the tolerance of {} has been surpased. Exiting!", err_count, err_tolerance);
                    return Err(err);
                } else {
                    continue;
                }
            }
        };

        match status::set_status(client, &status, &cli.pico_ip).await {
            Ok(res) => res,
            Err(err) => {
                tracing::warn!("An error occurred while fetching the status: {:#?}", err);
                err_count += 1;
                tracing::warn!(
                    "This is the {} err occurrence. Tolerates {}.",
                    err_count,
                    err_tolerance
                );
                if err_count > err_tolerance {
                    tracing::error!("Err number {} has occurred! This means the tolerance of {} has been surpased. Exiting!", err_count, err_tolerance);
                    return Err(err);
                } else {
                    continue;
                }
            }
        };

        tokio::time::sleep(Duration::from_secs(cli.poll_after)).await;
    }

    #[allow(unreachable_code)]
    Ok(())
}
