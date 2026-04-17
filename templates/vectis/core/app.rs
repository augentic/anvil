use crux_core::{
    App, Command,
    macros::effect,
    render::{RenderOperation, render},
};
use facet::Facet;
use serde::{Deserialize, Serialize};
<<<CAP:http
use crux_http::HttpRequest;
CAP:http>>>
<<<CAP:kv
use crux_kv::KeyValueOperation;
CAP:kv>>>
<<<CAP:time
use crux_time::TimeRequest;
CAP:time>>>
<<<CAP:platform
use crux_platform::PlatformRequest;
CAP:platform>>>

#[derive(Default)]
enum Page {
    #[default]
    Home,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub enum Route {
    #[default]
    Home,
}

#[derive(Default)]
pub struct Model {
    page: Page,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct HomeView {
    pub message: String,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
#[repr(C)]
pub enum ViewModel {
    #[default]
    Loading,
    Home(HomeView),
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum Event {
    Navigate(Route),
    <<<CAP:http
    FetchData,
    #[serde(skip)]
    #[facet(skip)]
    Fetched(#[facet(opaque)] crux_http::Result<crux_http::Response<Vec<u8>>>),
    CAP:http>>>
    <<<CAP:kv
    LoadData,
    #[serde(skip)]
    #[facet(skip)]
    Loaded(#[facet(opaque)] Result<Option<Vec<u8>>, crux_kv::KeyValueError>),
    CAP:kv>>>
}

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    <<<CAP:http
    Http(HttpRequest),
    CAP:http>>>
    <<<CAP:kv
    KeyValue(KeyValueOperation),
    CAP:kv>>>
    <<<CAP:time
    Time(TimeRequest),
    CAP:time>>>
    <<<CAP:platform
    Platform(PlatformRequest),
    CAP:platform>>>
}

<<<CAP:http
type Http = crux_http::Http<Effect, Event>;
CAP:http>>>
<<<CAP:kv
type KeyValue = crux_kv::KeyValue<Effect, Event>;
CAP:kv>>>
<<<CAP:time
type Time = crux_time::Time<Effect, Event>;
CAP:time>>>
<<<CAP:platform
type Platform = crux_platform::Platform<Effect, Event>;
CAP:platform>>>

#[derive(Default)]
pub struct __APP_STRUCT__;

impl App for __APP_STRUCT__ {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Navigate(Route::Home) => {
                model.page = Page::Home;
                render()
            }
            <<<CAP:http
            Event::FetchData => render(),
            Event::Fetched(_) => render(),
            CAP:http>>>
            <<<CAP:kv
            Event::LoadData => render(),
            Event::Loaded(_) => render(),
            CAP:kv>>>
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        match model.page {
            Page::Home => ViewModel::Home(HomeView {
                message: "Hello from __APP_NAME__".to_string(),
            }),
        }
    }
}
