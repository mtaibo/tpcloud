import Foundation

struct MeResponse: Codable {
    let email: String
    let displayName: String
    enum CodingKeys: String, CodingKey {
        case email
        case displayName = "display_name"
    }
}

struct UserProfile: Codable {
    let id: Int
    let email: String
    let displayName: String
    let isAdmin: Bool
    let passkeys: [PasskeyItem]
    let sessions: [SessionItem]
    enum CodingKeys: String, CodingKey {
        case id, email, passkeys, sessions
        case displayName = "display_name"
        case isAdmin = "is_admin"
    }
}

struct PasskeyItem: Codable, Identifiable {
    var id: String { credentialId }
    let credentialId: String
    let deviceName: String
    let createdAt: String
    let lastUsedAt: String?
    enum CodingKeys: String, CodingKey {
        case credentialId = "credential_id"
        case deviceName = "device_name"
        case createdAt = "created_at"
        case lastUsedAt = "last_used_at"
    }
}

struct SessionItem: Codable, Identifiable {
    var id: String { sessionId }
    let sessionId: String
    let createdAt: String
    let expiresAt: String
    let ipAddress: String?
    let current: Bool
    enum CodingKeys: String, CodingKey {
        case sessionId = "session_id"
        case createdAt = "created_at"
        case expiresAt = "expires_at"
        case ipAddress = "ip_address"
        case current
    }
}

struct AdminUser: Codable, Identifiable {
    let id: Int
    let email: String
    let displayName: String
    let isAdmin: Bool
    let createdAt: String
    let passkeyCount: Int
    let activeSessionCount: Int
    enum CodingKeys: String, CodingKey {
        case id, email
        case displayName = "display_name"
        case isAdmin = "is_admin"
        case createdAt = "created_at"
        case passkeyCount = "passkey_count"
        case activeSessionCount = "active_session_count"
    }
}

struct AdminSession: Codable, Identifiable {
    var id: String { sessionId }
    let sessionId: String
    let userEmail: String
    let createdAt: String
    let expiresAt: String
    let ipAddress: String?
    enum CodingKeys: String, CodingKey {
        case sessionId = "session_id"
        case userEmail = "user_email"
        case createdAt = "created_at"
        case expiresAt = "expires_at"
        case ipAddress = "ip_address"
    }
}

struct Invite: Codable, Identifiable {
    var id: String { email }
    let email: String
    let used: Bool
    let invitedBy: Int?
    let createdAt: String
    enum CodingKeys: String, CodingKey {
        case email, used
        case invitedBy = "invited_by"
        case createdAt = "created_at"
    }
}
