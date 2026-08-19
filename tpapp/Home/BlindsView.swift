import SwiftUI

struct BlindsView: View {
    @Binding var blinds: [BlindDevice]

    var body: some View {
        List(blinds) { blind in
            BlindCard(blind: blind)
        }
        .navigationTitle("Persianas")
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    Task { try? await HomeAPI.adminUpdate() }
                } label: {
                    Image(systemName: "arrow.triangle.2.circlepath")
                }
            }
        }
        .refreshable {
            blinds = (try? await HomeAPI.blinds()) ?? []
        }
        .overlay {
            if blinds.isEmpty {
                ContentUnavailableView(
                    "Sin persianas",
                    systemImage: "blinds.horizontal.closed",
                    description: Text("No hay dispositivos configurados")
                )
            }
        }
    }
}

// MARK: - BlindCard

struct BlindCard: View {
    let blind: BlindDevice
    @State private var sliderValue: Double = 0

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text(blind.id).font(.headline)
                    Text(blind.mac).font(.caption).foregroundStyle(.secondary)
                }
                Spacer()
                HStack(spacing: 4) {
                    Circle()
                        .fill(blind.online ? Color.green : Color.red)
                        .frame(width: 10, height: 10)
                    Text(blind.online ? "Online" : "Offline")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }

            // Position indicator
            if let pos = blind.position {
                VStack(spacing: 4) {
                    HStack {
                        Text("Posición").font(.caption).foregroundStyle(.secondary)
                        Spacer()
                        Text("\(pos)%").font(.caption.bold())
                    }
                    ProgressView(value: Double(pos), total: 100)
                        .tint(.blue)
                }
            }

            // Command buttons
            HStack(spacing: 12) {
                CommandButton(icon: "chevron.up", label: "Subir", color: .blue) {
                    try? await HomeAPI.commandUp(blind.id)
                }
                CommandButton(icon: "stop.fill", label: "Stop", color: .orange) {
                    try? await HomeAPI.commandStop(blind.id)
                }
                CommandButton(icon: "chevron.down", label: "Bajar", color: .blue) {
                    try? await HomeAPI.commandDown(blind.id)
                }
            }

            // Position slider
            HStack {
                Text("0%").font(.caption2).foregroundStyle(.secondary)
                Slider(value: $sliderValue, in: 0...100, step: 1)
                    .onAppear { sliderValue = Double(blind.position ?? 0) }
                    .onChange(of: blind.position) { _, newVal in
                        sliderValue = Double(newVal ?? 0)
                    }
                Text("100%").font(.caption2).foregroundStyle(.secondary)
                Button("Ir") {
                    Task { try? await HomeAPI.commandSet(blind.id, position: Int(sliderValue)) }
                }
                .buttonStyle(.borderedProminent)
                .buttonBorderShape(.capsule)
                .controlSize(.mini)
            }
        }
        .padding(.vertical, 4)
    }
}

// MARK: - CommandButton

struct CommandButton: View {
    let icon: String
    let label: String
    let color: Color
    let action: () async -> Void
    @State private var isLoading = false

    var body: some View {
        Button {
            isLoading = true
            Task {
                await action()
                try? await Task.sleep(for: .milliseconds(300))
                isLoading = false
            }
        } label: {
            VStack(spacing: 4) {
                Image(systemName: isLoading ? "circle.dotted" : icon)
                    .font(.title3)
                Text(label).font(.caption2)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 8)
            .background(color.opacity(0.12), in: RoundedRectangle(cornerRadius: 10))
            .foregroundStyle(color)
        }
        .disabled(isLoading)
    }
}
