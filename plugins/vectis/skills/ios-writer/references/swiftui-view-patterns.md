# SwiftUI View Patterns for Crux Apps

Patterns for building SwiftUI views that consume Crux `ViewModel` data and
dispatch `Event` values back to the core.

## ContentView: ViewModel Switch

The root content view switches on the `ViewModel` enum to display the
appropriate screen. This is the main dispatch point.

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
        case .main(let viewModel):
            MainScreen(viewModel: viewModel) { event in
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

### Rules

- The switch must be exhaustive -- one case per `ViewModel` variant.
- Pass view model data down as a value, not the `Core` reference.
- Pass an event callback closure (`(Event) -> Void`) for user interactions.
- Views are pure functions of their input -- no direct core access.

## Screen Pattern

Each screen is a standalone SwiftUI view that receives its data and an event
callback. Screens correspond 1:1 to `ViewModel` variants.

```swift
struct MainScreen: View {
    let viewModel: MainView
    let onEvent: (Event) -> Void
    @ObserveInjection var inject

    var body: some View {
        NavigationStack {
            List(viewModel.items, id: \.id) { item in
                ItemRow(item: item) {
                    onEvent(.toggleItem(item.id))
                }
            }
            .navigationTitle("Items")
            .toolbar {
                Button("Add") {
                    onEvent(.addItem("New Item"))
                }
            }
        }
        .enableInjection()
    }
}
```

## Loading Screen

A simple centered indicator. No data needed from the core.

```swift
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
```

## Error Screen

Displays an error message with an optional retry button.

```swift
struct ErrorScreen: View {
    let viewModel: ErrorView
    let onEvent: (Event) -> Void
    @ObserveInjection var inject

    var body: some View {
        VStack(spacing: VectisSpacing.lg) {
            Image(systemName: "exclamationmark.triangle")
                .font(.system(size: 48))
                .foregroundStyle(VectisColors.error)

            Text(viewModel.message)
                .font(VectisTypography.body)
                .foregroundStyle(VectisColors.onSurface)
                .multilineTextAlignment(.center)
                .padding(.horizontal, VectisSpacing.xl)

            if viewModel.canRetry {
                Button("Try Again") {
                    onEvent(.navigate(.main))
                }
                .buttonStyle(.borderedProminent)
                .tint(VectisColors.primary)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(VectisColors.surface)
        .enableInjection()
    }
}
```

## List Rendering

Use SwiftUI `List` or `ForEach` with the view model's item array.

```swift
List(viewModel.items, id: \.id) { item in
    HStack {
        Image(systemName: item.completed ? "checkmark.circle.fill" : "circle")
            .foregroundStyle(
                item.completed ? VectisColors.primary : VectisColors.onSurfaceSecondary
            )
        Text(item.title)
            .font(VectisTypography.body)
            .strikethrough(item.completed)
    }
    .contentShape(Rectangle())
    .onTapGesture {
        onEvent(.toggleItem(item.id))
    }
}
```

## Form Inputs

For text input, use `@State` for the local editing buffer and dispatch an
event on submit.

```swift
struct AddItemSection: View {
    @State private var text = ""
    let onEvent: (Event) -> Void
    @ObserveInjection var inject

    var body: some View {
        HStack(spacing: VectisSpacing.sm) {
            TextField("New item", text: $text)
                .textFieldStyle(.roundedBorder)
                .onSubmit { submit() }

            Button("Add") { submit() }
                .buttonStyle(.borderedProminent)
                .tint(VectisColors.primary)
                .disabled(text.trimmingCharacters(in: .whitespaces).isEmpty)
        }
        .padding(.horizontal, VectisSpacing.md)
        .enableInjection()
    }

    private func submit() {
        let trimmed = text.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        onEvent(.addItem(trimmed))
        text = ""
    }
}
```

## Navigation with Route

When the Crux core defines a `Route` enum, use `NavigationStack` in the app
entry point. Navigation events are dispatched as `Event.navigate(route)`.

```swift
@main
struct MyApp: App {
    @StateObject private var core = Core()

    var body: some Scene {
        WindowGroup {
            ContentView(core: core)
                .vectisTheme()
        }
    }
}
```

For tab-based navigation:

```swift
struct ContentView: View {
    @ObservedObject var core: Core
    @State private var selectedTab: Route = .main
    @ObserveInjection var inject

    var body: some View {
        TabView(selection: $selectedTab) {
            MainScreen(viewModel: ...) { core.update($0) }
                .tabItem { Label("Home", systemImage: "house") }
                .tag(Route.main)

            SettingsScreen(viewModel: ...) { core.update($0) }
                .tabItem { Label("Settings", systemImage: "gear") }
                .tag(Route.settings)
        }
        .onChange(of: selectedTab) { _, newTab in
            core.update(.navigate(newTab))
        }
        .enableInjection()
    }
}
```

## Swipe Actions

```swift
.swipeActions(edge: .trailing, allowsFullSwipe: true) {
    Button(role: .destructive) {
        onEvent(.deleteItem(item.id))
    } label: {
        Label("Delete", systemImage: "trash")
    }
}
```

## Pull-to-Refresh

```swift
List(viewModel.items, id: \.id) { item in
    ItemRow(item: item)
}
.refreshable {
    onEvent(.refresh)
}
```

## Status Indicators

For sync status or connectivity indicators within a page (not a separate
error view):

```swift
if !viewModel.syncStatus.isEmpty {
    Label(viewModel.syncStatus, systemImage: "arrow.triangle.2.circlepath")
        .font(VectisTypography.caption)
        .foregroundStyle(VectisColors.onSurfaceSecondary)
}
```

## Accessibility

- Use `accessibilityLabel` for icons and non-text interactive elements.
- Use `accessibilityHint` for actions that are not obvious.
- Mark decorative images with `accessibilityHidden(true)`.

```swift
Image(systemName: "checkmark.circle.fill")
    .accessibilityLabel(item.completed ? "Completed" : "Not completed")
```

## Preview Provider

Every screen view should have a preview with sample data:

```swift
#Preview {
    MainScreen(
        viewModel: MainView(
            items: [
                ItemView(id: "1", title: "Sample Item", completed: false),
                ItemView(id: "2", title: "Done Item", completed: true),
            ],
            itemCount: "2 items"
        ),
        onEvent: { _ in }
    )
    .vectisTheme()
}
```
