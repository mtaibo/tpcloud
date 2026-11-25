import SwiftUI
import TPCloudFeature
import TPCore

private enum MoreDestination: Hashable {
    case account
    case networks
    case server
    case terminal
}

/// The fourth tab: who's signed in, Account and sign out — the screens that used to take up a
/// tab of their own.
struct MoreView: View {
    @ObservedObject var networks: NetworkStore
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
                    if profile.isAdmin {
                        NavigationLink(value: MoreDestination.server) {
                            Label("Server", systemImage: "server.rack")
                        }
                    }
                    NavigationLink(value: MoreDestination.terminal) {
                        Label("Terminal", systemImage: "terminal")
                    }
                    NavigationLink(value: MoreDestination.networks) {
                        LabeledContent {
                            Text(networks.current?.name ?? "")
                        } label: {
                            Label("Network", systemImage: "network")
                        }
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
                case .networks: NetworksView(networks: networks)
                case .server: ServerView(accountClient: accountClient)
                case .terminal: TerminalView()
                }
            }
            .confirmationDialog("Sign out of TPCloud?", isPresented: $isConfirmingSignOut, titleVisibility: .visible) {
                Button("Sign Out", role: .destructive) {
                    Task {
                        await FinderLocations.remove(networks.currentID)
                        await session.signOut()
                    }
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

/// The `tpcloud` command for a Mac or Linux terminal: the same one-line install as the website's
/// (this app can't put a command in the terminal by itself), so every way ends in one tpcloud.
struct TerminalView: View {
    static let command = "curl -fsSL https://tpcloud.migueltaibo.com/get | sh"
    @State private var copied = false

    var body: some View {
        List {
            Section {
                Text(Self.command)
                    .font(.callout.monospaced())
                    .textSelection(.enabled)
                Button {
                    #if os(macOS)
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(Self.command, forType: .string)
                    #else
                    UIPasteboard.general.string = Self.command
                    #endif
                    copied = true
                } label: {
                    Label(copied ? "Copied" : "Copy", systemImage: copied ? "checkmark" : "doc.on.doc")
                }
            } footer: {
                Text("Paste it in Terminal on a Mac or a Linux computer. It installs the tpcloud command (and, on Linux, the TPCloud app); running it again updates it. Then: tpcloud login.")
            }
            Section("What it does") {
                LabeledContent("tpcloud login", value: "sign in")
                LabeledContent("tpcloud users", value: "accounts (admins)")
                LabeledContent("tpcloud host update", value: "update the server")
                LabeledContent("tphome", value: "the house")
            }
        }
        .navigationTitle("Terminal")
    }
}
