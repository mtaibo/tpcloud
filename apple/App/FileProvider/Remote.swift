import FileProvider
import Foundation
import TPCore

/// One network as the Finder sees it: a connection of its own (address, pinned authority, the
/// app's session from the shared Keychain), the account's folder on the host, and the calls the
/// extension makes.
actor Remote {
    let networkID: String
    private var connection: TPConnection?
    private var email: String?
    private var lastProbe = Date.distantPast

    init(networkID: String) {
        self.networkID = networkID
        email = TPShared.defaults.string(forKey: Self.emailKey(networkID))
    }

    static func emailKey(_ id: String) -> String { "tpcloud.fileprovider.email.\(id)" }

    /// Where the account's files are on the host: `users/<email>`.
    func rootPath() async throws -> String {
        if let email { return "users/\(email)" }
        let profile: UserProfile = try await call { try await HTTPClient(.auth, connection: $0).get("/auth/passkey/me") }
        email = profile.email
        TPShared.defaults.set(profile.email, forKey: Self.emailKey(networkID))
        return "users/\(profile.email)"
    }

    /// The Store's calls, through this network's connection, with errors the system understands.
    func store<T: Sendable>(_ body: @Sendable (StoreClient) async throws -> T) async throws -> T {
        try await call { try await body(StoreClient(client: HTTPClient(.store, connection: $0))) }
    }

    private func call<T: Sendable>(_ body: @Sendable (TPConnection) async throws -> T) async throws -> T {
        let connection = try await connect()
        do {
            return try await body(connection)
        } catch let error as APIError {
            if case .transport = error { lastProbe = .distantPast }
            throw Self.fileProviderError(error)
        }
    }

    /// The app's session for this network (read again each time: signing in again in the app
    /// changes it), at the best address that answered lately.
    private func connect() async throws -> TPConnection {
        guard let network = SavedNetwork.saved().first(where: { $0.id == networkID }) else {
            throw NSFileProviderError(.notAuthenticated)
        }
        guard let token = SessionTokenStore().load(for: networkID) else {
            throw NSFileProviderError(.notAuthenticated)
        }
        if let connection, connection.token == token, Date().timeIntervalSince(lastProbe) < 300 {
            return connection
        }
        let fallback = connection?.server.url ?? network.lastAddress ?? network.addresses.first?.url
        guard let fallback else { throw NSFileProviderError(.serverUnreachable) }
        let probe = TPConnection(network: network, address: fallback, token: token)
        let address = await NetworkStore.fastest(network, session: probe.session) ?? fallback
        let fresh = address == fallback ? probe : TPConnection(network: network, address: address, token: token)
        connection = fresh
        lastProbe = Date()
        return fresh
    }

    static func fileProviderError(_ error: APIError) -> Error {
        switch error {
        case .sessionExpired: NSFileProviderError(.notAuthenticated)
        case .server(401, _), .server(403, _): NSFileProviderError(.notAuthenticated)
        case .server(404, _): NSFileProviderError(.noSuchItem)
        case .server(409, _): NSFileProviderError(.filenameCollision)
        case .transport: NSFileProviderError(.serverUnreachable)
        case .server(_, let message):
            NSError(domain: NSCocoaErrorDomain, code: NSFileWriteUnknownError, userInfo: [NSLocalizedDescriptionKey: message ?? "The host turned it down"])
        case .decoding, .invalidResponse:
            NSError(domain: NSCocoaErrorDomain, code: NSFileReadUnknownError, userInfo: [NSLocalizedDescriptionKey: "Unexpected answer from the host"])
        }
    }
}
