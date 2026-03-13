# Replay Tests Example: r9k-adapter

Production data replay testing for a time-sensitive messaging handler.

## Test Structure

```
crates/r9k-adapter/
├── tests/
│   ├── provider.rs         # Replay fixture + MockProvider + shift_time
│   ├── static.rs           # Named tests
│   └── replay.rs           # Directory-scanning replay runner
│   └── data/
        └── replay/
            ├── 0001.json       # Production snapshot: error case
            ├── 0304.json       # Production snapshot: success case
            └── ...
```

The `tests/provider.rs` file is shared between `static.rs` and `replay.rs`. See [testing patterns](../../../../omnia/skills/crate-writer/examples/testing.md) for the `static.rs` example and MockProvider setup.

## tests/provider.rs

Contains three components: the `Replay` fixture, the `MockProvider`, and the `shift_time` function.

```rust
#![allow(missing_docs)]

use core::panic;
use std::any::Any;
use std::error::Error;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use chrono::{Timelike, Utc};
use chrono_tz::Pacific::Auckland;
use http::{Request, Response};
use omnia_sdk::{Config, HttpRequest, Identity, Message, Publish};
use r9k_adapter::{R9kMessage, SmarTrakEvent};
use serde::Deserialize;
use serde_json::Value;

// ---- Foundational Traits and Types ----

/// A trait that expresses the structure of taking in some data and
/// constructing (say by deserialization) an input and an output.
pub trait Fixture {
    /// Type of input data needed by the test case. In most cases this is likely
    /// to be the request type of the handler under test.
    type Input: Default;

    /// Type of output data produced by the test case. This could be the
    /// expected output type of the handler under test, or an error type for
    /// failure cases. Many tests cases don't care about the handler's output
    /// type but a type that represents success or failure of some internal
    /// processing.
    type Output;

    /// Type of error that can occur when producing the expected output.
    type Error: std::error::Error;

    /// Sometimes the raw input data needs to be transformed before being
    /// passed to the test case handler, for example to adjust timestamps to
    /// be relative to 'now'.
    type TransformParams;

    /// Convert test data definition into the specific data type that implements
    /// this trait.
    fn from_data(data_def: &TestDef<Self::Error>) -> Self;

    /// Convert input data into the input type needed by the test case handler.
    fn input(&self) -> Option<Self::Input>;

    /// Convert input data into transformation parameters for the test case
    /// handler.
    fn params(&self) -> Option<Self::TransformParams> {
        None
    }

    /// Apply a transformation function to the input data before passing it to
    /// the test case handler.
    ///
    /// The default implementation returns a default input when there is no
    /// input data or applies the given transformation function to the input
    /// data. In most cases this should be sufficient.
    fn transform<F>(&self, f: F) -> Self::Input
    where
        F: FnOnce(&Self::Input, Option<&Self::TransformParams>) -> Self::Input,
    {
        let Some(input) = &self.input() else {
            return Self::Input::default();
        };
        f(input, self.params().as_ref())
    }

    /// Convert input data into the expected output type needed by the test
    /// case handler, which could be an error for failure cases.
    ///
    /// # Errors
    ///
    /// Returns an error when the fixture cannot produce the expected output.
    fn output(&self) -> Option<Result<Self::Output, Self::Error>>;
}

/// A test case builder that can be prepared for execution.
#[derive(Clone, Debug)]
pub struct TestCase<D>
where
    D: Fixture + Clone,
{
    test_def: TestDef<D::Error>,
}

/// A test case that has been prepared for execution by transforming its input
/// and extracting its expected output and extension data into a form that is
/// digestible by the test runner.
#[derive(Clone, Debug)]
pub struct PreparedTestCase<D>
where
    D: Fixture + Clone,
{
    /// Prepared input data ready for the handler under test.
    pub input: Option<D::Input>,
    /// Optional http request mocks required by the handler.
    pub http_requests: Option<Vec<Fetch>>,
    /// Expected output or error produced by the fixture.
    pub output: Option<Result<D::Output, D::Error>>,
}

impl<D> TestCase<D>
where
    D: Clone + Fixture,
{
    /// Create a new test case from the given fixture data.
    #[must_use]
    pub const fn new(test_def: TestDef<D::Error>) -> Self {
        Self { test_def }
    }

    /// Apply input transformation and translation of input data types into
    /// the types needed by the test case handler.
    pub fn prepare<F>(&self, transform: F) -> PreparedTestCase<D>
    where
        F: FnOnce(&D::Input, Option<&D::TransformParams>) -> D::Input,
    {
        let http_requests = self.test_def.http_requests.clone();
        let data = D::from_data(&self.test_def);
        let output = data.output();
        if data.input().is_none() {
            return PreparedTestCase { input: None, http_requests, output };
        }
        let input = data.transform(transform);
        PreparedTestCase { input: Some(input), http_requests, output }
    }
}

/// Standard test definition.
#[derive(Clone, Debug, Deserialize)]
pub struct TestDef<E: std::error::Error> {
    /// Input data.
    ///
    /// The `Value` is expected to be deserialized into the input
    /// type needed by the test case handler.
    pub input: Option<Value>,

    /// Transform parameters.
    ///
    /// Optional parameters that can be used to transform the input data
    /// before passing it to the test case handler. The type of this field
    /// depends on the specific test case handler so we use generic JSON here.
    pub params: Option<Value>,

    /// Outgoing HTTP requests that need to be mocked.
    pub http_requests: Option<Vec<Fetch>>,

    /// Output data.
    ///
    /// The expected output from the test case handler. This can either be an
    /// error or a successful output, depending on the test case. The type of
    /// this field depends on the specific test case handler so we use generic
    /// JSON here.
    ///
    /// Note: The "output" need not be the return type of the underlying handler
    /// under test. It could be a database query or published message that is
    /// sent out from the handler.
    pub output: Option<TestResult<E>>,
}

/// Overlay for a standard rust `Result` type that has tidier deserialization.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all(deserialize = "snake_case"))]
pub enum TestResult<E: std::error::Error> {
    /// Successful result.
    Success(Value),
    /// Error result.
    Failure(E),
}

/// Configuration for mocking fetch requests.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Fetch {
    /// Authority (host) to match for mock fetch requests.
    pub authority: Option<String>,

    /// Method to match for mock fetch requests.
    ///
    /// Defaults to GET.
    #[serde(default)]
    pub method: Method,

    /// Path to match for mock fetch requests, not including query parameters.
    #[serde(default = "default_path")]
    pub path: String,

    /// String to uniquely identify a fetch request.
    ///
    /// This simulates a query string or body content to differentiate requests
    /// so a serialized representation of those could be used in test fixtures,
    /// or some abbreviated identifier.
    pub request: Option<String>,

    /// Expected response if all the other fields match.
    #[serde(default)]
    pub response: Response,
}

fn default_path() -> String {
    "/".to_string()
}

/// Supported HTTP verbs (methods) for fetch requests.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    /// GET method.
    #[default]
    Get,
    /// POST method.
    Post,
    /// PUT method.
    Put,
    /// DELETE method.
    Delete,
    /// PATCH method.
    Patch,
}

/// Mock HTTP response for fetch requests.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Response {
    /// HTTP status code.
    ///
    /// Defaults to 200 so can be omitted in test fixtures unless a specific
    /// status is asserted.
    pub status: u16,

    /// Response body.
    ///
    /// This is a `Value` that the test is expected to deserialize as needed.
    /// Defaults to an empty string for tests that do not require asserting on
    /// response body contents.
    pub body: Value,
}

impl Default for Response {
    fn default() -> Self {
        Self { status: 200, body: Value::String(String::new()) }
    }
}

/// Collection of fetch request configurations that can be used in an Augentic
/// `HttpRequest` capability.
#[derive(Clone, Debug, Deserialize)]
pub struct Fetcher {
    /// List of fetch request configurations.
    pub fetches: Vec<Fetch>,
}

impl Fetcher {
    /// Create a new Fetcher with the given fetch request configurations.
    #[must_use]
    pub fn new(fetches: &[Fetch]) -> Self {
        Self { fetches: fetches.to_vec() }
    }

    /// Simulate fetching a request by finding a matching fetch configuration
    /// and returning the response.
    ///
    /// # Errors
    ///
    /// Returns an error when the request method is unsupported, the authority
    /// or host header is missing, or no matching fetch configuration is found.
    pub fn fetch<T>(&self, request: &http::Request<T>) -> anyhow::Result<http::Response<Bytes>> {
        let method = match *request.method() {
            http::Method::GET => Method::Get,
            http::Method::POST => Method::Post,
            http::Method::PUT => Method::Put,
            http::Method::DELETE => Method::Delete,
            http::Method::PATCH => Method::Patch,
            _ => return Err(anyhow!("unsupported HTTP method: {}", request.method())),
        };

        let authority = request
            .uri()
            .authority()
            .map(|auth| auth.as_str().to_owned())
            .or_else(|| {
                request.headers().get(HOST).and_then(|value| value.to_str().ok().map(str::to_owned))
            })
            .ok_or_else(|| anyhow!("request missing authority or host header"))?;

        let path = request.uri().path().to_owned();
        let request_id = request.uri().query().map(str::to_owned);

        let fetch = self.fetches.iter().find(|candidate| {
            candidate.authority.as_ref().is_none_or(|auth| auth == &authority)
                && candidate.method == method
                && candidate.path == path
                && (candidate.request.is_none() || candidate.request == request_id)
        });

        let fetch = fetch.ok_or_else(|| {
            anyhow!(
                "no fetch configured for method={method:?}, authority={authority}, path={path}, request={request_id:?}"
            )
        })?;

        let status = fetch.response.status;
        let body = Bytes::from(fetch.response.body.to_string());

        http::Response::builder().status(status).body(body).map_err(anyhow::Error::new)
    }
}

// ---- Replay Fixture ----

#[derive(Debug, Clone, Deserialize)]
pub struct Replay {
    pub input: Option<R9kMessage>,
    pub params: Option<ReplayTransform>,
    pub output: Option<ReplayOutput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ReplayOutput {
    Events(Vec<SmarTrakEvent>),
    Error(omnia_sdk::Error),
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct ReplayTransform {
    pub delay: i32,
}

impl Fixture for Replay {
    type Error = omnia_sdk::Error;
    type Input = R9kMessage;
    type Output = Vec<SmarTrakEvent>;
    type TransformParams = ReplayTransform;

    fn from_data(data_def: &TestDef<Self::Error>) -> Self {
        let input_str: Option<String> = data_def.input.as_ref().and_then(|v| {
            serde_json::from_value(v.clone()).expect("should deserialize input as XML String")
        });
        let input = input_str.map(|s| {
            let msg: R9kMessage =
                quick_xml::de::from_str(&s).expect("should deserialize R9kMessage");
            msg
        });
        let params: Option<Self::TransformParams> = data_def.params.as_ref().and_then(|v| {
            serde_json::from_value(v.clone()).expect("should deserialize transform parameters")
        });
        let Some(output_def) = &data_def.output else {
            return Self { input, params, output: None };
        };
        let output = match output_def {
            TestResult::Success(value) => serde_json::from_value(value.clone()).map_or_else(
                |_| panic!("should deserialize output as SmarTrak events"),
                |events| Some(ReplayOutput::Events(events)),
            ),
            TestResult::Failure(err) => Some(ReplayOutput::Error(err.clone())),
        };
        Self { input, params, output }
    }

    fn input(&self) -> Option<Self::Input> {
        self.input.clone()
    }

    fn params(&self) -> Option<Self::TransformParams> {
        self.params.clone()
    }

    fn output(&self) -> Option<Result<Self::Output, Self::Error>> {
        let output = self.output.as_ref()?;
        match output {
            ReplayOutput::Error(error) => Some(Err(error.clone())),
            ReplayOutput::Events(events) => {
                if events.is_empty() {
                    return None;
                }
                Some(Ok(events.clone()))
            }
        }
    }
}

// ---- MockProvider ----

#[derive(Clone)]
pub struct MockProvider {
    test_case: PreparedTestCase<Replay>,
    events: Arc<Mutex<Vec<SmarTrakEvent>>>,
}

impl MockProvider {
    #[allow(clippy::missing_panics_doc)]
    #[allow(dead_code)]
    #[must_use]
    pub fn events(&self) -> Vec<SmarTrakEvent> {
        self.events.lock().expect("should lock").clone()
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn new(test_case: PreparedTestCase<Replay>) -> Self {
        Self { test_case, events: Arc::new(Mutex::new(Vec::new())) }
    }
}

impl Config for MockProvider {
    async fn get(&self, _key: &str) -> Result<String> {
        Ok("http://localhost:8080".to_string())
    }
}

impl HttpRequest for MockProvider {
    async fn fetch<T>(&self, request: Request<T>) -> Result<Response<Bytes>>
    where
        T: http_body::Body + Any,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        let Some(http_requests) = &self.test_case.http_requests else {
            return Err(anyhow!("no http requests defined in replay session"));
        };
        let fetcher = Fetcher::new(http_requests);
        fetcher.fetch(&request)
    }
}

impl Publish for MockProvider {
    async fn send(&self, _topic: &str, message: &Message) -> Result<()> {
        let event: SmarTrakEvent =
            serde_json::from_slice(&message.payload).context("deserializing event")?;
        self.events.lock().map_err(|e| anyhow!("{e}"))?.push(event);
        Ok(())
    }
}

impl Identity for MockProvider {
    async fn access_token(&self, _identity: String) -> Result<String> {
        Ok("mock_access_token".to_string())
    }
}

// ---- Time-shifting function ----

#[must_use]
pub fn shift_time(input: &R9kMessage, params: Option<&ReplayTransform>) -> R9kMessage {
    if params.is_none() {
        return input.clone();
    }
    let delay = params.as_ref().map_or(0, |p| p.delay);
    let mut request = input.clone();
    let Some(change) = request.train_update.changes.get_mut(0) else {
        return request;
    };

    let now = Utc::now().with_timezone(&Auckland);
    request.train_update.created_date = now.date_naive();

    #[allow(clippy::cast_possible_wrap)]
    let from_midnight = now.num_seconds_from_midnight() as i32;
    let adjusted_secs = from_midnight - delay;

    if change.has_departed {
        change.actual_departure_time = adjusted_secs;
    } else if change.has_arrived {
        change.actual_arrival_time = adjusted_secs;
    }
    request
}
```

## tests/replay.rs

The replay runner iterates all fixtures in `data/replay/` and runs each through the handler.

```rust
#![allow(missing_docs)]
#![cfg(not(miri))]

mod provider;

use std::fs::{self, File};

use chrono::Utc;
use chrono_tz::Pacific::Auckland;
use omnia_sdk::{Client, Error};

use crate::provider::{Replay, shift_time};
use crate::{TestCase, TestDef};

#[tokio::test]
async fn run() {
    for entry in fs::read_dir("data/replay").expect("should read directory") {
        let file = File::open(entry.expect("should read entry").path()).expect("should open file");
        let test_def: TestDef<Error> =
            serde_json::from_reader(&file).expect("should deserialize session");
        replay(test_def).await;
    }
}

async fn replay(test_def: TestDef<Error>) {
    let test_case = TestCase::<Replay>::new(test_def).prepare(shift_time);
    let provider = provider::MockProvider::new(test_case.clone());
    let client = Client::new("at").provider(provider.clone());

    let result = client.request(test_case.input.expect("replay test input expected")).await;
    let curr_events = provider.events();

    let Some(expected_result) = &test_case.output else {
        // No expected output: handler should succeed with no side effects
        assert!(curr_events.is_empty());
        return;
    };

    match expected_result {
        Ok(expected_events) => {
            // Success case: compare events with timestamp normalization
            expected_events.iter().zip(curr_events).for_each(|(published, mut actual)| {
                let now = Utc::now().with_timezone(&Auckland);
                let diff = now.timestamp() - actual.message_data.timestamp.timestamp();
                assert!(diff.abs() < 3, "expected vs actual too great: {diff}");

                // Normalize timestamps before comparison
                actual.received_at = published.received_at;
                actual.message_data.timestamp = published.message_data.timestamp;

                let json_actual = serde_json::to_value(&actual).unwrap();
                let json_expected = serde_json::to_value(published).unwrap();
                assert_eq!(json_expected, json_actual);
            });
        }
        Err(expected_error) => {
            // Error case: compare error code and description
            let actual_error = result.expect_err("should have error");
            assert_eq!(actual_error.code(), expected_error.code());
            assert_eq!(actual_error.description(), expected_error.description());
        }
    }
}
```

### Key Design Points

- Single `#[tokio::test]` function `run()` iterates all `data/replay/*.json` files
- `replay()` handles three outcome types:
  1. **No output** -- handler succeeds, no events produced (e.g., unmapped station)
  2. **Success** -- zip expected events with actual, normalize timestamps (3s drift tolerance), compare full JSON
  3. **Error** -- compare error `code()` and `description()`
- `#![cfg(not(miri))]` disables under Miri (filesystem access)
- Timestamp normalization: set `received_at` and `message_data.timestamp` to expected values before comparison, since these are always `Utc::now()` in the handler

## Replay Assertion Flow

```text
Load fixture from data/replay/*.json
    |
TestCase::<Replay>::new(test_def).prepare(shift_time)
    |
MockProvider::new(prepared_test_case)
    |
client.request(input).await
    |
+-- No expected output? --> assert events empty
|
+-- Expected success?
|     |
|     zip(expected_events, actual_events)
|     |
|     assert timestamp drift < 3s
|     normalize timestamps
|     assert_eq JSON
|
+-- Expected error?
      |
      expect_err
      assert code matches
      assert description matches
```

## Example Fixture: Success (data/replay/0304.json)

```json
{
    "input": "<CCO ...>...XML with departure from station 0...</CCO>",
    "params": { "delay": 9 },
    "http_requests": [
        {
            "path": "/gtfs/stops",
            "response": {
                "body": "[{\"stop_code\":\"133\",\"stop_lat\":-36.84429,\"stop_lon\":174.76847}]"
            }
        },
        {
            "path": "/allocations/trips",
            "method": "GET",
            "response": {
                "body": "[\"AMP        1074\",\"AMP        253\"]"
            }
        }
    ],
    "output": {
        "success": [
            "{\"eventType\":\"Location\",\"receivedAt\":\"2025-10-07T11:00:00.000Z\",\"remoteData\":{\"externalId\":\"AMP1074\"},\"locationData\":{\"latitude\":-36.84448,\"longitude\":174.76915,\"speed\":0,\"gpsAccuracy\":0}}",
            "{\"eventType\":\"Location\",\"receivedAt\":\"2025-10-07T11:00:00.000Z\",\"remoteData\":{\"externalId\":\"AMP253\"},\"locationData\":{\"latitude\":-36.84448,\"longitude\":174.76915,\"speed\":0,\"gpsAccuracy\":0}}"
        ]
    }
}
```

## Example Fixture: Error (data/replay/0001.json)

```json
{
    "input": "<CCO ...>...stale XML message...</CCO>",
    "params": { "delay": 506 },
    "output": {
        "failure": {
            "BadRequest": {
                "code": "bad_time",
                "description": "outdated by 506 seconds"
            }
        }
    }
}
```



