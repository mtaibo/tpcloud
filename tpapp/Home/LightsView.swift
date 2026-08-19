import SwiftUI

struct LightsView: View {
    @Binding var lights: [LightDevice]

    var body: some View {
        List($lights) { $light in
            LightRow(light: $light)
        }
        .navigationTitle("Luces")
        .refreshable {
            lights = (try? await HomeAPI.lights()) ?? []
        }
    }
}

// MARK: - LightRow

struct LightRow: View {
    @Binding var light: LightDevice

    var body: some View {
        HStack {
            Image(systemName: light.on ? "lightbulb.fill" : "lightbulb")
                .font(.title3)
                .foregroundStyle(light.on ? .yellow : .secondary)
            VStack(alignment: .leading, spacing: 2) {
                Text(light.id).font(.subheadline)
                Text(light.mac).font(.caption).foregroundStyle(.secondary)
            }
            Spacer()
            HStack(spacing: 4) {
                Circle()
                    .fill(light.online ? Color.green : Color.red)
                    .frame(width: 8, height: 8)
                Text(light.online ? "Online" : "Offline")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
    }
}
