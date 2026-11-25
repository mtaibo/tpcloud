import Foundation
import Security

/// A network the app knows: its card as last read, and the certificate authority it's pinned to
/// when one of its addresses uses the host's own.
public struct SavedNetwork: Codable, Identifiable, Equatable, Sendable {
    public var id: String
    public var name: String
    public var addresses: [NetworkCard.Address]
    /// DER of the host's own certificate authority, for its local addresses.
    public var anchor: Data?
    /// The address it was reached at last.
    public var lastAddress: URL?

    public init(id: String, name: String, addresses: [NetworkCard.Address], anchor: Data?, lastAddress: URL?) {
        self.id = id
        self.name = name
        self.addresses = addresses
        self.anchor = anchor
        self.lastAddress = lastAddress
    }

    /// The networks saved on this device (the app's list, shared with its extensions on the Mac).
    public static func saved(in defaults: UserDefaults = TPShared.defaults) -> [SavedNetwork] {
        guard let data = defaults.data(forKey: NetworkStore.networksKey) else { return [] }
        return (try? JSONDecoder().decode([SavedNetwork].self, from: data)) ?? []
    }

    /// Host → the host's own certificate authority, for each of its local addresses.
    var anchors: [String: SecCertificate] {
        guard let der = anchor, let cert = SecCertificateCreateWithData(nil, der as CFData) else { return [:] }
        var map: [String: SecCertificate] = [:]
        for address in addresses where address.kind == "local" {
            if let host = address.url.host() { map[host.lowercased()] = cert }
        }
        return map
    }
}

/// The networks this app has been added to, which one is in use, and how it's reached. One is
/// in use at a time (photo backup goes to it); each keeps its own session in the Keychain and its
/// own photo index and backup ledger, swapped in when the app moves to it.
@MainActor
public final class NetworkStore: ObservableObject {
    @Published public private(set) var networks: [SavedNetwork] = []
    @Published public private(set) var currentID: String?
    @Published public private(set) var address: URL?
    /// The version the network in use says it runs (from its card), once read.
    @Published public private(set) var hostVersion: String?

    public var current: SavedNetwork? { networks.first { $0.id == currentID } }

    private let defaults: UserDefaults
    private let tokens = SessionTokenStore()
    nonisolated static let networksKey = "tpcloud.networks"
    nonisolated static let currentKey = "tpcloud.network.current"
    /// Cloud address of the app before it had networks: its session becomes this network's.
    public static let legacyAddress = URL(string: "https://cloud.migueltaibo.com")!

    public init(defaults: UserDefaults = TPShared.defaults) {
        TPShared.migrateDefaults([Self.networksKey, Self.currentKey, TPConnection.addressKey])
        self.defaults = defaults
        if let data = defaults.data(forKey: Self.networksKey),
           let saved = try? JSONDecoder().decode([SavedNetwork].self, from: data) {
            networks = saved
        }
        currentID = defaults.string(forKey: Self.currentKey)
        let tokens = tokens
        TPConnection.shared.onTokenChange = { [weak self] token in
            Task { @MainActor in
                guard let id = self?.currentID else { return }
                tokens.save(token, for: id)
            }
        }
    }

    /// At launch: carries the old single-server app over, then connects to the network in use.
    public func start() async {
        migrateLegacyIfNeeded()
        guard let id = currentID else { return }
        await connect(id)
        Task { await refreshCard() }
    }

    /// Adds a network (or updates its card) and moves to it.
    @discardableResult
    public func add(_ loaded: NetworkCardLoader.Loaded, reachedAt url: URL) async -> SavedNetwork {
        var network = SavedNetwork(id: loaded.card.id, name: loaded.card.name, addresses: loaded.card.addresses, anchor: loaded.anchor, lastAddress: url)
        if let i = networks.firstIndex(where: { $0.id == network.id }) {
            network.anchor = loaded.anchor ?? networks[i].anchor
            networks[i] = network
        } else {
            networks.append(network)
        }
        save()
        await use(network.id)
        return network
    }

    /// Moves to another network: its session, its address, its photo index and backup ledger.
    public func use(_ id: String) async {
        guard networks.contains(where: { $0.id == id }) else { return }
        if let old = currentID, old != id {
            NetworkData.stash(for: old)
            NetworkData.restore(for: id)
        }
        currentID = id
        defaults.set(id, forKey: Self.currentKey)
        await connect(id)
    }

    public func remove(_ id: String) async {
        tokens.save(nil, for: id)
        NetworkData.forget(id)
        networks.removeAll { $0.id == id }
        save()
        if currentID == id {
            currentID = nil
            defaults.removeObject(forKey: Self.currentKey)
            if let next = networks.first {
                NetworkData.restore(for: next.id)
                await use(next.id)
            } else {
                TPConnection.shared.use(TPServer(url: Self.legacyAddress), token: nil, anchors: [:])
            }
        }
    }

    /// Picks the fastest address that answers as this network (home's first, then Tailscale's,
    /// then the public one) and points every request at it.
    public func connect(_ id: String) async {
        guard let network = networks.first(where: { $0.id == id }) else { return }
        let anchors = network.anchors
        let token = tokens.load(for: id)
        let fallback = network.lastAddress ?? network.addresses.first?.url ?? Self.legacyAddress
        // Probes go through the shared session, which needs the anchors to reach local addresses.
        TPConnection.shared.use(TPServer(url: fallback), token: token, anchors: anchors)
        let reached = await Self.fastest(network)
        let url = reached ?? fallback
        TPConnection.shared.use(TPServer(url: url), token: token, anchors: anchors)
        address = url
        if reached != nil, let i = networks.firstIndex(where: { $0.id == id }), networks[i].lastAddress != url {
            networks[i].lastAddress = url
            save()
        }
    }

    /// Reads the card again from where it's reached, for new addresses or a new name.
    public func refreshCard() async {
        guard let id = currentID, let url = address, let loaded = try? await NetworkCardLoader.load(url.absoluteString) else { return }
        guard let i = networks.firstIndex(where: { $0.id == id }) else { return }
        if loaded.card.id != id {
            // The app's first network, from before cards, now that its server says its id.
            guard id.hasPrefix("legacy:") else { return }
            tokens.save(tokens.load(for: id), for: loaded.card.id)
            tokens.save(nil, for: id)
            networks[i].id = loaded.card.id
            currentID = loaded.card.id
            defaults.set(loaded.card.id, forKey: Self.currentKey)
        }
        hostVersion = loaded.card.version
        networks[i].name = loaded.card.name
        networks[i].addresses = loaded.card.addresses
        if let anchor = loaded.anchor { networks[i].anchor = anchor }
        save()
    }

    nonisolated private static func priority(_ kind: String) -> Int {
        switch kind {
        case "local": 0
        case "tailscale": 1
        default: 2
        }
    }

    /// Asks every address at once; the best kind that answers within a moment wins.
    nonisolated public static func fastest(_ network: SavedNetwork, session: URLSession = TPConnection.shared.session) async -> URL? {
        let candidates = network.addresses.sorted { priority($0.kind) < priority($1.kind) }
        return await withTaskGroup(of: (Int, URL)?.self) { group in
            for (rank, address) in candidates.enumerated() {
                group.addTask {
                    var request = URLRequest(url: address.url.appending(path: ".well-known/tpcloud"))
                    request.timeoutInterval = 2.5
                    request.cachePolicy = .reloadIgnoringLocalCacheData
                    guard let (data, response) = try? await session.data(for: request),
                          (response as? HTTPURLResponse)?.statusCode == 200,
                          let card = try? JSONDecoder().decode(IDOnly.self, from: data),
                          card.id == network.id || network.id.hasPrefix("legacy:") else { return nil }
                    return (rank, address.url)
                }
            }
            var best: (Int, URL)?
            for await result in group {
                guard let result else { continue }
                if best == nil || result.0 < best!.0 { best = result }
                if result.0 == 0 { group.cancelAll() }
            }
            return best?.1
        }
    }

    private struct IDOnly: Decodable { let id: String }

    /// The app before networks talked to one server with a cookie session. That session id is
    /// what the server now takes as a bearer token too, so it carries over and nobody signs in
    /// again; the card fills in the rest once the server has one.
    private func migrateLegacyIfNeeded() {
        guard networks.isEmpty else { return }
        let host = Self.legacyAddress.host() ?? ""
        let cookie = (HTTPCookieStorage.shared.cookies ?? []).first { $0.domain == host && $0.name == "__Host-tpcloud_session" }
        guard let cookie else { return }
        let id = "legacy:\(host)"
        networks = [SavedNetwork(id: id, name: "TPCloud", addresses: [.init(url: Self.legacyAddress, kind: "public")], anchor: nil, lastAddress: Self.legacyAddress)]
        currentID = id
        tokens.save(cookie.value, for: id)
        defaults.set(id, forKey: Self.currentKey)
        save()
    }

    private func save() {
        if let data = try? JSONEncoder().encode(networks) {
            defaults.set(data, forKey: Self.networksKey)
        }
    }
}

/// What belongs to one network on this device (its photo index, which photos have been backed up
/// to it, its Store favourites): kept in place for the network in use, put aside in
/// Application Support/Networks/<id>/ for the others.
enum NetworkData {
    private static let defaultsKeys = ["store.favorites", "PhotoBackgroundUploader.ownerEmail"]

    private static var support: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
    }

    private static func folder(_ id: String) -> URL {
        let safe = id.replacingOccurrences(of: "/", with: "_").replacingOccurrences(of: ":", with: "_")
        return support.appendingPathComponent("Networks/\(safe)", isDirectory: true)
    }

    private static func isNetworkFile(_ name: String) -> Bool {
        name == "PhotoUploadLedger.json" || (name.hasPrefix("PhotoIndex-") && name.hasSuffix(".json"))
    }

    static func stash(for id: String) {
        let fm = FileManager.default
        let dir = folder(id)
        try? fm.removeItem(at: dir)
        try? fm.createDirectory(at: dir, withIntermediateDirectories: true)
        for name in (try? fm.contentsOfDirectory(atPath: support.path)) ?? [] where isNetworkFile(name) {
            try? fm.moveItem(at: support.appendingPathComponent(name), to: dir.appendingPathComponent(name))
        }
        var values: [String: Any] = [:]
        for key in defaultsKeys {
            if let value = UserDefaults.standard.object(forKey: key) { values[key] = value }
            UserDefaults.standard.removeObject(forKey: key)
        }
        if let data = try? PropertyListSerialization.data(fromPropertyList: values, format: .binary, options: 0) {
            try? data.write(to: dir.appendingPathComponent("defaults.plist"))
        }
    }

    static func restore(for id: String) {
        let fm = FileManager.default
        let dir = folder(id)
        for name in (try? fm.contentsOfDirectory(atPath: support.path)) ?? [] where isNetworkFile(name) {
            try? fm.removeItem(at: support.appendingPathComponent(name))
        }
        for key in defaultsKeys { UserDefaults.standard.removeObject(forKey: key) }
        guard fm.fileExists(atPath: dir.path) else { return }
        for name in (try? fm.contentsOfDirectory(atPath: dir.path)) ?? [] where isNetworkFile(name) {
            try? fm.moveItem(at: dir.appendingPathComponent(name), to: support.appendingPathComponent(name))
        }
        if let data = try? Data(contentsOf: dir.appendingPathComponent("defaults.plist")),
           let values = try? PropertyListSerialization.propertyList(from: data, format: nil) as? [String: Any] {
            for (key, value) in values { UserDefaults.standard.set(value, forKey: key) }
        }
        try? fm.removeItem(at: dir)
    }

    static func forget(_ id: String) {
        try? FileManager.default.removeItem(at: folder(id))
    }
}
