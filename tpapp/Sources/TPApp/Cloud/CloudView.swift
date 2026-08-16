import SwiftUI

struct CloudView: View {
    let me: MeResponse
    @Environment(AuthManager.self) var auth
    @State private var profile: UserProfile?
    @State private var showHome = false
    @State private var selectedTab = 0

    var body: some View {
        TabView(selection: $selectedTab) {
            DashboardTab(me: me, showHome: $showHome)
                .tabItem { Label("Inicio", systemImage: "house.fill") }
                .tag(0)

            ProfileTab(profile: $profile)
                .tabItem { Label("Cuenta", systemImage: "person.fill") }
                .tag(1)

            if profile?.isAdmin == true {
                AdminTab()
                    .tabItem { Label("Admin", systemImage: "shield.fill") }
                    .tag(2)
            }
        }
        .task {
            profile = try? await CloudAPI.profile()
        }
        .sheet(isPresented: $showHome) {
            HomeView()
        }
    }
}

// MARK: - Dashboard Tab

struct DashboardTab: View {
    let me: MeResponse
    @Binding var showHome: Bool
    @Environment(AuthManager.self) var auth

    var body: some View {
        NavigationStack {
            List {
                Section {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Hola, \(me.displayName)")
                            .font(.title2.bold())
                        Text(me.email)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    .padding(.vertical, 4)
                }

                Section("Servicios") {
                    ServiceRow(
                        icon: "house.and.flag.fill",
                        color: .orange,
                        title: "TPHome",
                        subtitle: "Control del hogar"
                    ) {
                        showHome = true
                    }
                }

                Section {
                    Button(role: .destructive) {
                        Task { await auth.logout() }
                    } label: {
                        Label("Cerrar sesión", systemImage: "rectangle.portrait.and.arrow.right")
                    }
                }
            }
            .navigationTitle("TPCloud")
        }
    }
}

// MARK: - Service Row

struct ServiceRow: View {
    let icon: String
    let color: Color
    let title: String
    let subtitle: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 14) {
                Image(systemName: icon)
                    .font(.title3)
                    .foregroundStyle(.white)
                    .frame(width: 36, height: 36)
                    .background(color, in: RoundedRectangle(cornerRadius: 8))
                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(.headline)
                        .foregroundStyle(.primary)
                    Text(subtitle)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
        }
    }
}
