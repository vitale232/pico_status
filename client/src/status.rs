use std::{collections::HashMap, fmt::Display};

use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use serde::{de, Deserialize, Deserializer};

use crate::{http::SharedHttpClient, oauth::SharedAccessToken};

pub async fn get_status(
    client: &SharedHttpClient,
    token: &SharedAccessToken,
) -> Result<Status, Box<dyn std::error::Error>> {
    // TODO: Make these simultaneouse to take advantage of the tokio runtime
    let presence = get_presence(client, token).await?;
    println!("{:?}", presence);
    let calendar = get_calendar(client, token).await?;
    println!("{:?}", calendar);

    let status = Status::new(&presence, &calendar);
    println!("{:?}", status);
    Ok(status)
}

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

pub async fn get_calendar(
    client: &SharedHttpClient,
    token: &SharedAccessToken,
) -> Result<CalendarView, Box<dyn std::error::Error>> {
    let today = Utc::today();
    let soon = today + Duration::days(7);

    let cal = client
        .get_client()
        .await
        .get(format!(
            "{}?startDateTime={}&endDateTime={}&$select=id,createdDateTime,lastModifiedDateTime,subject,start,end,attendees",
            "https://graph.microsoft.com/v1.0/me/calendarview",
            today.format("%Y-%m-%d"),
            soon.format("%Y-%m-%d")
        ))
        .header(
            "Authorization",
            format!("Bearer {}", token.get_access_token()),
        )
        .send()
        .await?
        .json::<CalendarView>()
        .await?;
    Ok(cal)
}

#[derive(Clone, Debug)]
pub struct Status {
    pub availability: Availability,
    pub activity: Activity,
    pub meeting_start: DateTime<Utc>,
    pub meeting_end: DateTime<Utc>,
    pub meeting_subject: String,
    pub meeting_attendee_count: usize,
}

impl Status {
    pub fn new(presence: &Presence, calendar: &CalendarView) -> Self {
        let next_meeting = calendar.value.first();
        Self {
            meeting_attendee_count: calendar.value.len(),
            availability: presence.availability.clone(),
            activity: presence.activity.clone(),
            meeting_start: next_meeting.map(|mtg| mtg.start).unwrap_or_default(),
            meeting_end: next_meeting.map(|mtg| mtg.end).unwrap_or_default(),
            meeting_subject: next_meeting
                .map(|mtg| mtg.subject.clone())
                .unwrap_or_default(),
        }
    }

    pub fn paint_status_uri(&self) -> String {
        format!(
            "{}?line2={}&line3={}&line5={}&line6=   {}&line7=   {} attendees",
            self.screen_color(),
            self.line2(),
            self.line3(),
            self.line5(),
            self.line6(),
            self.meeting_attendee_count,
        )
    }

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

    fn line2(&self) -> String {
        match self.availability {
            Availability::Available => "  Availability: Available".into(),
            Availability::AvailableIdle => "  Availability: Available (Idle)".into(),
            Availability::Away => "  Availability: Away from Computer".into(),
            Availability::BeRightBack => "  Availability: Be Right Back".into(),
            Availability::Busy => "  Availability: Busy".into(),
            Availability::BusyIdle => "  Availability: Busy (Idle)".into(),
            Availability::DoNotDisturb => "  Availability: Do Not Disturb".into(),
            Availability::Offline => "  Availability: Offline".into(),
            Availability::PresenceUnknown => "  Availability: Dono".into(),
        }
    }

    fn line3(&self) -> String {
        match self.activity {
            Activity::Available => "  Activity: Available".into(),
            Activity::Away => "  Activity: Away".into(),
            Activity::BeRightBack => "  Activity: Be Right Back".into(),
            Activity::Busy => "  Activity: Busy".into(),
            Activity::DoNotDisturb => "  Activity: Do Not Disturb".into(),
            Activity::InACall => "  Activity: In a Call".into(),
            Activity::InAConferenceCall => "  Activity: In a Conference Call".into(),
            Activity::Inactive => "  Activity: Inactive".into(),
            Activity::InAMeeting => "  Activity: In a Meeting".into(),
            Activity::Offline => "  Activity: Offline".into(),
            Activity::OffWork => "  Activity: Off Work!".into(),
            Activity::OutOfOffice => "  Activity: Out of Office!".into(),
            Activity::PresenceUnknown => "  Activity: Presence Unknown ??".into(),
            Activity::Presenting => "  Activity: Presenting".into(),
            Activity::UrgentInterruptionsOnly => "  Activity: Urgent Interruptions ONLY".into(),
        }
    }

    fn line5(&self) -> String {
        let now = Utc::now();
        if now > self.meeting_start && self.meeting_end < now {
            return "  Meeting goes until:".into();
        }
        "  Next Meeting:".into()
    }

    fn line6(&self) -> String {
        let start = self.meeting_start.with_timezone(&Local);
        format!("{} ({})", start.format("%I:%M %P"), self.meeting_subject)
    }
}

pub async fn set_status(
    client: &SharedHttpClient,
    status: &Status,
    pi_ip_addr: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let pico_url = format!("http://{}/{}", pi_ip_addr, status.paint_status_uri());
    println!("pico_url={}", pico_url);
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

#[derive(Clone, Debug, Deserialize)]
pub struct Presence {
    pub id: String,
    pub availability: Availability,
    pub activity: Activity,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GraphDateTimeZone {
    #[serde(rename = "dateTime")]
    pub datetime: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Attendee {
    #[serde(rename = "type")]
    pub type_: String,
}

fn deser_msgraph_datetimezone_utc<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let map_datetime_obj: HashMap<String, String> = Deserialize::deserialize(deserializer)?;
    let datetime = match map_datetime_obj.get("dateTime") {
        Some(dt) => dt,
        // I have no idea what this will do at runtime.
        None => "",
    };
    NaiveDateTime::parse_from_str(datetime, "%Y-%m-%dT%H:%M:%S%.f")
        .map_err(de::Error::custom)
        .map(|val| DateTime::<Utc>::from_utc(val, Utc))
}

#[derive(Clone, Debug, Deserialize)]
pub struct CalendarView {
    #[serde(rename = "@odata.context")]
    pub context: String,
    pub value: Vec<Event>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Event {
    #[serde(rename = "@odata.etag")]
    pub context: String,
    pub id: String,
    #[serde(rename = "createdDateTime")]
    pub created_datetime: DateTime<Utc>,
    #[serde(rename = "lastModifiedDateTime")]
    pub last_modified_datetime: DateTime<Utc>,
    pub subject: String,
    #[serde(deserialize_with = "deser_msgraph_datetimezone_utc")]
    pub start: DateTime<Utc>,
    #[serde(deserialize_with = "deser_msgraph_datetimezone_utc")]
    pub end: DateTime<Utc>,
    pub attendees: Vec<Attendee>,
}

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Deserialize)]
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
