#[macro_use]
extern crate dotenv_codegen;
#[macro_use]
extern crate serde;

use reqwest::Client;
use tokio::time::Duration;

mod oauth;
use oauth::OAuthConfiguration;

mod status;
use crate::status::{get_presence, set_status};

static CLIENT_ID: &str = dotenv!("CLIENT_ID");
static TENANT_ID: &str = dotenv!("TENANT_ID");
static PI_IP: &str = dotenv!("PI_IP");

const POLL_AFTER_SECS: u64 = 150;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scope = "Presence.Read Calendars.read offline_access";
    let config = OAuthConfiguration::new(CLIENT_ID, TENANT_ID, scope);

    let client = Client::new();
    let token = oauth::flow(config.clone(), &client).await?;
    oauth::use_autorefresh(token.clone(), config.clone(), 120);

    loop {
        let presence = get_presence(&client, &token).await?;
        println!("presence: {:#?}", presence);

        let pires = set_status(&client, &presence, PI_IP).await?;
        println!("Pi Response: {:#?}", pires);

        println!("Sleeping for {} seconds...", POLL_AFTER_SECS);
        tokio::time::sleep(Duration::from_secs(POLL_AFTER_SECS)).await;
    }

    #[allow(unreachable_code)]
    Ok(())
}
