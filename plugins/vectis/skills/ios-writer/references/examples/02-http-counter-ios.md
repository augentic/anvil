# Example: HTTP Counter iOS Shell

An iOS shell for a Crux counter app that persists count to a server via HTTP.
Demonstrates async HTTP effect handling, loading states, and error views.

This shell pairs with the core-writer example `02-http-counter.md`. The
shared crate defines:

- `ViewModel::Loading`, `ViewModel::Counter(CounterView)`, `ViewModel::Error(ErrorView)`
- Shell-facing events: `Event::Navigate(Route)`, `Event::Increment`, `Event::Decrement`, `Event::Reset`, `Event::FetchCount`
- Internal events: `Event::Set(Result<...>)`, `Event::Updated(Result<...>)`
- `Effect::Render(RenderOperation)`, `Effect::Http(HttpRequest)`
- `Route::Counter`
- `CounterView { count: String }`, `ErrorView { message: String, can_retry: bool }`

## Capabilities Handled

- **Render** -- update the published `ViewModel`
- **HTTP** -- perform HTTP requests via `URLSession`

## Directory Structure

```
examples/http-counter/
    shared/
    iOS/
        project.yml
        Makefile
        HttpCounter/
            HttpCounterApp.swift
            Core.swift
            ContentView.swift
            Views/
                LoadingScreen.swift
                CounterScreen.swift
                ErrorScreen.swift
```

## `iOS/HttpCounter/Core.swift`

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
        update(.fetchCount)
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
}
```

## `iOS/HttpCounter/ContentView.swift`

```swift
import Inject
import SwiftUI
import VectisDesign

struct ContentView: View {
    @ObservedObject var core: Core
    @ObserveInjection var inject

    var body: some View {
        switch core.view {
        case .loading:
            LoadingScreen()
        case .counter(let viewModel):
            CounterScreen(viewModel: viewModel) { event in
                core.update(event)
            }
        case .error(let viewModel):
            ErrorScreen(viewModel: viewModel) { event in
                core.update(event)
            }
        }
        .enableInjection()
    }
}
```

## `iOS/HttpCounter/Views/ErrorScreen.swift`

```swift
import Inject
import SwiftUI
import VectisDesign

struct ErrorScreen: View {
    let viewModel: ErrorView
    let onEvent: (Event) -> Void
    @ObserveInjection var inject

    var body: some View {
        VStack(spacing: VectisSpacing.lg) {
            Spacer()

            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 56))
                .foregroundStyle(VectisColors.error)
                .accessibilityHidden(true)

            Text(viewModel.message)
                .font(VectisTypography.body)
                .foregroundStyle(VectisColors.onSurface)
                .multilineTextAlignment(.center)
                .padding(.horizontal, VectisSpacing.xl)

            if viewModel.canRetry {
                Button("Try Again") {
                    onEvent(.navigate(.counter))
                }
                .buttonStyle(.borderedProminent)
                .tint(VectisColors.primary)
            }

            Spacer()
        }
        .frame(maxWidth: .infinity)
        .background(VectisColors.surface)
        .enableInjection()
    }
}

#Preview("With retry") {
    ErrorScreen(
        viewModel: ErrorView(
            message: "Failed to connect to server. Please check your connection.",
            canRetry: true
        ),
        onEvent: { _ in }
    )
    .vectisTheme()
}

#Preview("Without retry") {
    ErrorScreen(
        viewModel: ErrorView(
            message: "An unexpected error occurred.",
            canRetry: false
        ),
        onEvent: { _ in }
    )
    .vectisTheme()
}
```

## `iOS/HttpCounter/Views/CounterScreen.swift`

```swift
import Inject
import SwiftUI
import VectisDesign

struct CounterScreen: View {
    let viewModel: CounterView
    let onEvent: (Event) -> Void
    @ObserveInjection var inject

    var body: some View {
        VStack(spacing: VectisSpacing.lg) {
            Spacer()

            Text(viewModel.count)
                .font(VectisTypography.largeTitle)
                .foregroundStyle(VectisColors.onSurface)
                .contentTransition(.numericText())

            HStack(spacing: VectisSpacing.md) {
                Button {
                    onEvent(.decrement)
                } label: {
                    Image(systemName: "minus.circle.fill")
                        .font(.system(size: 44))
                }
                .tint(VectisColors.secondary)
                .accessibilityLabel("Decrement")

                Button {
                    onEvent(.reset)
                } label: {
                    Image(systemName: "arrow.counterclockwise.circle.fill")
                        .font(.system(size: 44))
                }
                .tint(VectisColors.error)
                .accessibilityLabel("Reset")

                Button {
                    onEvent(.increment)
                } label: {
                    Image(systemName: "plus.circle.fill")
                        .font(.system(size: 44))
                }
                .tint(VectisColors.primary)
                .accessibilityLabel("Increment")
            }

            Spacer()
        }
        .frame(maxWidth: .infinity)
        .background(VectisColors.surface)
        .enableInjection()
    }
}

#Preview {
    CounterScreen(
        viewModel: CounterView(count: "Count is: 7"),
        onEvent: { _ in }
    )
    .vectisTheme()
}
```

## Key Patterns Demonstrated

1. **HTTP effect handling** -- `performHttpRequest` uses `URLSession.shared.data(for:)`.
2. **Async effect resolution** -- HTTP effects run in a `Task`, results resolve
   back to the core via `core.resolve(id, data)`.
3. **Error view** -- `ErrorScreen` renders the `ErrorView` view model with a
   conditional retry button.
4. **Three-view pattern** -- Loading, Counter, Error correspond to the three
   `ViewModel` variants.
5. **Initialization** -- `Core.init()` dispatches `.fetchCount` to trigger the
   initial data load.
6. **Graceful HTTP failure** -- network errors return a zero-status response
   rather than crashing; the core handles the error state.
