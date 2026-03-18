# Example: Simple Counter iOS Shell (Render Only)

A minimal iOS shell for a Crux counter app with local state and no external
side-effects. Demonstrates Core.swift, ContentView, screen views, project.yml,
and Makefile.

This shell pairs with the core-writer example `01-simple-counter.md`. The
shared crate defines:

- `ViewModel::Loading` and `ViewModel::Counter(CounterView)` variants
- `Event::Navigate(Route)`, `Event::Increment`, `Event::Decrement`, `Event::Reset`
- `Effect::Render(RenderOperation)`
- `Route::Counter`
- `CounterView { count: String }`

## Capabilities Handled

- **Render** -- update the published `ViewModel`

## Directory Structure

```
examples/counter/
    shared/             # Already exists from core-writer
    iOS/
        project.yml
        Makefile
        Counter/
            CounterApp.swift
            Core.swift
            ContentView.swift
            Views/
                LoadingScreen.swift
                CounterScreen.swift
```

## `iOS/project.yml`

```yaml
name: Counter
packages:
  SharedTypes:
    path: ./generated/SharedTypes
  Shared:
    path: ./generated/Shared
  VectisDesign:
    path: ../../../design-system/ios
  Inject:
    url: https://github.com/krzysztofzablocki/Inject.git
    from: "1.5.2"
options:
  bundleIdPrefix: com.vectis.counter
  deploymentTarget:
    iOS: "17.0"
attributes:
  BuildIndependentTargetsInParallel: true
targets:
  Counter:
    type: application
    platform: iOS
    sources:
      - Counter
    dependencies:
      - package: SharedTypes
      - package: Shared
      - package: VectisDesign
      - package: Inject
    info:
      path: Counter/Info.plist
      properties:
        UILaunchScreen: {}
        UISupportedInterfaceOrientations:
          - UIInterfaceOrientationPortrait
    settings:
      base:
        SWIFT_VERSION: "6.0"
        SWIFT_STRICT_CONCURRENCY: complete
        ENABLE_USER_SCRIPT_SANDBOXING: "NO"
      configs:
        Debug:
          PRODUCT_BUNDLE_IDENTIFIER: com.vectis.counter.debug
          OTHER_LDFLAGS: ["-w", "-Xlinker", "-interposable"]
          EMIT_FRONTEND_COMMAND_LINES: "YES"
        Release:
          PRODUCT_BUNDLE_IDENTIFIER: com.vectis.counter
          OTHER_LDFLAGS: ["-w"]
```

## `iOS/Makefile`

```makefile
.PHONY: all build clean typegen package xcode

SHARED_DIR := ../shared

all: build

build: typegen package xcode

typegen:
	@echo "Generating SharedTypes..."
	@RUST_LOG=info cargo run --manifest-path $(SHARED_DIR)/Cargo.toml \
		--bin codegen --features codegen,facet_typegen -- \
		--language swift --output-dir generated

package:
	@echo "Building Shared Swift package..."
	@cd $(SHARED_DIR) && \
		cargo swift package --name Shared --platforms ios \
			--lib-type static --features uniffi && \
		rm -rf ../iOS/generated/Shared && \
		mkdir -p ../iOS/generated/Shared && \
		cp -r Shared/* ../iOS/generated/Shared/ && \
		rm -rf Shared

xcode:
	@echo "Generating Xcode project..."
	@xcodegen

sim-build:
	@xcodebuild build \
		-project Counter.xcodeproj \
		-scheme Counter \
		-destination 'platform=iOS Simulator,name=iPhone 16' \
		-configuration Debug \
		CODE_SIGNING_ALLOWED=NO \
		2>&1 | xcbeautify

clean:
	@rm -rf generated/ *.xcodeproj
```

## `iOS/Counter/CounterApp.swift`

```swift
import Inject
import SwiftUI

@main
struct CounterApp: App {
    @StateObject private var core = Core()
    @ObserveInjection var inject

    var body: some Scene {
        WindowGroup {
            ContentView(core: core)
                .vectisTheme()
        }
    }
}
```

## `iOS/Counter/Core.swift`

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

## `iOS/Counter/ContentView.swift`

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
        }
        .enableInjection()
    }
}
```

## `iOS/Counter/Views/LoadingScreen.swift`

```swift
import Inject
import SwiftUI
import VectisDesign

struct LoadingScreen: View {
    @ObserveInjection var inject

    var body: some View {
        VStack(spacing: VectisSpacing.md) {
            ProgressView()
                .controlSize(.large)
            Text("Loading...")
                .font(VectisTypography.body)
                .foregroundStyle(VectisColors.onSurfaceSecondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(VectisColors.surface)
        .enableInjection()
    }
}

#Preview {
    LoadingScreen()
        .vectisTheme()
}
```

## `iOS/Counter/Views/CounterScreen.swift`

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
        viewModel: CounterView(count: "Count is: 42"),
        onEvent: { _ in }
    )
    .vectisTheme()
}
```

## Key Patterns Demonstrated

1. **One screen per ViewModel variant** -- `LoadingScreen` and `CounterScreen`.
2. **Event callback pattern** -- screens receive `(Event) -> Void`, not the `Core`.
3. **VectisDesign tokens** -- all colors, fonts, and spacing from the design system.
4. **Preview support** -- every screen has a `#Preview` with sample data.
5. **Accessibility** -- interactive icons have `accessibilityLabel`.
6. **Render-only Core.swift** -- the simplest possible effect handler.
7. **Hot reloading** -- Inject boilerplate (`@ObserveInjection`, `.enableInjection()`)
   in every view; Debug-only linker flags in `project.yml`.
