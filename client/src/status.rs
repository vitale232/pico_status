use std::collections::HashMap;

use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use serde::{de, Deserialize, Deserializer};

use crate::http::DurableClient;
use crate::oauth::SharedAccessToken;

#[tracing::instrument]
pub async fn get_status(
    client: &DurableClient,
    token: &SharedAccessToken,
) -> Result<Status, Box<dyn std::error::Error>> {
    let (pres_result, cal_result) =
        tokio::join!(get_presence(client, token), get_calendar(client, token));

    let presence = match pres_result {
        Ok(pres) => pres,
        Err(err) => return Err(err),
    };
    let calendar = match cal_result {
        Ok(cal) => cal,
        Err(err) => return Err(err),
    };

    let status = Status::new(&presence, &calendar);
    tracing::info!("Status: {:#?}", status);
    Ok(status)
}

#[tracing::instrument]
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
    tracing::info!("Calendar URL: {:#?}", cal_url);
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
    tracing::trace!("Calendar response: {:?}", cal);
    Ok(cal)
}

#[tracing::instrument]
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
    tracing::trace!("Presence Response: {:#?}", pres);
    Ok(pres)
}

#[tracing::instrument]
pub async fn set_status(
    client: &DurableClient,
    status: &Status,
    pi_ip_addr: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let pico_url = format!("http://{}/{}", pi_ip_addr, status.uri());
    tracing::info!("Pi URL {:#?}", pico_url);
    let pires = client.get(pico_url).send().await?.text().await?;
    tracing::info!("Pi Response {:#?}", pires);
    Ok(pires)
}

#[tracing::instrument]
pub async fn set_graceful_shutdown(
    client: &DurableClient,
    pi_ip_addr: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!(
        "http://{}/yellow?line3=  Good bye&line4=    for now...",
        pi_ip_addr
    );
    tracing::info!("Graceful shutdown URL: {:#?}", url);
    let pires = client.get(url).send().await?.text().await?;
    tracing::info!("Pi Response {:#?}", pires);
    Ok(pires)
}

#[tracing::instrument]
pub async fn set_fatal_error(
    client: &DurableClient,
    pi_ip_addr: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!(
        "http://{}/late?line2= FATAL ERROR!&line3=   FATAL ERROR!&line5=  We can't go on",
        pi_ip_addr
    );
    tracing::info!("Graceless shutdown URL: {:#?}", url);
    let pires = client.get(url).send().await?.text().await?;
    tracing::info!("Pi Response {:#?}", pires);
    Ok(pires)
}
#[tracing::instrument]
pub async fn debug_status(
    client: &DurableClient,
    token: &SharedAccessToken,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::trace!("Debugging status GETs");
    let presence = debug_presence(client, token).await?;
    tracing::trace!("Presence: {:?}", presence);
    let calendar = debug_calendar(client, token).await?;
    tracing::trace!("Calendar: {:?}", calendar);
    Ok(())
}

#[tracing::instrument]
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
    tracing::info!("Presence as text: {:#?}", pres);
    Ok(pres)
}

#[tracing::instrument]
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
    tracing::trace!("Calendar URL: {:?}", cal_url);
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
    tracing::info!("Calendar as text: {:#?}", cal);
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
        let next_event = calendar.value.iter().find(|evt| evt.end > Utc::now());
        Self {
            event_attendee_count: next_event
                .map(|mtg| mtg.attendees.len())
                .unwrap_or_default(),
            availability: presence.availability.clone(),
            activity: presence.activity.clone(),
            event_start: next_event.map(|mtg| mtg.start).unwrap_or_default(),
            event_end: next_event.map(|mtg| mtg.end).unwrap_or_default(),
            event_subject: next_event
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

    pub fn is_busy(&self) -> bool {
        let now = Utc::now();
        now > self.event_start && now < self.event_end
    }

    pub fn is_late(&self) -> bool {
        if !self.is_busy() {
            return false;
        }
        matches!(self.availability, Availability::Away)
    }

    pub fn screen_color(&self) -> String {
        let color = match self.availability {
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
        };
        if self.is_late() {
            "late".into()
        } else {
            color
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
        let value: String = if self.is_busy() {
            "Event goes until:".into()
        } else {
            format!("Next Event ({}):", self.event_start.format("%m/%d"))
        };
        format!(" {}", value)
    }

    fn line6(&self) -> String {
        let time = match self.is_busy() {
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

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;

    #[test]
    fn test_uri_availabile_future_event() {
        let presence = build_presence(Availability::Available, Activity::Available);
        let (future_event, next_start, _) = build_future_cal_event("Test One");
        let calendar = CalendarView {
            value: vec![future_event.clone()],
        };

        let status = Status::new(&presence, &calendar);
        println!("{:?}", status.uri());

        assert!(!status.is_busy());
        assert_eq!(
            status.uri(),
            format!(
                "{}?line1={:>28}&line2= {}&line3= ({})&line5= Next Event ({}):&line6=  {} ({})&line7=  {} attendees",
                "green",
                Local::now().format("%I:%M %P"),
                "Available",
                "Available",
                next_start.format("%m/%d"),
                next_start.format("%I:%M %P"),
                future_event.subject,
                future_event.attendees.len()
           )
        );
    }

    #[test]
    fn test_uri_busy_curr_event() {
        let presence = build_presence(Availability::Busy, Activity::InACall);
        let (event, _, end) = build_current_cal_event("Current Events");
        let cal = CalendarView {
            value: vec![event.clone()],
        };

        let status = Status::new(&presence, &cal);
        println!("{:?}", status.uri());
        assert!(status.is_busy());

        assert_eq!(
            status.uri(),
            format!(
                "{}?line1={:>28}&line2= {}&line3= ({})&line5= Event goes until:&line6=  {} ({})&line7=  {} attendees",
                "red",
                Local::now().format("%I:%M %P"),
                "Busy",
                "In a Call",
                end.format("%I:%M %P"),
                event.subject,
                event.attendees.len()
           )
        );
    }

    #[test]
    fn test_uri_late_for_event() {
        let presence = build_presence(Availability::Away, Activity::Away);
        let (event, _, end) = build_current_cal_event("Current Events");
        let cal = CalendarView {
            value: vec![event.clone()],
        };

        let status = Status::new(&presence, &cal);
        println!("{:?}", status.uri());
        assert!(status.is_busy());
        assert!(status.is_late());

        assert_eq!(
            status.uri(),
            format!(
                "{}?line1={:>28}&line2= {}&line3= ({})&line5= Event goes until:&line6=  {} ({})&line7=  {} attendees",
                "late",
                Local::now().format("%I:%M %P"),
                "Away from Computer",
                "Away",
                end.format("%I:%M %P"),
                event.subject,
                event.attendees.len()
           )
        );
    }

    fn build_presence(availability: Availability, activity: Activity) -> Presence {
        Presence {
            id: String::from("id123"),
            availability,
            activity,
        }
    }

    fn build_future_cal_event(subject: &str) -> (Event, DateTime<Local>, DateTime<Local>) {
        let now = Utc::now();
        let start = now + Duration::hours(1);
        let end = start + Duration::hours(1);
        (
            Event {
                subject: subject.into(),
                start,
                end,
                attendees: vec![Attendee {
                    _type: String::from("who cares"),
                }],
            },
            DateTime::from(start),
            DateTime::from(end),
        )
    }

    fn build_current_cal_event(subject: &str) -> (Event, DateTime<Local>, DateTime<Local>) {
        let now = Utc::now();
        let start = now - Duration::minutes(10);
        let end = start + Duration::hours(1);
        (
            Event {
                subject: subject.into(),
                start,
                end,
                attendees: vec![Attendee {
                    _type: String::from("who cares"),
                }],
            },
            DateTime::from(start),
            DateTime::from(end),
        )
    }
}
