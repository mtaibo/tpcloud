import SwiftUI

struct RootView: View {
    @Environment(AuthManager.self) var auth

    var body: some View {
        switch auth.state {
        case .loading:
            ProgressView("Cargando...")
                .task { await auth.checkSession() }
        case .unauthenticated:
            LoginView()
        case .authenticated(let me):
            CloudView(me: me)
        }
    }
}
