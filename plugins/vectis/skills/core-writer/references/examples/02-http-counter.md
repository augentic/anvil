# Example: HTTP Counter (Render + HTTP)

A Crux app that communicates with a REST API, demonstrating HTTP requests,
optimistic updates, error handling, and effect testing.

## Capabilities Used

- **Render** (built-in)
- **HTTP** (`crux_http`)

## Workspace `Cargo.toml`

```toml
[workspace]
members = ["shared"]
resolver = "3"

[workspace.package]
edition = "2024"
rust-version = "1.88"

[workspace.dependencies]
crux_core = { git = "https://github.com/redbadger/crux", tag = "crux_core-v0.17.0-rc3" }
crux_http = { git = "https://github.com/redbadger/crux", tag = "crux_core-v0.17.0-rc3" }
serde = "1.0"
facet = "=0.31"
```

## `shared/Cargo.toml`

```toml
[package]
name = "shared"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[lib]
crate-type = ["cdylib", "lib", "staticlib"]

[[bin]]
name = "codegen"
required-features = ["codegen"]

[features]
uniffi = ["dep:uniffi"]
wasm_bindgen = ["dep:wasm-bindgen", "getrandom/wasm_js"]
codegen = [
    "crux_core/cli",
    "crux_http/facet_typegen",
    "dep:clap",
    "dep:log",
    "dep:pretty_env_logger",
    "uniffi",
]
facet_typegen = ["crux_core/facet_typegen", "crux_http/facet_typegen"]

[dependencies]
crux_core.workspace = true
crux_http.workspace = true
serde = { workspace = true, features = ["derive"] }
facet.workspace = true
url = "2"

clap = { version = "4", optional = true, features = ["derive"] }
getrandom = { version = "0.3", optional = true, default-features = false }
log = { version = "0.4", optional = true }
pretty_env_logger = { version = "0.5", optional = true }
uniffi = { version = "=0.29.4", optional = true }
wasm-bindgen = { version = "0.2", optional = true }
```

## `shared/src/lib.rs`

```rust
mod app;
#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
mod ffi;

pub use app::*;
pub use crux_core::Core;

#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
pub use ffi::CoreFFI;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();
```

## `shared/src/app.rs`

```rust
use crux_core::{
    macros::effect,
    render::{render, RenderOperation},
    App, Command,
};
use crux_http::HttpRequest;
use facet::Facet;
use serde::{Deserialize, Serialize};
use url::Url;

const API_URL: &str = "https://crux-counter.fly.dev";

type Http = crux_http::Http<Effect, Event>;

// Domain types

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Count {
    value: isize,
    updated_at: Option<String>,
}

// Page (internal)

#[derive(Default)]
enum Page {
    #[default]
    Loading,
    Counter,
}

// Route (shell-navigable destinations)

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub enum Route {
    #[default]
    Counter,
}

// Model

#[derive(Default, Serialize)]
pub struct Model {
    #[serde(skip)]
    page: Page,
    count: Count,
}

// Per-page view structs

#[derive(Facet, Serialize, Deserialize, Debug, Clone, Default)]
pub struct CounterView {
    pub text: String,
    pub confirmed: bool,
}

// ViewModel

#[derive(Facet, Serialize, Deserialize, Debug, Clone, Default)]
#[repr(C)]
pub enum ViewModel {
    #[default]
    Loading,
    Counter(CounterView),
}

// Events

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Event {
    Navigate(Route),
    Get,
    Increment,
    Decrement,

    #[serde(skip)]
    #[facet(skip)]
    Set(#[facet(opaque)] crux_http::Result<crux_http::Response<Count>>),

    #[serde(skip)]
    #[facet(skip)]
    Updated(#[facet(opaque)] Count),
}

// Effects

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
}

// App

#[derive(Default)]
pub struct Counter;

impl App for Counter {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Navigate(route) => match route {
                Route::Counter => match model.page {
                    Page::Loading => Command::done(),
                    Page::Counter => Command::done(),
                },
            },

            Event::Get => Http::get(API_URL)
                .expect_json()
                .build()
                .then_send(Event::Set),

            Event::Set(Ok(mut response)) => {
                let count = response.take_body().unwrap();
                Command::event(Event::Updated(count))
            }

            Event::Set(Err(_)) => {
                model.page = Page::Counter;
                model.count.updated_at = Some("error".to_string());
                render()
            }

            Event::Updated(count) => {
                model.page = Page::Counter;
                model.count = count;
                render()
            }

            Event::Increment => {
                model.count = Count {
                    value: model.count.value + 1,
                    updated_at: None,
                };

                let call_api = {
                    let base = Url::parse(API_URL).unwrap();
                    let url = base.join("/inc").unwrap();
                    Http::post(url).expect_json().build().then_send(Event::Set)
                };

                render().and(call_api)
            }

            Event::Decrement => {
                model.count = Count {
                    value: model.count.value - 1,
                    updated_at: None,
                };

                let call_api = {
                    let base = Url::parse(API_URL).unwrap();
                    let url = base.join("/dec").unwrap();
                    Http::post(url).expect_json().build().then_send(Event::Set)
                };

                render().and(call_api)
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        match model.page {
            Page::Loading => ViewModel::Loading,
            Page::Counter => {
                let suffix = model
                    .count
                    .updated_at
                    .as_ref()
                    .map_or_else(|| " (pending)".to_string(), |d| format!(" ({d})"));

                ViewModel::Counter(CounterView {
                    text: model.count.value.to_string() + &suffix,
                    confirmed: model.count.updated_at.is_some(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crux_core::App as _;
    use crux_http::{
        protocol::{HttpRequest as HttpReq, HttpResponse, HttpResult},
        testing::ResponseBuilder,
    };

    #[test]
    fn get_fetches_counter() {
        let app = Counter;
        let mut model = Model::default();

        let mut cmd = app.update(Event::Get, &mut model);

        let (operation, mut request) =
            cmd.expect_one_effect().expect_http().split();

        assert_eq!(
            operation,
            HttpReq::get("https://crux-counter.fly.dev/").build()
        );

        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(r#"{"value": 1, "updated_at": "2023-01-01"}"#)
                    .build(),
            ))
            .unwrap();

        let event = cmd.expect_one_event();
        let expected = Event::Set(Ok(ResponseBuilder::ok()
            .body(Count {
                value: 1,
                updated_at: Some("2023-01-01".to_string()),
            })
            .build()));
        assert_eq!(event, expected);
    }

    #[test]
    fn increment_optimistic_update() {
        let app = Counter;
        let mut model = Model {
            count: Count {
                value: 5,
                updated_at: Some("previous".to_string()),
            },
        };

        let mut cmd = app.update(Event::Increment, &mut model);

        assert_eq!(model.count.value, 6);
        assert!(model.count.updated_at.is_none());

        cmd.expect_effect().expect_render();

        let mut request = cmd.expect_one_effect().expect_http();
        assert_eq!(
            &request.operation,
            &HttpReq::post("https://crux-counter.fly.dev/inc").build()
        );

        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(r#"{"value": 6, "updated_at": "2023-01-02"}"#)
                    .build(),
            ))
            .unwrap();

        let event = cmd.expect_one_event();
        assert!(matches!(event, Event::Set(Ok(_))));
    }

    #[test]
    fn decrement_optimistic_update() {
        let app = Counter;
        let mut model = Model {
            count: Count {
                value: 3,
                updated_at: Some("previous".to_string()),
            },
        };

        let mut cmd = app.update(Event::Decrement, &mut model);

        assert_eq!(model.count.value, 2);
        assert!(model.count.updated_at.is_none());

        cmd.expect_effect().expect_render();
        let _ = cmd.expect_one_effect().expect_http();
    }

    #[test]
    fn initial_view_is_loading() {
        let app = Counter;
        let model = Model::default();

        assert!(matches!(app.view(&model), ViewModel::Loading));
    }

    #[test]
    fn view_shows_pending_when_unconfirmed() {
        let app = Counter;
        let model = Model {
            page: Page::Counter,
            count: Count {
                value: 7,
                updated_at: None,
            },
        };

        let ViewModel::Counter(view) = app.view(&model) else {
            panic!("expected Counter view");
        };
        assert_eq!(view.text, "7 (pending)");
        assert!(!view.confirmed);
    }

    #[test]
    fn view_shows_timestamp_when_confirmed() {
        let app = Counter;
        let model = Model {
            page: Page::Counter,
            count: Count {
                value: 7,
                updated_at: Some("2023-01-01".to_string()),
            },
        };

        let ViewModel::Counter(view) = app.view(&model) else {
            panic!("expected Counter view");
        };
        assert_eq!(view.text, "7 (2023-01-01)");
        assert!(view.confirmed);
    }

    #[test]
    fn updated_event_sets_model_and_renders() {
        let app = Counter;
        let mut model = Model::default();

        let count = Count {
            value: 42,
            updated_at: Some("now".to_string()),
        };

        let mut cmd = app.update(Event::Updated(count.clone()), &mut model);
        assert_eq!(model.count, count);

        cmd.expect_one_effect().expect_render();
    }
}
```

## `shared/src/ffi.rs`

```rust
use crux_core::{
    Core,
    bridge::{Bridge, EffectId},
};

use crate::Counter;

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm_bindgen", wasm_bindgen::prelude::wasm_bindgen)]
pub struct CoreFFI {
    core: Bridge<Counter>,
}

impl Default for CoreFFI {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
#[cfg_attr(feature = "wasm_bindgen", wasm_bindgen::prelude::wasm_bindgen)]
impl CoreFFI {
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    #[cfg_attr(
        feature = "wasm_bindgen",
        wasm_bindgen::prelude::wasm_bindgen(constructor)
    )]
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: Bridge::new(Core::new()),
        }
    }

    #[must_use]
    pub fn update(&self, data: &[u8]) -> Vec<u8> {
        let mut effects = Vec::new();
        match self.core.update(data, &mut effects) {
            Ok(()) => effects,
            Err(e) => panic!("{e}"),
        }
    }

    #[must_use]
    pub fn resolve(&self, id: u32, data: &[u8]) -> Vec<u8> {
        let mut effects = Vec::new();
        match self.core.resolve(EffectId(id), data, &mut effects) {
            Ok(()) => effects,
            Err(e) => panic!("{e}"),
        }
    }

    #[must_use]
    pub fn view(&self) -> Vec<u8> {
        let mut view_model = Vec::new();
        match self.core.view(&mut view_model) {
            Ok(()) => view_model,
            Err(e) => panic!("{e}"),
        }
    }
}
```
