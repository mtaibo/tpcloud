public struct CloudAccountClient: Sendable {
    public struct TOTPSetup: Decodable, Sendable {
        public let secret: String
        public let otpauthUri: String
    }

    private struct DisplayNameBody: Codable { let displayName: String }
    private struct PasswordChangeBody: Encodable { let currentPassword: String?; let newPassword: String }
    private struct TOTPCodeBody: Encodable { let code: String }
    private struct StatusResponse: Decodable { let status: String }

    private let client: HTTPClient

    public init(client: HTTPClient) {
        self.client = client
    }

    public func profile() async throws -> AccountProfile {
        try await client.get("/auth/account/profile")
    }

    public func updateDisplayName(_ displayName: String) async throws {
        let _: DisplayNameBody = try await client.patch("/auth/account/profile", body: DisplayNameBody(displayName: displayName))
    }

    public func changePassword(currentPassword: String?, newPassword: String) async throws {
        let _: StatusResponse = try await client.patch(
            "/auth/account/password",
            body: PasswordChangeBody(currentPassword: currentPassword, newPassword: newPassword)
        )
    }

    public func beginTOTPSetup() async throws -> TOTPSetup {
        try await client.post("/auth/account/totp/setup/begin")
    }

    public func completeTOTPSetup(code: String) async throws {
        let _: StatusResponse = try await client.post("/auth/account/totp/setup/complete", body: TOTPCodeBody(code: code))
    }

    public func disableTOTP(code: String) async throws {
        let _: StatusResponse = try await client.delete("/auth/account/totp", body: TOTPCodeBody(code: code))
    }

    // MARK: The host (admins): its version, a newer one, updating it

    public func host() async throws -> HostStatus {
        try await client.get("/auth/admin/host")
    }

    public func checkHost() async throws {
        let _: [String: String] = try await client.post("/auth/admin/host/check")
    }

    public func updateHost() async throws {
        let _: [String: String] = try await client.post("/auth/admin/host/update")
    }

    public func revokeSession(_ id: Int) async throws {
        let _: StatusResponse = try await client.delete("/auth/account/session/\(id)")
    }
}

/// The server's version and how an update is going (`GET /auth/admin/host`).
public struct HostStatus: Decodable, Sendable, Equatable {
    public struct Update: Decodable, Sendable, Equatable {
        /// checking, downloading, installing, done, latest or failed.
        public let state: String
        public let detail: String?
    }

    public let version: String
    public let latest: String?
    public let available: Bool
    public let update: Update?
    public let canUpdate: Bool

    public var isUpdating: Bool { ["checking", "downloading", "installing"].contains(update?.state ?? "") }
}
