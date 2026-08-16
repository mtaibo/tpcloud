import Foundation

private let base = "https://cloud.migueltaibo.com"
private let client = APIClient.shared

enum CloudAPI {

    // MARK: - Auth

    static func me() async throws -> MeResponse {
        try await client.get(URL(string: "\(base)/auth/passkey/me")!)
    }

    /// Returns the raw challenge JSON data from the server.
    static func loginBegin(email: String) async throws -> Data {
        var comps = URLComponents(string: "\(base)/auth/passkey/login/begin")!
        comps.queryItems = [URLQueryItem(name: "email", value: email)]
        var req = URLRequest(url: comps.url!)
        req.httpMethod = "POST"
        let (data, response) = try await client.session.data(for: req)
        guard let http = response as? HTTPURLResponse else {
            throw APIError.serverError(0)
        }
        if http.statusCode >= 400 {
            throw APIError.serverError(http.statusCode)
        }
        return data
    }

    /// Posts the assembled assertion object (as JSON Data) to complete login.
    static func loginComplete(email: String, body: Data) async throws {
        var comps = URLComponents(string: "\(base)/auth/passkey/login/complete")!
        comps.queryItems = [URLQueryItem(name: "email", value: email)]
        var req = URLRequest(url: comps.url!)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = body
        let (_, response) = try await client.session.data(for: req)
        guard let http = response as? HTTPURLResponse else { return }
        if http.statusCode == 401 { throw APIError.unauthorized }
        if http.statusCode >= 400 { throw APIError.serverError(http.statusCode) }
    }

    static func logout() async throws {
        var req = URLRequest(url: URL(string: "\(base)/auth/passkey/logout")!)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = "{}".data(using: .utf8)
        _ = try await client.session.data(for: req)
    }

    // MARK: - Account

    static func profile() async throws -> UserProfile {
        try await client.get(URL(string: "\(base)/auth/account/profile")!)
    }

    static func updateDisplayName(_ name: String) async throws {
        try await client.patchJSON(
            URL(string: "\(base)/auth/account/profile")!,
            json: ["display_name": name]
        )
    }

    static func deletePasskey(_ credentialId: String) async throws {
        try await client.delete(URL(string: "\(base)/auth/account/passkey/\(credentialId)")!)
    }

    static func revokeMySession(_ sessionId: String) async throws {
        try await client.delete(URL(string: "\(base)/auth/account/session/\(sessionId)")!)
    }

    // MARK: - Admin: Users

    static func adminUsers() async throws -> [AdminUser] {
        try await client.get(URL(string: "\(base)/auth/admin/users")!)
    }

    static func adminToggleAdmin(_ userId: Int) async throws {
        var req = URLRequest(url: URL(string: "\(base)/auth/admin/users/\(userId)")!)
        req.httpMethod = "PATCH"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = "{}".data(using: .utf8)
        _ = try await client.session.data(for: req)
    }

    static func adminDeleteUser(_ userId: Int) async throws {
        try await client.delete(URL(string: "\(base)/auth/admin/users/\(userId)")!)
    }

    // MARK: - Admin: Sessions

    static func adminSessions() async throws -> [AdminSession] {
        try await client.get(URL(string: "\(base)/auth/admin/sessions")!)
    }

    static func adminRevokeSession(_ sessionId: String) async throws {
        try await client.delete(URL(string: "\(base)/auth/admin/sessions/\(sessionId)")!)
    }

    // MARK: - Admin: Invites

    static func adminInvites() async throws -> [Invite] {
        try await client.get(URL(string: "\(base)/auth/admin/invites")!)
    }

    static func adminCreateInvite(email: String) async throws {
        var req = URLRequest(url: URL(string: "\(base)/auth/admin/invites")!)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = try JSONSerialization.data(withJSONObject: ["email": email])
        _ = try await client.session.data(for: req)
    }

    static func adminRevokeInvite(_ email: String) async throws {
        let encoded = email.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? email
        try await client.delete(URL(string: "\(base)/auth/admin/invites/\(encoded)")!)
    }
}
