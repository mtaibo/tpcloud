import SwiftUI

// MARK: - Container

struct HomeSettingsView: View {
    @State private var selectedSection = 0

    var body: some View {
        VStack(spacing: 0) {
            Picker("Sección", selection: $selectedSection) {
                Text("Dispositivos").tag(0)
                Text("Firmware").tag(1)
            }
            .pickerStyle(.segmented)
            .padding()

            if selectedSection == 0 {
                DevicesView()
            } else {
                FirmwareView()
            }
        }
        .navigationTitle("Ajustes")
    }
}

// MARK: - Devices

struct DevicesView: View {
    @State private var pending: [PendingDevice] = []

    var body: some View {
        List {
            if !pending.isEmpty {
                Section("Dispositivos pendientes") {
                    ForEach(pending) { device in
                        HStack {
                            Image(systemName: "questionmark.circle")
                                .foregroundStyle(.orange)
                            Text(device.mac).font(.subheadline)
                        }
                    }
                }
            }
        }
        .task { await reload() }
        .refreshable { await reload() }
        .overlay {
            if pending.isEmpty {
                ContentUnavailableView(
                    "Sin dispositivos pendientes",
                    systemImage: "checkmark.circle",
                    description: Text("Todos los dispositivos están configurados")
                )
            }
        }
    }

    private func reload() async {
        pending = (try? await HomeAPI.pendingDevices()) ?? []
    }
}

// MARK: - Firmware

struct FirmwareView: View {
    @State private var firmwares: [FirmwareInfo] = []
    @State private var confirmActivate: FirmwareInfo?
    @State private var confirmDelete: FirmwareInfo?

    var body: some View {
        List(firmwares) { fw in
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(fw.name).font(.headline)
                    if fw.active {
                        Text("ACTIVO")
                            .font(.caption2.bold())
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(.green.opacity(0.15), in: Capsule())
                            .foregroundStyle(.green)
                    }
                }
                HStack {
                    Label(fw.chip, systemImage: "cpu")
                    Text("v\(fw.version)").foregroundStyle(.secondary)
                }
                .font(.caption)
                if let notes = fw.notes, !notes.isEmpty {
                    Text(notes).font(.caption2).foregroundStyle(.tertiary)
                }
            }
            .swipeActions(edge: .trailing) {
                Button(role: .destructive) {
                    confirmDelete = fw
                } label: {
                    Label("Eliminar", systemImage: "trash")
                }
                if !fw.active {
                    Button {
                        confirmActivate = fw
                    } label: {
                        Label("Activar", systemImage: "checkmark")
                    }
                    .tint(.green)
                }
            }
        }
        .task { await reload() }
        .refreshable { await reload() }
        .confirmationDialog(
            "Activar \(confirmActivate?.name ?? "")?",
            isPresented: Binding(
                get: { confirmActivate != nil },
                set: { if !$0 { confirmActivate = nil } }
            ),
            titleVisibility: .visible
        ) {
            Button("Activar") {
                guard let fw = confirmActivate else { return }
                confirmActivate = nil
                Task {
                    try? await HomeAPI.activateFirmware(fw.id)
                    await reload()
                }
            }
            Button("Cancelar", role: .cancel) { confirmActivate = nil }
        }
        .confirmationDialog(
            "Eliminar \(confirmDelete?.name ?? "")?",
            isPresented: Binding(
                get: { confirmDelete != nil },
                set: { if !$0 { confirmDelete = nil } }
            ),
            titleVisibility: .visible
        ) {
            Button("Eliminar", role: .destructive) {
                guard let fw = confirmDelete else { return }
                confirmDelete = nil
                Task {
                    try? await HomeAPI.deleteFirmware(fw.id)
                    await reload()
                }
            }
            Button("Cancelar", role: .cancel) { confirmDelete = nil }
        }
    }

    private func reload() async {
        firmwares = (try? await HomeAPI.firmwareList()) ?? []
    }
}
