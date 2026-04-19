import SharedTypes
import SwiftUI

struct HomeScreen: View {
    let viewModel: HomeView
    let onEvent: (Event) -> Void

    var body: some View {
        VStack(spacing: 24) {
            Spacer()
            Text(viewModel.message)
                .font(.title)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

#Preview {
    HomeScreen(
        viewModel: HomeView(message: "Hello from Counter"),
        onEvent: { _ in }
    )
}
