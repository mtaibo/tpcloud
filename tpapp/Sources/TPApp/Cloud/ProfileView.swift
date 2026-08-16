import SwiftUI

struct ProfileTab: View {
    @Binding var profile: UserProfile?
    @State private var isEditingName = false
    @State private var newName = ""

    var body: some View {
        NavigationStack {
            Group {
                if let p = profile {
                    List {
                        Section("Perfil") {
                            LabeledContent("Nombre", value: p.displayName)
                            LabeledContent("Email", value: p.email)
                            LabeledContent("Rol", value: p.isAdmin ? "Admin" : "Usuario")
                            Button("Cambiar nombre") {
                                newName = p.displayName
                                isEditingName = true
                            }
                        }

                        Section("Passkeys") {
                            ForEach(p.passkeys) { key in
                                PasskeyRow(key: key, canDelete: p.passkeys.count > 1) {
                                    await deletePasskey(key.credentialId)
                                }
                            }
                        }

                        Section("Sesiones activas") {
                            ForEach(p.sessions) { session in
                                SessionRow(session: session) {
                                    await revokeSession(session.sessionId)
                                }
                            }
                        }
                    }
                } else {
                    ProgressView()
                }
            }
            .navigationTitle("Mi cuenta")
            .refreshable { await reload() }
            .alert("Cambiar nombre", isPresented: $isEditingName) {
                TextField("Nombre", text: $newName)
                Button("Guardar") { Task { await saveName() } }
                Button("Cancelar", role: .cancel) {}
            }
            .task { await reload() }
        }
    }

    private func reload() async {
        profile = try? await CloudAPI.profile()
    }

    private func saveName() async {
        guard !newName.isEmpty else { return }
        try? await CloudAPI.updateDisplayName(newName)
        await reload()
    }

    private func deletePasskey(_ id: String) async {
        try? await CloudAPI.deletePasskey(id)
        await reload()
    }

    private func revokeSession(_ id: String) async {
        try? await CloudAPI.revokeMySession(id)
        await reload()
    }
}

// MARK: - PasskeyRow

struct PasskeyRow: View {
    let key: PasskeyItem
    let canDelete: Bool
    let onDelete: () async -> Void

    var body: some View {
        HStack {
            Image(systemName: "key.fill")
                .foregroundStyle(.blue)
            VStack(alignment: .leading, spacing: 2) {
                Text(key.deviceName)
                    .font(.subheadline)
                Text(String(key.createdAt.prefix(10)))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
        }
        .swipeActions(edge: .trailing, allowsFullSwipe: false) {
            if canDelete {
                Button(role: .destructive) {
                    Task { await onDelete() }
                } label: {
                    Label("Eliminar", systemImage: "trash")
                }
            }
        }
    }
}

// MARK: - SessionRow

struct SessionRow: View {
    let session: SessionItem
    let onRevoke: () async -> Void

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                HStack {
                    Text(session.ipAddress ?? "IP desconocida")
                        .font(.subheadline)
                    if session.current {
                        Text("ACTUAL")
                            .font(.caption2.bold())
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(.blue.opacity(0.15), in: Capsule())
                            .foregroundStyle(.blue)
                    }
                }
                Text("Caduca: \(session.expiresAt.prefix(16).replacingOccurrences(of: "T", with: " "))")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
        }
        .swipeActions(edge: .trailing, allowsFullSwipe: false) {
            if !session.current {
                Button(role: .destructive) {
                    Task { await onRevoke() }
                } label: {
                    Label("Revocar", systemImage: "xmark")
                }
            }
        }
    }
}
