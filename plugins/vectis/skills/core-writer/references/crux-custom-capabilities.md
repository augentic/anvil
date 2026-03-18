# Building Custom Capabilities

When published capabilities don't cover a needed side-effect, build a custom one.
A capability consists of:

1. An **Operation** type (the request sent to the shell)
2. An **Output** type (the response from the shell)
3. A **capability struct** with methods that return command builders
4. An **Effect variant** wrapping the Operation

## The Operation Trait

Every operation must implement `crux_core::capability::Operation`, which ties
the request type to its response type:

```rust
use crux_core::capability::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MyRequest {
    pub url: String,
    pub some_param: String,
}

#[derive(Facet, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum MyResponse {
    Data(Vec<u8>),
    Done,
}

impl Operation for MyRequest {
    type Output = MyResponse;
}
```

Rules:
- Both `MyRequest` and `MyResponse` must be serializable (they cross the FFI boundary).
- Derive `Facet` for FFI type generation.
- Add `#[repr(C)]` on response enums.
- `Operation` links them: the shell knows that resolving a `MyRequest` requires
  sending back a `MyResponse`.

## Request-Response Capability

For a one-shot request/response pattern, use `Command::request_from_shell`:

```rust
use std::marker::PhantomData;
use crux_core::{Command, command::RequestBuilder};

pub struct MyCapability<Effect, Event> {
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

impl<Effect, Event> MyCapability<Effect, Event>
where
    Effect: Send + From<crux_core::Request<MyRequest>> + 'static,
    Event: Send + 'static,
{
    pub fn fetch(
        url: impl Into<String>,
        param: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, MyResponse> {
        Command::request_from_shell(MyRequest {
            url: url.into(),
            some_param: param.into(),
        })
    }
}
```

Usage in the app:

```rust
type MyCap = MyCapability<Effect, Event>;

// In update():
MyCap::fetch("https://example.com", "value")
    .then_send(Event::MyResponse)
```

## Streaming Capability (SSE Example)

For capabilities that produce a stream of responses (like Server-Sent Events),
use `StreamBuilder`.

**Critical:** The shell delivers SSE data as arbitrary TCP chunks. An SSE event
can span multiple chunks. You must feed **all** chunks into a **single**
`async_sse::decode` instance via a unified `AsyncBufRead` reader. Never create
a new `decode` per chunk -- events that straddle chunk boundaries will be
silently lost.

```rust
use std::{future, pin::Pin};

use async_sse::{decode, Event as SseEvent};
use crux_core::{capability::Operation, command::StreamBuilder, Request};
use facet::Facet;
use futures::{Stream, StreamExt, TryStreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SseRequest {
    pub url: String,
}

#[derive(Facet, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum SseResponse {
    Chunk(Vec<u8>),
    Done,
}

impl SseResponse {
    #[must_use]
    pub const fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }
}

impl Operation for SseRequest {
    type Output = SseResponse;
}

pub struct ServerSentEvents;

impl ServerSentEvents {
    pub fn get<Effect, Event, T>(
        url: impl Into<String>,
    ) -> StreamBuilder<Effect, Event, Pin<Box<dyn Stream<Item = T> + Send>>>
    where
        Effect: From<Request<SseRequest>> + Send + 'static,
        Event: Send + 'static,
        T: Send + DeserializeOwned,
    {
        let url = url.into();

        StreamBuilder::new(
            |ctx| -> Pin<Box<dyn Stream<Item = T> + Send>> {
                // Convert chunks into a single contiguous AsyncBufRead reader
                // so that async_sse::decode can reassemble events across chunk
                // boundaries.
                let chunk_reader = ctx
                    .stream_from_shell(SseRequest { url })
                    .take_while(|response| future::ready(!response.is_done()))
                    .map(|response| {
                        let SseResponse::Chunk(data) = response else {
                            unreachable!()
                        };
                        Ok::<_, std::io::Error>(data)
                    })
                    .into_async_read();

                Box::pin(
                    decode(chunk_reader).filter_map(|sse_event| async {
                        sse_event.ok().and_then(|event| match event {
                            SseEvent::Message(msg) => {
                                serde_json::from_slice(msg.data()).ok()
                            }
                            SseEvent::Retry(_) => None,
                        })
                    }),
                )
            },
        )
    }
}
```

### Dependencies for SSE

```toml
[dependencies]
async-sse = "5"
futures = "0.3"
```

### Effect variant for SSE

```rust
#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    ServerSentEvents(SseRequest),
}
```

### Usage in the app

```rust
Event::StartWatch => {
    let base = Url::parse(API_URL).unwrap();
    let url = base.join("/sse").unwrap();
    ServerSentEvents::get(url).then_send(Event::Update)
}
Event::Update(data) => {
    model.data = data;
    render()
}
```

The stream automatically sends `Event::Update` for each SSE message.

## File Layout

Place custom capabilities in their own module within the `shared` crate:

```
shared/src/
    lib.rs          # pub mod sse;
    app.rs
    ffi.rs
    sse.rs          # custom SSE capability
```

In `lib.rs`:

```rust
pub mod sse;
```

In `app.rs`:

```rust
use crate::sse::{ServerSentEvents, SseRequest};
```

## Testing Custom Capabilities

Custom capabilities are tested the same way as built-in ones.
The effect macro generates `expect_*` helpers based on the Effect variant name:

```rust
// For Effect::ServerSentEvents(SseRequest), the macro generates:
// .expect_server_sent_events() -> returns the request

#[test]
fn test_sse_subscription() {
    let app = MyApp;
    let mut model = Model::default();

    let mut cmd = app.update(Event::StartWatch, &mut model);

    let mut request = cmd.expect_one_effect().expect_server_sent_events();
    assert_eq!(
        request.operation,
        SseRequest {
            url: "https://api.example.com/sse".to_string(),
        }
    );

    // Resolve with a chunk
    request
        .resolve(SseResponse::Chunk(
            br#"data: {"value": 42}

"#
            .to_vec(),
        ))
        .unwrap();

    let event = cmd.expect_one_event();
    assert_eq!(event, Event::Update(MyData { value: 42 }));

    // Send done to close the stream
    request.resolve(SseResponse::Done).unwrap();
}
```

## Checklist for Custom Capabilities

- [ ] Operation type implements `Operation` with `type Output`
- [ ] Both request and output types derive `Facet, Serialize, Deserialize`
- [ ] Enums have `#[repr(C)]`
- [ ] Effect variant wraps the request type
- [ ] Capability struct uses `PhantomData` for `Effect` and `Event` type params
- [ ] Capability methods have correct trait bounds: `Effect: Send + From<Request<Op>> + 'static`
- [ ] Module is declared in `lib.rs` and imported in `app.rs`
