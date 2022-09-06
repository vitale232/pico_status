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

    loop {
        let status = get_status(&client, &token).await?;

        let pires = set_status(&client, &status, PI_IP).await?;
        println!("Pi Response: {:#?}", pires);

        println!("Sleeping for {} seconds...", POLL_AFTER_SECS);
        tokio::time::sleep(Duration::from_secs(POLL_AFTER_SECS)).await;
    }

    #[allow(unreachable_code)]
    Ok(())
}
