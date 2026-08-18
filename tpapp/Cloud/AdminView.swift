import SwiftUI

// MARK: - Admin Tab Container

struct AdminTab: View {
    @State private var selectedSection = 0

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                Picker("Sección", selection: $selectedSection) {
                    Text("Usuarios").tag(0)
                    Text("Sesiones").tag(1)
                    Text("Invitaciones").tag(2)
                }
                .pickerStyle(.segmented)
                .padding()

                switch selectedSection {
                case 0: AdminUsersView()
                case 1: AdminSessionsView()
                default: AdminInvitesView()
                }
            }
            .navigationTitle("Administración")
        }
    }
}

// MARK: - Users

struct AdminUsersView: View {
    @State private var users: [AdminUser] = []
    @State private var confirmDelete: AdminUser?

    var body: some View {
        List(users) { user in
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(user.displayName).font(.headline)
                    if user.isAdmin {
                        Image(systemName: "shield.fill")
                            .foregroundStyle(.blue)
                            .font(.caption)
                    }
                }
                Text(user.email).font(.caption).foregroundStyle(.secondary)
                HStack {
                    Label("\(user.passkeyCount) passkeys", systemImage: "key")
                    Spacer()
                    Label("\(user.activeSessionCount) sesiones", systemImage: "person.crop.circle")
                }
                .font(.caption2)
                .foregroundStyle(.secondary)
            }
            .swipeActions(edge: .trailing) {
                Button(role: .destructive) {
                    confirmDelete = user
                } label: {
                    Label("Eliminar", systemImage: "trash")
                }
                Button {
                    Task { await toggleAdmin(user.id) }
                } label: {
                    Label("Toggle Admin", systemImage: "shield")
                }
                .tint(.blue)
            }
        }
        .task { await reload() }
        .refreshable { await reload() }
        .confirmationDialog(
            "Eliminar \(confirmDelete?.email ?? "")?",
            isPresented: Binding(
                get: { confirmDelete != nil },
                set: { if !$0 { confirmDelete = nil } }
            ),
            titleVisibility: .visible
        ) {
            Button("Eliminar", role: .destructive) {
                guard let u = confirmDelete else { return }
                confirmDelete = nil
                Task { await deleteUser(u.id) }
            }
            Button("Cancelar", role: .cancel) { confirmDelete = nil }
        }
    }

    private func reload() async {
        users = (try? await CloudAPI.adminUsers()) ?? []
    }

    private func toggleAdmin(_ id: Int) async {
        try? await CloudAPI.adminToggleAdmin(id)
        await reload()
    }

    private func deleteUser(_ id: Int) async {
        try? await CloudAPI.adminDeleteUser(id)
        await reload()
    }
}

// MARK: - Sessions

struct AdminSessionsView: View {
    @State private var sessions: [AdminSession] = []

    var body: some View {
        List(sessions) { s in
            VStack(alignment: .leading, spacing: 4) {
                Text(s.userEmail).font(.subheadline)
                Text(s.ipAddress ?? "IP desconocida").font(.caption).foregroundStyle(.secondary)
                Text("Caduca: \(s.expiresAt.prefix(16).replacingOccurrences(of: "T", with: " "))")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
            .swipeActions {
                Button(role: .destructive) {
                    Task { await revoke(s.sessionId) }
                } label: {
                    Label("Revocar", systemImage: "xmark")
                }
            }
        }
        .task { await reload() }
        .refreshable { await reload() }
    }

    private func reload() async {
        sessions = (try? await CloudAPI.adminSessions()) ?? []
    }

    private func revoke(_ id: String) async {
        try? await CloudAPI.adminRevokeSession(id)
        await reload()
    }
}

// MARK: - Invites

struct AdminInvitesView: View {
    @State private var invites: [Invite] = []
    @State private var showingAddInvite = false
    @State private var newEmail = ""

    var body: some View {
        List(invites, id: \.email) { invite in
            InviteRow(invite: invite, onRevoke: {
                Task { await revokeInvite(invite.email) }
            })
        }
        .task { await reload() }
        .refreshable { await reload() }
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button { showingAddInvite = true } label: {
                    Image(systemName: "plus")
                }
            }
        }
        .alert("Nueva invitación", isPresented: $showingAddInvite) {
            TextField("Email", text: $newEmail)
                .keyboardType(.emailAddress)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
            Button("Invitar") { Task { await createInvite() } }
            Button("Cancelar", role: .cancel) { newEmail = "" }
        }
    }

    private func reload() async {
        invites = (try? await CloudAPI.adminInvites()) ?? []
    }

    private func createInvite() async {
        guard !newEmail.isEmpty else { return }
        try? await CloudAPI.adminCreateInvite(email: newEmail)
        newEmail = ""
        await reload()
    }

    private func revokeInvite(_ email: String) async {
        try? await CloudAPI.adminRevokeInvite(email)
        await reload()
    }
}

struct InviteRow: View {
    let invite: Invite
    let onRevoke: () -> Void

    var body: some View {
        HStack {
            VStack(alignment: .leading) {
                Text(invite.email).font(.subheadline)
                Text(invite.used ? "Usada" : "Pendiente")
                    .font(.caption)
                    .foregroundStyle(invite.used ? Color.secondary : Color.green)
            }
            Spacer()
            if invite.used {
                Image(systemName: "checkmark.circle.fill").foregroundStyle(Color.green)
            }
        }
        .swipeActions {
            if !invite.used {
                Button(role: .destructive, action: onRevoke) {
                    Label("Revocar", systemImage: "xmark")
                }
            }
        }
    }
}
