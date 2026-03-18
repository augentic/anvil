# Crux iOS Shell Pattern (0.17+ API)

The iOS shell is a thin SwiftUI layer that renders the `ViewModel` from the Crux
core and sends user-initiated `Event` values back. All business logic lives in
the shared Rust crate; the shell only handles platform I/O (HTTP, KV, SSE) and
UI rendering.

## Architecture

```
┌─────────────────────────────────────────────┐
│  SwiftUI Views                              │
│  ┌───────────┐  ┌───────────┐  ┌─────────┐ │
│  │ ListView  │  │ ErrorView │  │ Loading │ │
│  └─────┬─────┘  └─────┬─────┘  └────┬────┘ │
│        │               │              │      │
│        └───────┬───────┘──────────────┘      │
│                ▼                              │
│         ContentView                          │
│          switch viewModel { ... }            │
│                │                              │
│                ▼                              │
│         Core (ObservableObject)              │
│         ┌──────────────────────────────────┐ │
│         │ @Published var view: ViewModel   │ │
│         │ func update(_ event: Event)      │ │
│         │ func processEffect(_ req)        │ │
│         └──────────────┬───────────────────┘ │
│                        │                      │
│                        ▼                      │
│              CoreFfi (UniFFI bridge)          │
│              .update(data) → effects         │
│              .resolve(id, data) → effects    │
│              .view() → viewModel             │
└─────────────────────────────────────────────┘
                        │
                        ▼
             ┌────────────────────┐
             │  Rust shared crate │
             │  (static library)  │
             └────────────────────┘
```

## Core.swift

The `Core` class is the bridge between SwiftUI and the Rust core. It is an
`@MainActor` `ObservableObject` that publishes the current `ViewModel`.

### Minimal Core (Render only)

```swift
import Foundation
import Shared
import SharedTypes

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel

    private let core: CoreFfi

    init() {
        self.core = CoreFfi()
        self.view = try! .bincodeDeserialize(input: [UInt8](core.view()))
    }

    func update(_ event: Event) {
        let effects = [UInt8](
            core.update(Data(try! event.bincodeSerialize()))
        )
        let requests: [Request] = try! .bincodeDeserialize(input: effects)
        for request in requests {
            processEffect(request)
        }
    }

    func processEffect(_ request: Request) {
        switch request.effect {
        case .render:
            self.view = try! .bincodeDeserialize(input: [UInt8](core.view()))
        }
    }
}
```

### Core with HTTP Capability

Add the `.http` case to the effect switch. Use `URLSession` for the request.

```swift
func processEffect(_ request: Request) {
    switch request.effect {
    case .render:
        self.view = try! .bincodeDeserialize(input: [UInt8](core.view()))

    case .http(let httpRequest):
        Task {
            let response = await performHttpRequest(httpRequest)
            let effects = [UInt8](
                core.resolve(
                    request.id,
                    Data(try! HttpResult.ok(response).bincodeSerialize())
                )
            )
            let requests: [Request] = try! .bincodeDeserialize(input: effects)
            for request in requests {
                processEffect(request)
            }
        }
    }
}
```

### HTTP Helper

```swift
private func performHttpRequest(_ request: HttpRequest) async -> HttpResponse {
    var urlRequest = URLRequest(url: URL(string: request.url)!)
    urlRequest.httpMethod = request.method

    for header in request.headers {
        urlRequest.setValue(header.value, forHTTPHeaderField: header.name)
    }

    if !request.body.isEmpty {
        urlRequest.httpBody = Data(request.body)
    }

    do {
        let (data, response) = try await URLSession.shared.data(for: urlRequest)
        let httpResponse = response as! HTTPURLResponse
        return HttpResponse(
            status: UInt16(httpResponse.statusCode),
            headers: httpResponse.allHeaderFields.map { key, value in
                HttpHeader(
                    name: String(describing: key),
                    value: String(describing: value)
                )
            },
            body: [UInt8](data)
        )
    } catch {
        return HttpResponse(status: 0, headers: [], body: [])
    }
}
```

### Core with Key-Value Capability

Use `UserDefaults` or file-based storage for key-value operations.

```swift
case .keyValue(let kvOp):
    Task {
        let result = performKeyValueOp(kvOp)
        let effects = [UInt8](
            core.resolve(
                request.id,
                Data(try! result.bincodeSerialize())
            )
        )
        let requests: [Request] = try! .bincodeDeserialize(input: effects)
        for request in requests {
            processEffect(request)
        }
    }
```

### Core with SSE (Server-Sent Events)

SSE produces a stream of values. Each value is resolved against the same
request ID, producing a new batch of effects each time.

```swift
case .serverSentEvents(let sseRequest):
    Task {
        for await result in await requestSse(sseRequest) {
            let response = try result.get()
            let effects = [UInt8](
                core.resolve(
                    request.id,
                    Data(try! response.bincodeSerialize())
                )
            )
            let requests: [Request] = try! .bincodeDeserialize(input: effects)
            for request in requests {
                processEffect(request)
            }
        }
    }
```

## Serialization Protocol

All data crossing the FFI boundary uses **Bincode** serialization via the
generated `bincodeSerialize()` and `bincodeDeserialize(input:)` methods
on the shared types.

| Direction | Data | Serialization |
|-----------|------|---------------|
| Shell → Core | `Event` | `event.bincodeSerialize()` → `core.update(data)` |
| Core → Shell | Effect requests | `core.update(data)` → `[Request].bincodeDeserialize()` |
| Shell → Core | Effect response | `response.bincodeSerialize()` → `core.resolve(id, data)` |
| Core → Shell | `ViewModel` | `core.view()` → `ViewModel.bincodeDeserialize()` |

## Effect Loop

The effect processing loop is recursive: resolving one effect may produce
additional effects. The loop runs until no more effects are returned.

```
User taps button
    → core.update(Event.buttonTapped)
    → [Request(id: 1, effect: .http(...))]
    → perform HTTP request
    → core.resolve(1, httpResponse)
    → [Request(id: 2, effect: .render)]
    → update published view model
    → SwiftUI re-renders
```

## Initialization

Send an initialization event when the app starts. This triggers the core to
load persisted state or fetch initial data.

```swift
init() {
    self.core = CoreFfi()
    self.view = try! .bincodeDeserialize(input: [UInt8](core.view()))
    update(.navigate(.main))
}
```

## Thread Safety

- `Core` is `@MainActor` -- all property access is main-thread.
- Async effect handlers (`Task { ... }`) return to the main actor because
  `Core` methods are implicitly `@MainActor`.
- `CoreFfi` is thread-safe internally (Rust `Bridge` uses interior mutability).

## Type Mapping: Rust → Swift

The `codegen` binary (or manual type generation) produces Swift equivalents
of all Crux types that cross the FFI boundary.

| Rust Type | Swift Type |
|-----------|------------|
| `enum ViewModel { Loading, Main(MainView) }` | `enum ViewModel { case loading; case main(MainView) }` |
| `enum Event { Navigate(Route), AddItem(String) }` | `enum Event { case navigate(Route); case addItem(String) }` |
| `enum Effect { Render(...), Http(...) }` | `enum Effect { case render(...); case http(...) }` |
| `enum Route { Main, Settings }` | `enum Route { case main; case settings }` |
| `struct MainView { pub items: Vec<ItemView> }` | `struct MainView { var items: [ItemView] }` |
| `String` | `String` |
| `Vec<T>` | `[T]` |
| `Option<T>` | `T?` |
| `bool` | `Bool` |
| `isize` / `i32` | `Int` / `Int32` |
