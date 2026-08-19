import SwiftUI

struct HomeView: View {
    let me: MeResponse
    @Environment(\.dismiss) private var dismiss
    @Environment(AuthManager.self) private var auth
    @State private var wsManager = WebSocketManager()
    @State private var blinds: [BlindDevice] = []
    @State private var lights: [LightDevice] = []

    var body: some View {
        NavigationStack {
            TabView {
                BlindsView(blinds: $blinds)
                    .tabItem { Label("Persianas", systemImage: "blinds.horizontal.closed") }

                if !lights.isEmpty {
                    LightsView(lights: $lights)
                        .tabItem { Label("Luces", systemImage: "lightbulb.fill") }
                }

                HomeSettingsView()
                    .tabItem { Label("Ajustes", systemImage: "gearshape.fill") }
            }
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Menu {
                        Button { dismiss() } label: {
                            Label("TPCloud", systemImage: "cloud.fill")
                        }
                        Button(role: .destructive) {
                            Task { await auth.logout() }
                        } label: {
                            Label("Cerrar sesión", systemImage: "rectangle.portrait.and.arrow.right")
                        }
                    } label: {
                        HStack(spacing: 6) {
                            Text(String(me.displayName.prefix(1)))
                                .font(.caption.bold())
                                .foregroundStyle(.white)
                                .frame(width: 28, height: 28)
                                .background(.blue, in: Circle())
                            Text(me.displayName)
                                .font(.subheadline)
                                .foregroundStyle(.primary)
                        }
                    }
                }
            }
            .task {
                async let fetchBlinds = HomeAPI.blinds()
                async let fetchLights = HomeAPI.lights()
                blinds = (try? await fetchBlinds) ?? []
                lights = (try? await fetchLights) ?? []

                wsManager.onBlindStateUpdate = { id, pos, motor in
                    if let i = blinds.firstIndex(where: { $0.id == id }) {
                        blinds[i] = blinds[i].withPosition(pos, motorState: motor)
                    }
                }
                wsManager.onDeviceOnline = { id, online in
                    if let i = blinds.firstIndex(where: { $0.id == id }) {
                        blinds[i] = blinds[i].withOnline(online)
                    }
                }

                wsManager.connect()
            }
            .onDisappear {
                wsManager.disconnect()
            }
        }
    }
}
