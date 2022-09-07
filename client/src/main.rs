#[macro_use]
extern crate dotenv_codegen;
#[macro_use]
extern crate serde;

use tokio::time::Duration;

mod http;

mod oauth;
use oauth::OAuthConfiguration;

mod status;
use crate::status::{get_status, set_status};

static CLIENT_ID: &str = dotenv!("CLIENT_ID");
static TENANT_ID: &str = dotenv!("TENANT_ID");
static PI_IP: &str = dotenv!("PI_IP");

const POLL_AFTER_SECS: u64 = 60;
const REFRESH_EXPIRY_PADDING_SECS: u64 = 120;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
            panic!("Err number {} has occurred! This means the tolerance of {} has been surpased. Exiting!", err_count, err_tolerance);
        }

        let status = match get_status(&client, &token).await {
            Ok(status) => status,
            Err(err) => {
                println!("An error occurred while fetching the status: {:#?}", err);
                err_count += 1;
                println!(
                    "This is the {} err occurrence. Tolerates {}.",
                    err_count, err_tolerance
                );
                continue;
            }
        };

        let pires = match set_status(&client, &status, PI_IP).await {
            Ok(res) => res,
            Err(err) => {
                println!("An error occurred while fetching the status: {:#?}", err);
                err_count += 1;
                println!(
                    "This is the {} err occurrence. Tolerates {}.",
                    err_count, err_tolerance
                );
                continue;
            }
        };
        println!("Pi Response: {:#?}", pires);

        println!("Sleeping for {} seconds...", POLL_AFTER_SECS);
        tokio::time::sleep(Duration::from_secs(POLL_AFTER_SECS)).await;
    }

    #[allow(unreachable_code)]
    Ok(())
}
