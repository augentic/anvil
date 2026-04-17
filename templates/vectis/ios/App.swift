import SwiftUI

@main
struct __APP_NAME__App: App {
    @StateObject private var core = Core()

    var body: some Scene {
        WindowGroup {
            ContentView(core: core)
        }
    }
}
