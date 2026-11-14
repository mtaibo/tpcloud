import SwiftUI
import TPCloudFeature
import TPCore

private enum MoreDestination: Hashable {
    case account
}

/// The fourth tab: who's signed in, Account and sign out — the screens that used to take up a
/// tab of their own.
struct MoreView: View {
    @ObservedObject var session: AuthSession
    let profile: UserProfile
    let accountClient: CloudAccountClient
    let resetSignal: Int

    @State private var path: [MoreDestination] = []
    @State private var isConfirmingSignOut = false

    var body: some View {
        NavigationStack(path: $path) {
            List {
                Section {
                    HStack(spacing: 14) {
                        Text(initials)
                            .font(.title2.weight(.semibold))
                            .foregroundStyle(.white)
                            .frame(width: 56, height: 56)
                            .background(.tint, in: .circle)
                            .accessibilityHidden(true)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(profile.displayName.isEmpty ? profile.email : profile.displayName)
                                .font(.headline)
                            Text(profile.email)
                                .font(.subheadline)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .padding(.vertical, 4)
                    .accessibilityElement(children: .combine)
                }

                Section {
                    NavigationLink(value: MoreDestination.account) {
                        Label("Account", systemImage: "person.crop.circle")
                    }
                }

                Section {
                    Button("Sign Out", role: .destructive) {
                        isConfirmingSignOut = true
                    }
                } footer: {
                    Text(versionText)
                        .frame(maxWidth: .infinity)
                        .padding(.top, 8)
                }
            }
            .navigationTitle("More")
            .navigationDestination(for: MoreDestination.self) { destination in
                switch destination {
                case .account: AccountView(accountClient: accountClient)
                }
            }
            .confirmationDialog("Sign out of TPCloud?", isPresented: $isConfirmingSignOut, titleVisibility: .visible) {
                Button("Sign Out", role: .destructive) {
                    Task { await session.signOut() }
                }
            }
        }
        .onChange(of: resetSignal) {
            path = []
        }
    }

    private var initials: String {
        let source = profile.displayName.isEmpty ? profile.email : profile.displayName
        let letters = source
            .split(whereSeparator: { $0 == " " || $0 == "." || $0 == "@" })
            .prefix(2)
            .compactMap(\.first)
        return String(letters).uppercased()
    }

    private var versionText: String {
        let info = Bundle.main.infoDictionary
        let version = info?["CFBundleShortVersionString"] as? String ?? "?"
        let build = info?["CFBundleVersion"] as? String ?? "?"
        return "TPCloud \(version) (\(build))"
    }
}
