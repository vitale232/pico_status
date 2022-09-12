mod http;
mod oauth;
mod status;

use oauth::OAuthConfiguration;
use tokio::time::Duration;
use tracing::Level;

#[macro_use]
extern crate dotenv_codegen;
#[macro_use]
extern crate serde;

static CLIENT_ID: &str = dotenv!("CLIENT_ID");
static TENANT_ID: &str = dotenv!("TENANT_ID");
static PI_IP: &str = dotenv!("PI_IP");

const POLL_AFTER_SECS: u64 = 60;
const REFRESH_EXPIRY_PADDING_SECS: u64 = 120;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::fmt()
        .with_level(true)
        .with_thread_ids(true)
        .with_target(true)
        .with_max_level(Level::DEBUG)
        .with_line_number(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let scope = "Presence.Read Calendars.read offline_access";
    let config = OAuthConfiguration::new(CLIENT_ID, TENANT_ID, scope);

    let client = http::build_durable_client();
    let token = oauth::flow(config.clone(), &client).await?;
    token.autorefresh(
        client.clone(),
        token.clone(),
        config.clone(),
        REFRESH_EXPIRY_PADDING_SECS,
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

        match status::set_status(&client, &status, PI_IP).await {
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

        tokio::time::sleep(Duration::from_secs(POLL_AFTER_SECS)).await;
    }

    #[allow(unreachable_code)]
    Ok(())
}
