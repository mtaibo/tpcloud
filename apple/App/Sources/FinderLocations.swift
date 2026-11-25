import Foundation
import TPCore
#if os(macOS)
import AppKit
import FileProvider
import os
#endif

/// TPCloud's locations in the Finder's sidebar: one per network this Mac is signed in to (the
/// file provider extension does the rest). Nothing on the iPhone yet.
enum FinderLocations {
    #if os(macOS)
    private static let log = Logger(subsystem: "com.migueltaibo.tpcloud", category: "FinderLocations")

    /// Signed in: the network's location appears (or stays), named after the network.
    static func signedIn(_ network: SavedNetwork, email: String) async {
        TPShared.defaults.set(email, forKey: "tpcloud.fileprovider.email.\(network.id)")
        let domain = NSFileProviderDomain(identifier: NSFileProviderDomainIdentifier(network.id), displayName: network.name)
        do {
            try await NSFileProviderManager.add(domain)
            refresh()
        } catch {
            log.error("Couldn't add the Finder location for \(network.id, privacy: .public): \(error.localizedDescription, privacy: .public)")
        }
    }

    /// Signed out or forgotten: its location goes, with what the Mac had downloaded of it.
    static func remove(_ networkID: String?) async {
        guard let networkID else { return }
        TPShared.defaults.removeObject(forKey: "tpcloud.fileprovider.email.\(networkID)")
        let domains = (try? await NSFileProviderManager.domains()) ?? []
        for domain in domains where domain.identifier.rawValue == networkID {
            try? await NSFileProviderManager.remove(domain)
        }
    }

    /// Locations of networks this app no longer has (removed while it couldn't say so).
    static func keep(only networks: [SavedNetwork]) async {
        let ids = Set(networks.map(\.id))
        for domain in (try? await NSFileProviderManager.domains()) ?? [] where !ids.contains(domain.identifier.rawValue) {
            try? await NSFileProviderManager.remove(domain)
        }
    }

    /// Whether macOS lets the locations run: a new File Provider stays off until the person turns
    /// it on in System Settings, and until then the Finder shows a folder that never opens.
    static func isEnabled() async -> Bool {
        let domains = (try? await NSFileProviderManager.domains()) ?? []
        return domains.isEmpty || domains.allSatisfy(\.userEnabled)
    }

    /// System Settings at Login Items & Extensions, where File Providers are turned on.
    static func openSettings() {
        if let url = URL(string: "x-apple.systempreferences:com.apple.LoginItems-Settings.extension") {
            NSWorkspace.shared.open(url)
        }
    }

    /// Asks every location to look for changes on its host.
    static func refresh() {
        Task {
            for domain in (try? await NSFileProviderManager.domains()) ?? [] {
                try? await NSFileProviderManager(for: domain)?.signalEnumerator(for: .workingSet)
            }
        }
    }
    #else
    static func isEnabled() async -> Bool { true }
    static func openSettings() {}
    static func signedIn(_ network: SavedNetwork, email: String) async {}
    static func remove(_ networkID: String?) async {}
    static func keep(only networks: [SavedNetwork]) async {}
    static func refresh() {}
    #endif
}

#if os(macOS)
import SwiftUI

/// Over the Store on a Mac while TPCloud's Finder location is off in System Settings.
struct FinderLocationBanner: View {
    @State private var enabled = true

    var body: some View {
        Group {
            if !enabled {
                HStack(spacing: 12) {
                    Image(systemName: "externaldrive.badge.icloud")
                        .font(.title2)
                        .foregroundStyle(.tint)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Show your files in the Finder").font(.headline)
                        Text("Turn on TPCloud under File Providers in System Settings, and your files appear in the Finder's sidebar.")
                            .font(.callout)
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                    Button("Open System Settings") { FinderLocations.openSettings() }
                        .buttonStyle(.glassProminent)
                }
                .padding(14)
                .glassEffect(.regular, in: .rect(cornerRadius: 16))
                .padding([.horizontal, .top], 16)
            }
        }
        .task {
            while !Task.isCancelled {
                enabled = await FinderLocations.isEnabled()
                try? await Task.sleep(for: .seconds(enabled ? 30 : 2))
            }
        }
    }
}
#endif
