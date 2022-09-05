use std::fmt::Display;

use crate::{http::SharedHttpClient, oauth::SharedAccessToken};

pub async fn get_presence(
    client: &SharedHttpClient,
    token: &SharedAccessToken,
) -> Result<Presence, Box<dyn std::error::Error>> {
    let pres = client
        .get_client()
        .await
        .get("https://graph.microsoft.com/v1.0/me/presence")
        .header(
            "Authorization",
            format!("Bearer {}", token.get_access_token()),
        )
        .send()
        .await?
        .json::<Presence>()
        .await?;
    Ok(pres)
}

pub async fn set_status(
    client: &SharedHttpClient,
    presence: &Presence,
    pi_ip_addr: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let pico_url = format!(
        "http://{}/{}?top_text=Availability: {}&bottom_text=Activity: {}",
        pi_ip_addr,
        presence.screen_color(),
        presence.availability,
        presence.activity
    );
    let pires = client
        .get_client()
        .await
        .get(pico_url)
        .send()
        .await?
        .text()
        .await?;
    Ok(pires)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Presence {
    #[serde(rename = "@odata.context")]
    pub context: String,
    pub id: String,
    pub availability: Availability,
    pub activity: Activity,
}

impl Presence {
    pub fn screen_color(&self) -> String {
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
pub enum Availability {
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
pub enum Activity {
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
