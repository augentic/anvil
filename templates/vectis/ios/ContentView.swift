import SharedTypes
import SwiftUI

struct ContentView: View {
    @ObservedObject var core: Core

    var body: some View {
        switch core.view {
        case .loading:
            LoadingScreen()
        case .home(let viewModel):
            HomeScreen(viewModel: viewModel) { event in
                core.update(event)
            }
        }
    }
}
