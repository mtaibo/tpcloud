import SwiftUI
import TPCore

/// The server's own version and updating it from here (admins): it downloads the new TPCloud,
/// checks it against GitHub's checksum, installs it and restarts by itself.
public struct ServerView: View {
    let accountClient: CloudAccountClient

    @State private var host: HostStatus?
    @State private var busy = false
    @State private var confirming = false
    @State private var errorMessage: String?

    public init(accountClient: CloudAccountClient) {
        self.accountClient = accountClient
    }

    public var body: some View {
        List {
            if let host {
                Section {
                    LabeledContent("Version", value: host.version)
                    if let latest = host.latest { LabeledContent("Newest", value: latest) }
                    if !host.isUpdating {
                        Button {
                            Task { await check() }
                        } label: {
                            HStack {
                                Text("Look for Updates")
                                Spacer()
                                if busy { ProgressView() }
                            }
                        }
                        .disabled(busy)
                    }
                } footer: {
                    Text(status(host))
                }

                if host.isUpdating {
                    Section {
                        HStack {
                            Text(step(host.update?.state))
                            Spacer()
                            ProgressView()
                        }
                    } footer: {
                        Text("The server restarts by itself at the end; this carries on once it's back.")
                    }
                } else if host.available && host.canUpdate, let latest = host.latest {
                    Section {
                        Button("Update to \(latest)") { confirming = true }
                            .disabled(busy)
                    } footer: {
                        Text("It downloads the new version, checks it against GitHub's checksum and installs it. Accounts, files and settings stay as they are.")
                    }
                }
            } else if errorMessage == nil {
                ProgressView().frame(maxWidth: .infinity)
            }
            if let errorMessage {
                Text(errorMessage).foregroundStyle(.red).font(.footnote)
            }
        }
        .navigationTitle("Server")
        .confirmationDialog("Update to TPCloud \(host?.latest ?? "")?", isPresented: $confirming, titleVisibility: .visible) {
            Button("Update") { Task { await update() } }
        } message: {
            Text("Everything stops for a few seconds while it restarts. Nothing is lost.")
        }
        .task { await follow() }
        .refreshable { await load() }
    }

    private func status(_ h: HostStatus) -> String {
        switch h.update?.state {
        case "done": "Updated to \(h.update?.detail ?? h.version)."
        case "failed": "The update failed: \(h.update?.detail ?? "")"
        default: h.available ? "TPCloud \(h.latest ?? "") is out." : "This is the latest version."
        }
    }

    private func step(_ state: String?) -> String {
        switch state {
        case "downloading": "Downloading…"
        case "installing": "Installing and restarting…"
        default: "Looking for the new version…"
        }
    }

    private func load() async {
        do {
            host = try await accountClient.host()
            errorMessage = nil
        } catch {
            // While it restarts the server doesn't answer: keep waiting.
            if host?.isUpdating != true { errorMessage = apiErrorMessage(error) }
        }
    }

    /// Loads, and keeps following while an update runs.
    private func follow() async {
        await load()
        while !Task.isCancelled, host?.isUpdating == true {
            try? await Task.sleep(for: .seconds(1.5))
            await load()
        }
    }

    private func check() async {
        busy = true
        defer { busy = false }
        do {
            try await accountClient.checkHost()
            try? await Task.sleep(for: .seconds(2.5))
            await load()
        } catch {
            errorMessage = apiErrorMessage(error)
        }
    }

    private func update() async {
        busy = true
        do {
            try await accountClient.updateHost()
            busy = false
            await follow()
        } catch {
            busy = false
            errorMessage = apiErrorMessage(error)
        }
    }
}
