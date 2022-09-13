pub use clap::Parser;
use tokio::time::Duration;

use crate::{
    http::build_durable_client,
    oauth::{self, OAuthConfiguration},
    status,
};

#[derive(Debug, Parser)]
#[clap(version, about, long_about = None)]
pub struct Cli {
    #[clap(value_parser)]
    pi_ip: String,

    #[clap(value_parser)]
    client_id: String,

    #[clap(value_parser)]
    tenant_id: String,

    #[clap(short, long, value_parser, default_value = "60")]
    poll_after_secs: u64,

    #[clap(short, long, value_parser, default_value = "120")]
    refresh_expiry_padding_secs: u64,

    #[clap(
        short,
        long,
        value_parser,
        default_value = "Presence.Read Calendars.Read offline_access"
    )]
    scope: String,
}

#[tracing::instrument]
pub async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Received CLI Args: {:?}", cli);

    let config = OAuthConfiguration::new(&cli.client_id, &cli.tenant_id, &cli.scope);

    let client = build_durable_client();
    let token = oauth::flow(config.clone(), &client).await?;
    token.autorefresh(
        client.clone(),
        token.clone(),
        config.clone(),
        cli.refresh_expiry_padding_secs,
    );

    let err_tolerance = 5;
    let mut err_count = 0;
    loop {
        if err_count > err_tolerance {
            tracing::error!("Err number {} has occurred! This means the tolerance of {} has been surpased. Exiting!", err_count, err_tolerance);
            panic!("Err number {} has occurred! This means the tolerance of {} has been surpased. Exiting!", err_count, err_tolerance);
        }

        let status = match status::get_status(&client, &token).await {
            Ok(status) => status,
            Err(err) => {
                tracing::warn!("An error occurred while fetching the status: {:#?}", err);
                err_count += 1;
                tracing::warn!(
                    "This is the {} err occurrence. Tolerates {}.",
                    err_count,
                    err_tolerance
                );
                status::debug_status(&client, &token).await?;
                continue;
            }
        };

        match status::set_status(&client, &status, &cli.pi_ip).await {
            Ok(res) => res,
            Err(err) => {
                tracing::warn!("An error occurred while fetching the status: {:#?}", err);
                err_count += 1;
                tracing::warn!(
                    "This is the {} err occurrence. Tolerates {}.",
                    err_count,
                    err_tolerance
                );
                continue;
            }
        };

        tokio::time::sleep(Duration::from_secs(cli.poll_after_secs)).await;
    }

    #[allow(unreachable_code)]
    Ok(())
}
