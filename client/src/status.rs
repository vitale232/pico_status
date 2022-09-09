use std::{collections::HashMap, fmt::Display};

use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use serde::{de, Deserialize, Deserializer};

use crate::http::DurableClient;
use crate::oauth::SharedAccessToken;

pub async fn get_status(
    client: &DurableClient,
    token: &SharedAccessToken,
) -> Result<Status, Box<dyn std::error::Error>> {
    // TODO: Make these simultaneouse to take advantage of the tokio runtime
    let presence = get_presence(client, token).await?;
    let calendar = get_calendar(client, token).await?;

    let status = Status::new(&presence, &calendar);
    Ok(status)
}

pub async fn debug_status(
    client: &DurableClient,
    token: &SharedAccessToken,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Make these simultaneouse to take advantage of the tokio runtime
    let presence = debug_presence(client, token).await?;
    println!("{:?}", presence);
    let calendar = debug_calendar(client, token).await?;
    println!("{:?}", calendar);

    Ok(())
}

pub async fn set_status(
    client: &DurableClient,
    status: &Status,
    pi_ip_addr: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let pico_url = format!("http://{}/{}", pi_ip_addr, status.uri());
    println!("pico_url={}", pico_url);
    let pires = client.get(pico_url).send().await?.text().await?;
    Ok(pires)
}

pub async fn get_presence(
    client: &DurableClient,
    token: &SharedAccessToken,
) -> Result<Presence, Box<dyn std::error::Error>> {
    let pres = client
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

pub async fn debug_presence(
    client: &DurableClient,
    token: &SharedAccessToken,
) -> Result<String, Box<dyn std::error::Error>> {
    let pres = client
        .get("https://graph.microsoft.com/v1.0/me/presence")
        .header(
            "Authorization",
            format!("Bearer {}", token.get_access_token()),
        )
        .send()
        .await?
        .text()
        .await?;
    Ok(pres)
}

pub async fn debug_calendar(
    client: &DurableClient,
    token: &SharedAccessToken,
) -> Result<String, Box<dyn std::error::Error>> {
    let today = Utc::now();
    let soon = today + Duration::days(7);
    let cal_url = format!(
        "{}?startDateTime={}&endDateTime={}&$select={}&$orderby={}",
        "https://graph.microsoft.com/v1.0/me/calendarview",
        today.format("%Y-%m-%dT%H:%M:%S"),
        soon.format("%Y-%m-%d"),
        "id,createdDateTime,lastModifiedDateTime,subject,start,end,attendees",
        "start/dateTime"
    );
    println!("{:#?}", cal_url);
    let cal = client
        .get(cal_url)
        .header(
            "Authorization",
            format!("Bearer {}", token.get_access_token()),
        )
        .send()
        .await?
        .text()
        .await?;
    Ok(cal)
}

pub async fn get_calendar(
    client: &DurableClient,
    token: &SharedAccessToken,
) -> Result<CalendarView, Box<dyn std::error::Error>> {
    let today = Utc::now();
    let soon = today + Duration::days(7);
    let cal_url = format!(
        "{}?startDateTime={}&endDateTime={}&$select={}&$orderby={}",
        "https://graph.microsoft.com/v1.0/me/calendarview",
        today.format("%Y-%m-%dT%H:%M:%S"),
        soon.format("%Y-%m-%d"),
        "id,createdDateTime,lastModifiedDateTime,subject,start,end,attendees",
        "start/dateTime"
    );
    println!("{:#?}", cal_url);
    let cal = client
        .get(cal_url)
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
    availability: Availability,
    activity: Activity,
    event_start: DateTime<Utc>,
    event_end: DateTime<Utc>,
    event_subject: String,
    event_attendee_count: usize,
}

impl Status {
    pub fn new(presence: &Presence, calendar: &CalendarView) -> Self {
        // This assumes that the CalendarView is ordered by start/dateTime
        let next_meeting = calendar.value.iter().find(|e| e.end > Utc::now());
        Self {
            event_attendee_count: next_meeting
                .map(|mtg| mtg.attendees.len())
                .unwrap_or_default(),
            availability: presence.availability.clone(),
            activity: presence.activity.clone(),
            event_start: next_meeting.map(|mtg| mtg.start).unwrap_or_default(),
            event_end: next_meeting.map(|mtg| mtg.end).unwrap_or_default(),
            event_subject: next_meeting
                .map(|mtg| mtg.subject.clone())
                .unwrap_or_default(),
        }
    }

    pub fn uri(&self) -> String {
        format!(
            "{}?line1={}&line2={}&line3={}&line5={}&line6={}&line7={}",
            self.screen_color(),
            self.line1(),
            self.line2(),
            self.line3(),
            self.line5(),
            self.line6(),
            self.line7(),
        )
    }

    pub fn is_in_meeting(&self) -> bool {
        let now = Utc::now();
        now > self.event_start && now < self.event_end
    }

    pub fn screen_color(&self) -> String {
        match self.availability {
            // Green
            Availability::Available => "green".into(),
            // Yeller
            Availability::AvailableIdle => "yellow".into(),
            Availability::Away => "yellow".into(),
            Availability::BeRightBack => "yellow".into(),
            Availability::Offline => "yellow".into(),
            Availability::PresenceUnknown => "yellow".into(),
            // Red
            Availability::Busy => "red".into(),
            Availability::BusyIdle => "red".into(),
            Availability::DoNotDisturb => "red".into(),
        }
    }

    fn line1(&self) -> String {
        format!("{:>28}", Local::now().format("%I:%M %P"))
    }

    fn line2(&self) -> String {
        let value = match self.availability {
            Availability::Available => "Available",
            Availability::AvailableIdle => "Available (Idle)",
            Availability::Away => "Away from Computer",
            Availability::BeRightBack => "Be Right Back",
            Availability::Busy => "Busy",
            Availability::BusyIdle => "Busy (Idle)",
            Availability::DoNotDisturb => "Do Not Disturb",
            Availability::Offline => "Offline",
            Availability::PresenceUnknown => "Dono",
        };
        format!(" {}", value)
    }

    fn line3(&self) -> String {
        let value = match self.activity {
            Activity::Available => "(Available)",
            Activity::Away => "(Away)",
            Activity::BeRightBack => "(Be Right Back)",
            Activity::Busy => "(Busy)",
            Activity::DoNotDisturb => "(Do Not Disturb)",
            Activity::InACall => "(In a Call)",
            Activity::InAConferenceCall => "(In a Conference Call)",
            Activity::Inactive => "(Inactive)",
            Activity::InAMeeting => "(In a Meeting)",
            Activity::Offline => "(Offline)",
            Activity::OffWork => "(Off Work!)",
            Activity::OutOfOffice => "(Out of Office!)",
            Activity::PresenceUnknown => "(Presence Unknown ??)",
            Activity::Presenting => "(Presenting)",
            Activity::UrgentInterruptionsOnly => "(Urgent Interruptions ONLY)",
        };
        format!(" {}", value)
    }

    fn line5(&self) -> String {
        let value: String = if self.is_in_meeting() {
            "Meeting goes until:".into()
        } else {
            format!("Next Event ({}):", self.event_start.format("%m/%d"))
        };
        format!(" {}", value)
    }

    fn line6(&self) -> String {
        let time = match self.is_in_meeting() {
            true => self.event_end,
            false => self.event_start,
        };
        format!(
            "  {} ({})",
            time.with_timezone(&Local).format("%I:%M %P"),
            self.event_subject
        )
    }

    fn line7(&self) -> String {
        format!("  {} attendees", self.event_attendee_count)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Presence {
    pub id: String,
    pub availability: Availability,
    pub activity: Activity,
}

#[derive(Clone, Debug, Deserialize)]
struct Attendee {
    #[serde(rename = "type")]
    pub _type: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CalendarView {
    pub value: Vec<Event>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Event {
    subject: String,
    #[serde(deserialize_with = "deser_msgraph_datetimezone_utc")]
    start: DateTime<Utc>,
    #[serde(deserialize_with = "deser_msgraph_datetimezone_utc")]
    end: DateTime<Utc>,
    attendees: Vec<Attendee>,
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
