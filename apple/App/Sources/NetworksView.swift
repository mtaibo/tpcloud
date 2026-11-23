import SwiftUI
import TPCore

/// The networks this app knows, the one in use first. Tapping another moves to it (and to its own
/// session); the one in use is where photos back up.
struct NetworksView: View {
    @ObservedObject var networks: NetworkStore
    @State private var isAdding = false

    var body: some View {
        List {
            Section {
                ForEach(networks.networks) { network in
                    Button {
                        Task { await networks.use(network.id) }
                    } label: {
                        HStack {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(network.name).foregroundStyle(.primary)
                                Text(subtitle(for: network))
                                    .font(.footnote)
                                    .foregroundStyle(.secondary)
                            }
                            Spacer()
                            if network.id == networks.currentID {
                                Image(systemName: "checkmark").foregroundStyle(.tint).fontWeight(.semibold)
                            }
                        }
                    }
                    .swipeActions {
                        Button("Remove", role: .destructive) {
                            Task { await networks.remove(network.id) }
                        }
                    }
                }
            } footer: {
                Text("Photos back up to the network in use.")
            }
            Section {
                Button {
                    isAdding = true
                } label: {
                    Label("Add a Network", systemImage: "plus")
                }
            }
        }
        .navigationTitle("Networks")
        .sheet(isPresented: $isAdding) {
            NavigationStack {
                AddNetworkView(networks: networks) { isAdding = false }
                    .toolbar {
                        ToolbarItem(placement: .cancellationAction) {
                            Button("Cancel", systemImage: "xmark") { isAdding = false }
                        }
                    }
            }
        }
    }

    private func subtitle(for network: SavedNetwork) -> String {
        if network.id == networks.currentID, let url = networks.address {
            return url.host() ?? url.absoluteString
        }
        return network.addresses.first?.url.host() ?? ""
    }
}

/// Adding a network: one announcing itself on this Wi-Fi, an address typed in, or a machine
/// waiting to be set up.
struct AddNetworkView: View {
    @ObservedObject var networks: NetworkStore
    let onDone: () -> Void

    @StateObject private var browser = NetworkBrowser()
    @State private var address = ""
    @State private var busy: String?
    @State private var errorMessage: String?

    var body: some View {
        List {
            let known = Set(networks.networks.map(\.id))
            let found = browser.networks.filter { $0.networkID.map { !known.contains($0) } ?? true }
            Section {
                if found.isEmpty {
                    Label("Looking on this network…", systemImage: "wifi")
                        .foregroundStyle(.secondary)
                }
                ForEach(found) { item in
                    Button {
                        Task { await add(item.url.absoluteString, key: item.id) }
                    } label: {
                        row(item.name, item.host, busy: busy == item.id)
                    }
                    .disabled(busy != nil)
                }
            } header: {
                Text("On This Network")
            }

            Section {
                TextField("tphome.local or cloud.example.com", text: $address)
                    .autocorrectionDisabled()
                    #if os(iOS)
                    .textInputAutocapitalization(.never)
                    .keyboardType(.URL)
                    #endif
                    .onSubmit { Task { await add(address, key: "address") } }
                Button {
                    Task { await add(address, key: "address") }
                } label: {
                    row("Add", nil, busy: busy == "address")
                }
                .disabled(address.trimmingCharacters(in: .whitespaces).isEmpty || busy != nil)
            } header: {
                Text("Address")
            } footer: {
                if let errorMessage {
                    Label(errorMessage, systemImage: "exclamationmark.circle.fill").foregroundStyle(.red)
                }
            }

            if !browser.waiting.isEmpty {
                Section {
                    ForEach(browser.waiting) { item in
                        NavigationLink {
                            SetupHostView(found: item, networks: networks, onDone: onDone)
                        } label: {
                            row(item.name, "\(item.host) · waiting to be set up", busy: false)
                        }
                    }
                } header: {
                    Text("Set Up a New Host")
                } footer: {
                    Text("Machines on this network with TPCloud installed and nothing set up yet.")
                }
            }
        }
        .navigationTitle("Add a Network")
        .onAppear { browser.start() }
        .onDisappear { browser.stop() }
    }

    private func row(_ title: String, _ subtitle: String?, busy: Bool) -> some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                if let subtitle {
                    Text(subtitle).font(.footnote).foregroundStyle(.secondary)
                }
            }
            Spacer()
            if busy { ProgressView() }
        }
    }

    private func add(_ text: String, key: String) async {
        errorMessage = nil
        busy = key
        defer { busy = nil }
        do {
            let loaded = try await NetworkCardLoader.load(text)
            let reached = NetworkCardLoader.normalized(text) ?? loaded.card.addresses.first!.url
            await networks.add(loaded, reachedAt: reached)
            onDone()
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}

/// The first screen when the app knows no network yet.
struct WelcomeView: View {
    @ObservedObject var networks: NetworkStore

    var body: some View {
        NavigationStack {
            AddNetworkView(networks: networks) {}
                .safeAreaInset(edge: .top) {
                    VStack(spacing: 10) {
                        Image(systemName: "cloud.fill")
                            .font(.system(size: 52))
                            .foregroundStyle(.tint)
                            .accessibilityHidden(true)
                        Text("TPCloud").font(.largeTitle.bold())
                        Text("Add your network: the one at home, or one at its address.")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                            .multilineTextAlignment(.center)
                    }
                    .padding(.vertical, 20)
                    .frame(maxWidth: .infinity)
                    .background(.background)
                }
                .navigationTitle("")
        }
    }
}
