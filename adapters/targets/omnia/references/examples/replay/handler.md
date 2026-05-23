# Handler Example: r9k-adapter

This is an example of a handler in the expected format after Specify-driven generation. The corresponding [tests](tests.md) and [fixtures](fixtures.md) examples correspond to this handler.

## Crate entry point (lib.rs)

```rust
//! # R9K Transformer
//!
//! Transforms R9K messages into SmarTrak events.

mod handler;
mod r9k;
mod smartrak;
mod stops;

use omnia_sdk::Error;
use thiserror::Error;

pub use self::handler::*;
pub use self::r9k::*;
pub use self::smartrak::*;
pub use self::stops::StopInfo;

// TODO: use for internal methods
/// Errors that can occur while transforming R9K messages.
#[derive(Error, Debug)]
pub enum R9kError {
    /// The message timestamp is invalid (too old or future-dated).
    #[error("{0}")]
    BadTime(String),

    /// The message contains no updates or the arrival/departure time is
    /// invalid (negative or 0).
    #[error("{0}")]
    NoUpdate(String),

    /// The XML is invalid.
    #[error("{0}")]
    InvalidXml(String),
}

impl R9kError {
    fn code(&self) -> String {
        match self {
            Self::BadTime(_) => "bad_time".to_string(),
            Self::NoUpdate(_) => "no_update".to_string(),
            Self::InvalidXml(_) => "invalid_message".to_string(),
        }
    }
}

impl From<R9kError> for Error {
    fn from(err: R9kError) -> Self {
        Self::BadRequest { code: err.code(), description: err.to_string() }
    }
}

impl From<quick_xml::DeError> for R9kError {
    fn from(err: quick_xml::DeError) -> Self {
        Self::InvalidXml(err.to_string())
    }
}
```

## Handler (handler.rs)

```rust
//! R9K Position Adapter
//!
//! Transform an R9K XML message into a SmarTrak[`TrainUpdate`].

use anyhow::Context as _;
use bytes::Bytes;
use chrono::Utc;
use http::header::AUTHORIZATION;
use http_body_util::Empty;
use omnia_sdk::api::{Context, Handler, Reply};
use omnia_sdk::{Config, Error, HttpRequest, Identity, Message, Publisher, Result};
use serde::Deserialize;

use crate::r9k::TrainUpdate;
use crate::smartrak::{EventType, MessageData, RemoteData, SmarTrakEvent};
use crate::stops;

const SMARTRAK_TOPIC: &str = "realtime-r9k-to-smartrak.v1";

/// R9K train update message as deserialized from the XML received from
/// KiwiRail.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct R9kMessage {
    /// The train update.
    #[serde(rename(deserialize = "ActualizarDatosTren"))]
    pub train_update: TrainUpdate,
}

async fn handle<P>(owner: &str, request: R9kMessage, provider: &P) -> Result<Reply<()>>
where
    P: Config + HttpRequest + Identity + Publisher,
{
    // validate message
    let update = request.train_update;
    update.validate()?;

    // convert to SmarTrak events
    let events = update.into_events(owner, provider).await?;

    // publish events to SmarTrak topic
    // publish 2x in order to properly signal departure from the station
    // (for schedule adherence)
    let env = Config::get(provider, "ENV").await.unwrap_or_else(|_| "dev".to_string());
    let topic = format!("{env}-{SMARTRAK_TOPIC}");

    for _ in 0..2 {
        #[cfg(not(debug_assertions))]
        std::thread::sleep(std::time::Duration::from_secs(5));

        for event in &events {
            tracing::info!(monotonic_counter.smartrak_events_published = 1);

            let payload = serde_json::to_vec(&event).context("serializing event")?;
            let external_id = &event.remote_data.external_id;

            let mut message = Message::new(&payload);
            message.headers.insert("key".to_string(), external_id.clone());

            Publisher::send(provider, &topic, &message).await?;
        }
    }

    Ok(Reply::ok(()))
}

impl<P> Handler<P> for R9kMessage
where
    P: Config + HttpRequest + Identity + Publisher,
{
    type Error = Error;
    type Input = Vec<u8>;
    type Output = ();

    fn from_input(input: Vec<u8>) -> Result<Self> {
        quick_xml::de::from_reader(input.as_ref())
            .context("deserializing R9kMessage")
            .map_err(Into::into)
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<()>> {
        handle(ctx.owner, self, ctx.provider).await
    }
}

impl TrainUpdate {
    /// Transform the R9K message to SmarTrak events
    async fn into_events<P>(self, owner: &str, provider: &P) -> Result<Vec<SmarTrakEvent>>
    where
        P: Config + HttpRequest + Identity + Publisher,
    {
        let changes = &self.changes;
        let change_type = changes[0].r#type;

        // filter out irrelevant updates (not related to trip progress)
        if !change_type.is_relevant() {
            // TODO: do we need this metric?
            tracing::info!(monotonic_counter.irrelevant_change_type = 1, type = %change_type);
            return Ok(vec![]);
        }

        // is station is relevant?
        let station = changes[0].station;
        let Some(stop_info) =
            stops::stop_info(owner, provider, station, change_type.is_arrival()).await?
        else {
            tracing::info!(monotonic_counter.irrelevant_station = 1, station = %station);
            return Ok(vec![]);
        };

        // get train allocations for this trip
        let url = Config::get(provider, "BLOCK_MGT_URL").await?;
        let identity = Config::get(provider, "AZURE_IDENTITY").await?;

        let token = Identity::access_token(provider, identity).await?;

        let request = http::Request::builder()
            .uri(format!("{url}/allocations/trips?externalRefId={}", self.train_id()))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Empty::<Bytes>::new())
            .context("building block management request")?;
        let response =
            HttpRequest::fetch(provider, request).await.context("fetching train allocations")?;

        let bytes = response.into_body();
        let allocated: Vec<String> =
            serde_json::from_slice(&bytes).context("deserializing block management response")?;

        // publish `SmarTrak` events
        let mut events = Vec::new();
        for train in allocated {
            events.push(SmarTrakEvent {
                received_at: Utc::now(),
                event_type: EventType::Location,
                message_data: MessageData::default(),
                remote_data: RemoteData {
                    external_id: train.replace(' ', ""),
                    ..RemoteData::default()
                },
                location_data: stop_info.clone().into(),
                ..SmarTrakEvent::default()
            });
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::R9kMessage;

    #[test]
    fn deserialization() {
        let xml = include_str!("../data/sample.xml");
        let message: R9kMessage = quick_xml::de::from_str(xml).expect("should deserialize");

        let update = message.train_update;
        assert_eq!(update.even_train_id, Some("1234".to_string()));
        assert!(!update.changes.is_empty(), "should have changes");
    }
}
```

## Supplementary module (r9k.rs)

```rust
//! R9K data types

use std::fmt::{Display, Formatter};

use chrono::{NaiveDate, Utc};
use chrono_tz::Pacific;
use omnia_sdk::Result;
use serde::Deserialize;
use serde_repr::Deserialize_repr;

use crate::R9kError;

const MAX_DELAY_SECS: i64 = 60;
const MIN_DELAY_SECS: i64 = -30;

/// R9000 (R9K) train update as received from KiwiRail.
/// Defines the XML mappings as defined by the R9K provider - in Spanish.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TrainUpdate {
    /// Train ID for even trains.
    #[serde(rename(deserialize = "trenPar"))]
    pub even_train_id: Option<String>,

    /// Train ID for odd trains.
    #[serde(rename(deserialize = "trenImpar"))]
    pub odd_train_id: Option<String>,

    /// The creation date of the train update.
    #[serde(rename(deserialize = "fechaCreacion"))]
    #[serde(deserialize_with = "r9k_date")]
    pub created_date: NaiveDate,

    /// Train's registration number.
    #[serde(rename(deserialize = "numeroRegistro"))]
    pub registration_number: String,

    /// Type of train.
    #[serde(rename(deserialize = "operadorComercial"))]
    pub train_type: TrainType,

    /// Train type code.
    #[serde(rename(deserialize = "codigoOperadorComercial"))]
    pub train_type_code: String,

    /// Full train
    #[serde(rename(deserialize = "trenCompleto"))]
    pub full_train: Option<String>,

    /// Source of the train update.
    #[serde(rename(deserialize = "origenActualizaTren"))]
    pub source: String,

    /// Changes to train trip by station.     
    ///
    /// The list includes one entry for the station that the train has arrived
    /// at, with additional entries for stations not yet visited.
    ///
    /// N.B. Only the first entry is used as the remainder are a schedule only.
    #[serde(rename(deserialize = "pasoTren"), default)]
    pub changes: Vec<Change>,
}

fn r9k_date<'de, D>(deserializer: D) -> anyhow::Result<NaiveDate, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    NaiveDate::parse_from_str(&s, "%d/%m/%Y").map_err(serde::de::Error::custom)
}

impl TrainUpdate {
    /// Get the train ID, preferring even over odd.
    #[must_use]
    pub fn train_id(&self) -> String {
        self.even_train_id.clone().unwrap_or_else(|| self.odd_train_id.clone().unwrap_or_default())
    }

    /// Validate the message.
    ///
    /// # Errors
    ///
    /// Will return one of the following errors:
    ///  - `Error::NoUpdate` if there are no changes
    ///  - `Error::NoActualUpdate` if the arrival or departure time is -ve or 0
    ///  - `Error::Outdated` if the message is too old
    ///  - `Error::WrongTime` if the message is from the future
    pub fn validate(&self) -> Result<()> {
        if self.changes.is_empty() {
            return Err(R9kError::NoUpdate("contains no updates".to_string()).into());
        }

        // an *actual* update will have a +ve arrival or departure time
        let change = &self.changes[0];
        let since_midnight_secs = if change.has_departed {
            change.actual_departure_time
        } else if change.has_arrived {
            change.actual_arrival_time
        } else {
            return Err(R9kError::NoUpdate("arrival/departure time <= 0".to_string()).into());
        };

        if since_midnight_secs <= 0 {
            return Err(R9kError::NoUpdate("arrival/departure time <= 0".to_string()).into());
        }

        // rebuild the event timestamp from the creation date + seconds from midnight
        let naive_dt = self.created_date.and_hms_opt(0, 0, 0).unwrap_or_default();
        let Some(midnight_dt) = naive_dt.and_local_timezone(Pacific::Auckland).earliest() else {
            return Err(R9kError::BadTime(format!("invalid local time: {naive_dt}")).into());
        };
        let midnight_ts = midnight_dt.timestamp();
        let event_ts = midnight_ts + i64::from(since_midnight_secs);

        // calculate delay from 'now'
        let now_ts = Utc::now().with_timezone(&Pacific::Auckland).timestamp();
        let delay_secs = now_ts - event_ts;

        // TODO: do we need this metric?;
        tracing::info!(gauge.r9k_delay = delay_secs);

        if delay_secs > MAX_DELAY_SECS {
            return Err(R9kError::BadTime(format!("outdated by {delay_secs} seconds")).into());
        }
        if delay_secs < MIN_DELAY_SECS {
            return Err(
                R9kError::BadTime(format!("too early by {} seconds", delay_secs.abs())).into()
            );
        }

        Ok(())
    }
}

/// R9K train update change.
#[derive(Debug, Clone, Deserialize)]
pub struct Change {
    /// Type of change that triggered the update message.
    #[serde(rename(deserialize = "tipoCambio"))]
    pub r#type: ChangeType,

    /// Station identifier.
    #[serde(rename(deserialize = "estacion"))]
    pub station: u32,

    /// Unique id for the entry.
    #[serde(rename(deserialize = "idPaso"))]
    pub entry_id: String,

    /// Scheduled arrival time as per schedule.
    /// In seconds from train update creation date at midnight.
    #[serde(rename(deserialize = "horaEntrada"))]
    pub arrival_time: i32,

    /// Actual arrival, or estimated arrival time (based on the latest actual
    /// arrival or departure time of the preceding stations).
    ///
    /// In seconds from train update creation date at midnight. `-1` if not
    /// available.
    #[serde(rename(deserialize = "horaEntradaReal"))]
    pub actual_arrival_time: i32,

    /// The train has arrived.
    #[serde(rename(deserialize = "haEntrado"))]
    pub has_arrived: bool,

    /// Difference between the actual and scheduled arrival times if the train
    /// has already arrived at the station, 0 otherwise.
    #[serde(rename(deserialize = "retrasoEntrada"))]
    pub arrival_delay: i32,

    /// Scheduled departure time as per schedule.
    ///
    /// In seconds from train update creation date at midnight.
    #[serde(rename(deserialize = "horaSalida"))]
    pub departure_time: i32,

    /// Actual departure, or estimated departure time (based on the latest
    /// actual arrival or departure time of the preceding stations).
    ///
    /// In seconds from train update creation date at midnight. -1 if not
    /// available.
    #[serde(rename(deserialize = "horaSalidaReal"))]
    pub actual_departure_time: i32,

    /// The train has departed.
    #[serde(rename(deserialize = "haSalido"))]
    pub has_departed: bool,

    /// Difference between the actual and scheduled arrival times if the train
    /// has already arrived at the station, 0 otherwise.
    #[serde(rename(deserialize = "retrasoSalida"))]
    pub departure_delay: i32,

    /// The time at which the train was detained.
    #[serde(rename(deserialize = "horaInicioDetencion"))]
    pub detention_time: i32,

    /// The duration for which the train was detained.
    #[serde(rename(deserialize = "duracionDetencion"))]
    pub detention_duration: i32,

    /// The platform at which the train arrived.
    #[serde(rename(deserialize = "viaEntradaMallas"))]
    pub platform: String,

    /// The exit line from a station.
    #[serde(rename(deserialize = "viaCirculacionMallas"))]
    pub exit_line: String,

    /// Train direction in reference to the platform.
    #[serde(rename(deserialize = "sentido"))]
    pub train_direction: Direction,

    /// Should be an enum, but again, we don't have the full list.
    /// 4 - Original, Passing (non-stop/skip), or Destination (no dwell time in timetable)
    /// 5 - Intermediate stop (there is a dwell time in the time table).
    #[serde(rename(deserialize = "tipoParada"))]
    pub stop_type: StopType,

    /// N.B. Not sure what this is used for.
    #[serde(rename(deserialize = "paridad"))]
    pub parity: String,
}

/// The type of change that triggered the update message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr)]
#[repr(u8)]
pub enum ChangeType {
    /// Train has exited the first station.
    ExitedFirstStation = 1,

    /// Train has reached the final destination.
    ReachedFinalDestination = 2,

    /// Train has arrived at the station.
    ArrivedAtStation = 3,

    /// Train has exited the station.
    ExitedStation = 4,

    /// Train has passed the station without stopping.
    PassedStationWithoutStopping = 5,

    /// Train has been parked between stations.
    DetainedInPark = 6,

    /// Train has been detained at the station.
    DetainedAtStation = 7,

    /// Station is no longer part of the run.
    StationNoLongerPartOfTheRun = 8,

    /// Platform has changed.
    PlatformChange = 9,

    /// Exit line has changed.
    ExitLineChange = 10,

    /// Schedule has changed.
    ScheduleChange = 11,
}

impl Display for ChangeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReachedFinalDestination => write!(f, "ReachedFinalDestination"),
            Self::ArrivedAtStation => write!(f, "ArrivedAtStation"),
            Self::ExitedFirstStation => write!(f, "ExitedFirstStation"),
            Self::ExitedStation => write!(f, "ExitedStation"),
            Self::PassedStationWithoutStopping => write!(f, "PassedStationWithoutStopping"),
            Self::DetainedInPark => write!(f, "DetainedInPark"),
            Self::DetainedAtStation => write!(f, "DetainedAtStation"),
            Self::StationNoLongerPartOfTheRun => write!(f, "StationNoLongerPartOfTheRun"),
            Self::PlatformChange => write!(f, "PlatformChange"),
            Self::ExitLineChange => write!(f, "ExitLineChange"),
            Self::ScheduleChange => write!(f, "ScheduleChange"),
        }
    }
}

impl ChangeType {
    /// Returns `true` when this change type indicates a meaningful service update.
    #[must_use]
    pub const fn is_relevant(&self) -> bool {
        matches!(
            self,
            Self::ReachedFinalDestination
                | Self::ArrivedAtStation
                | Self::ExitedFirstStation
                | Self::ExitedStation
                | Self::PassedStationWithoutStopping
                | Self::ScheduleChange
        )
    }

    /// Returns `true` when this change type corresponds to an arrival event.
    #[must_use]
    pub const fn is_arrival(&self) -> bool {
        matches!(self, Self::ArrivedAtStation | Self::ReachedFinalDestination)
    }
}

/// Type of train.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrainType {
    /// Metro train.
    #[default]
    Metro,

    /// Ex Metro train.
    Exmetro,

    /// Freight train.
    Freight,
}

/// Direction of travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr)]
#[repr(i8)]
pub enum Direction {
    /// Right.
    Right = 0,

    /// Left.
    Left = 1,

    /// Unspecified.
    Unspecified = -1,
}

/// Direction of travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr)]
#[repr(i8)]
pub enum StopType {
    /// Original, Passing (non-stop/skip), or Destination (no dwell time in
    /// timetable).
    Original = 4,

    /// Intermediate stop (there is a dwell time in the time table).
    Intermediate = 5,
}
```

## Output messages module (smartrak.rs)

```rust
//! SmarTrak event types for handling SmarTrak data.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize, Serializer};

use crate::stops::StopInfo;

/// SmarTrak event.
/// N.B. that `@JsonProperty` descriptors are used for deserialisation only,
/// while the property name will be used when the data is serialised before
/// being published.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmarTrakEvent {
    /// The time the event was received.
    #[serde(serialize_with = "with_nanos")]
    pub received_at: DateTime<Utc>,

    /// The type of the event.
    // #[serde(rename(deserialize = "event"))]
    pub event_type: EventType,

    /// Event data containing specific details about the event.
    pub event_data: EventData,

    /// Message data for the event.
    pub message_data: MessageData,

    /// Remote data associated with the event.
    pub remote_data: RemoteData,

    /// Location data for the event.
    pub location_data: LocationData,

    /// The identifier of the company associated with the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<u64>,

    /// Serial data associated with the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial_data: Option<SerialData>,
}

fn with_nanos<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let trunc = dt.to_rfc3339_opts(SecondsFormat::Millis, true);
    serializer.serialize_str(&trunc)
}

/// Smartrak event type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventType {
    /// Location event.
    #[default]
    Location,

    /// Serial data event.
    SerialData,
}

/// Message data associated with an event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageData {
    /// Message identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<u64>,

    /// Message timestamp.
    pub timestamp: DateTime<Utc>,
}

impl Default for MessageData {
    fn default() -> Self {
        Self { message_id: None, timestamp: Utc::now() }
    }
}

/// Remote data associated with the event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteData {
    /// Remote identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_id: Option<u64>,

    /// Remote name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_name: Option<String>,

    /// External identifier.
    pub external_id: String,
}

/// Event data with specific details about the event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventData {
    /// Event code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_code: Option<u64>,

    /// Odometer reading at the time of the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub odometer: Option<u64>,

    /// Nearest address to the event location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nearest_address: Option<String>,

    /// Additional information about the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_info: Option<String>,
}

/// Location data for the event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationData {
    /// Latitude of the event location.
    pub latitude: f64,

    /// Longitude of the event location.
    pub longitude: f64,

    /// Speed of the event location.
    pub speed: i64,

    /// GPS accuracy of the event location.
    pub gps_accuracy: i64,

    /// Heading of the event location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<f64>,

    /// Kilometric point of the event location, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kilometric_point: Option<f64>,
}

impl From<StopInfo> for LocationData {
    fn from(stop: StopInfo) -> Self {
        Self { latitude: stop.stop_lat, longitude: stop.stop_lon, ..Self::default() }
    }
}

impl From<&StopInfo> for LocationData {
    fn from(stop: &StopInfo) -> Self {
        stop.clone().into()
    }
}

/// Serial data associated with the event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialData {
    /// Source of the serial data.
    pub source: u64,

    /// Raw serial bytes.
    pub serial_bytes: String,

    /// Decoded serial data.
    pub decoded: Option<DecodedSerialData>,
}

/// Decodes bdc Serial Data, supports base64 format encoded and just string.
///
/// ex: `MjQ1MDU0NDgzMTJjMzEyYzMxMzUzYTMwMzgyYzMwMmMzMjMwMzIzMTM5MzgzNTMzMmMyYzJjMzQzMzMxMzUzMDJjMzEyYzMxMzUzYTMyMzAyYzMxMmMzNDMzMzIzMzJjMzMzMzM2MzkyYzMxMzUyYzM2MmMzMjJhMzYzNg==`
/// ex: `$PTH1,1,00:02,0,22101670,,7380,124046,2,23:45,1,2035,2037,0,0,0*6b`
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedSerialData {
    /// Identifier of the service line associated with this serial record.
    pub line_id: String,
    /// Trip number reported in the serial data.
    pub trip_number: String,
    /// Trip start time encoded in the serial payload.
    pub start_at: String,
    /// Number of passengers recorded for the trip.
    pub passengers_number: u32,
    /// Identifier of the driver operating the trip.
    pub driver_id: String,
    /// Flag indicating whether the trip is currently active.
    pub trip_active: bool,
    /// Flag indicating whether the trip has ended.
    pub trip_ended: bool,
    /// Flag signifying the presence of a trip-ended marker.
    pub has_trip_ended_flag: bool,
    /// Count of tag-on events recorded.
    pub tag_ons: u32,
    /// Count of tag-off events recorded.
    pub tag_offs: u32,
    /// Cash fare transactions recorded for the trip.
    pub cash_fares: u32,
}
```

## Look-up information via HTTP module (stop.rs)

```rust
use std::collections::HashMap;
use std::sync::LazyLock;

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use http_body_util::Empty;
use omnia_sdk::{Config, HttpRequest, Identity, Publisher};
use serde::{Deserialize, Serialize};

/// Stop information from GTFS
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StopInfo {
    /// GTFS stop code used to identify the stop.
    pub stop_code: String,
    /// Latitude coordinate of the stop in decimal degrees.
    pub stop_lat: f64,
    /// Longitude coordinate of the stop in decimal degrees.
    pub stop_lon: f64,
}

pub async fn stop_info<P>(
    _owner: &str, provider: &P, station: u32, is_arrival: bool,
) -> Result<Option<StopInfo>>
where
    P: Config + HttpRequest + Identity + Publisher,
{
    if !ACTIVE_STATIONS.contains(&station) {
        return Ok(None);
    }

    // FIXME: if station is in list above, we should always get location data
    // get station's stop code
    let Some(stop_code) = STATION_STOP.get(&station) else {
        return Ok(None);
    };

    let cc_static_api_url =
        Config::get(provider, "CC_STATIC_URL").await.context("getting `CC_STATIC_URL`")?;
    let request = http::Request::builder()
        .uri(format!("{cc_static_api_url}/gtfs/stops?fields=stop_code,stop_lon,stop_lat"))
        .body(Empty::<Bytes>::new())
        .context("building block management request")?;
    let response = HttpRequest::fetch(provider, request).await.context("fetching stops")?;

    let bytes = response.into_body();
    let stops: Vec<StopInfo> =
        serde_json::from_slice(&bytes).context("deserializing block management response")?;

    let Some(mut stop_info) = stops.into_iter().find(|stop| stop.stop_code == *stop_code) else {
        return Err(anyhow!("stop info not found for stop code {stop_code}"));
    };

    if !is_arrival {
        stop_info = DEPARTURES.get(&stop_info.stop_code).cloned().unwrap_or(stop_info);
    }

    Ok(Some(stop_info))
}

const ACTIVE_STATIONS: &[u32] = &[0, 19, 40];

static STATION_STOP: LazyLock<HashMap<u32, &str>> =
    LazyLock::new(|| HashMap::from([(0, "133"), (19, "9218"), (40, "134")]));

// Correct stops that have separate departure and arrival locations.
static DEPARTURES: LazyLock<HashMap<String, StopInfo>> = LazyLock::new(|| {
    HashMap::from([
        (
            "133".to_string(),
            StopInfo { stop_code: "133".to_string(), stop_lat: -36.84448, stop_lon: 174.76915 },
        ),
        (
            "134".to_string(),
            StopInfo { stop_code: "134".to_string(), stop_lat: -37.20299, stop_lon: 174.90990 },
        ),
        (
            "9218".to_string(),
            StopInfo { stop_code: "9218".to_string(), stop_lat: -36.99412, stop_lon: 174.8770 },
        ),
    ])
});
```
