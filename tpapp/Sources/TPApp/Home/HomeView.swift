import SwiftUI

struct HomeView: View {
    @State private var wsManager = WebSocketManager()
    @State private var blinds: [BlindDevice] = []
    @State private var lights: [LightDevice] = []

    var body: some View {
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
        .task {
            // Load devices concurrently
            async let fetchBlinds = HomeAPI.blinds()
            async let fetchLights = HomeAPI.lights()
            blinds = (try? await fetchBlinds) ?? []
            lights = (try? await fetchLights) ?? []

            // Set up WebSocket callbacks before connecting
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
