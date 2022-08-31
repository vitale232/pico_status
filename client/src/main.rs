#[macro_use]
extern crate dotenv_codegen;
#[macro_use]
extern crate serde;

use std::{
    fmt::Display,
    sync::{Arc, Mutex},
};

use lib::oauth::{self, Config};
use reqwest::Client;
use tokio::time::Duration;

static CLIENT_ID: &str = dotenv!("CLIENT_ID");
static CLIENT_SECRET: &str = dotenv!("CLIENT_SECRET");
static TENANT_ID: &str = dotenv!("TENANT_ID");
static PI_IP: &str = dotenv!("PI_IP");

const POLL_AFTER_SECS: u64 = 150;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Presence {
    #[serde(rename = "@odata.context")]
    pub context: String,
    pub id: String,
    pub availability: Availability,
    pub activity: Activity,
}

impl Presence {
    fn screen_color(&self) -> String {
        match self.availability {
            Availability::Available => "green".into(),
            Availability::AvailableIdle => "yellow".into(),
            Availability::Away => "yellow".into(),
            Availability::BeRightBack => "yellow".into(),
            Availability::Busy => "red".into(),
            Availability::BusyIdle => "red".into(),
            Availability::DoNotDisturb => "red".into(),
            Availability::Offline => "yellow".into(),
            Availability::PresenceUnknown => "yellow".into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
enum Availability {
    Available,
    AvailableIdle,
    Away,
    BeRightBack,
    Busy,
    BusyIdle,
    DoNotDisturb,
    Offline,
    PresenceUnknown,
}

impl Display for Availability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            Availability::Available => "Available",
            Availability::AvailableIdle => "AvailableIdle",
            Availability::Away => "Away",
            Availability::BeRightBack => "BeRightBack",
            Availability::Busy => "Busy",
            Availability::BusyIdle => "BusyIdle",
            Availability::DoNotDisturb => "DoNotDisturb",
            Availability::Offline => "Offline",
            Availability::PresenceUnknown => "PresenceUnknown",
        };
        write!(f, "{}", val)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
enum Activity {
    Available,
    Away,
    BeRightBack,
    Busy,
    DoNotDisturb,
    InACall,
    InAConferenceCall,
    Inactive,
    InAMeeting,
    Offline,
    OffWork,
    OutOfOffice,
    PresenceUnknown,
    Presenting,
    UrgentInterruptionsOnly,
}

impl Display for Activity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            Activity::Available => "Available",
            Activity::Away => "Away",
            Activity::BeRightBack => "BeRightBack",
            Activity::Busy => "Busy",
            Activity::DoNotDisturb => "DotNotDisturb",
            Activity::InACall => "InACall",
            Activity::InAConferenceCall => "InAConferenceCall",
            Activity::Inactive => "Inactive",
            Activity::InAMeeting => "InAMeeting",
            Activity::Offline => "Offline",
            Activity::OffWork => "OffWork",
            Activity::OutOfOffice => "OutOfOffice",
            Activity::PresenceUnknown => "PresenceUnknown",
            Activity::Presenting => "Presenting",
            Activity::UrgentInterruptionsOnly => "UrgentInterruptionsOnly",
        };
        write!(f, "{}", val)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(Mutex::new(Config::new(CLIENT_ID, CLIENT_SECRET, TENANT_ID)));
    let scope = "Presence.Read Calendars.read offline_access";

    let client = Client::new();
    let token = oauth::flow(config, scope, &client).await?;
    println!("{:#?}", token);

    loop {
        let presence = client
            .get("https://graph.microsoft.com/v1.0/me/presence")
            .header("Authorization", format!("Bearer {}", token.access_token))
            .send()
            .await?
            .json::<Presence>()
            .await?;
        println!("presence: {:#?}", presence);

        let pico_url = format!(
            "http://{}/{}?top_text=Availability: {}&bottom_text=Activity: {}",
            PI_IP,
            presence.screen_color(),
            presence.availability,
            presence.activity
        );
        let pires = client.get(pico_url).send().await?.text().await?;

        println!("Pi Response: {:#?}", pires);
        println!("Sleeping for {} seconds...", POLL_AFTER_SECS);

        tokio::time::sleep(Duration::from_secs(POLL_AFTER_SECS)).await;
    }

    #[allow(unreachable_code)]
    Ok(())
}
