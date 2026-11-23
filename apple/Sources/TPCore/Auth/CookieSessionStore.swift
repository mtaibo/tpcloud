import Foundation

public protocol SessionCookieStore: Sendable {
    var hasSessionCookie: Bool { get }
    func clear()
}

public struct HTTPCookieSessionStore: SessionCookieStore {
    /// Host-only and Secure (the `__Host-` prefix makes the browser enforce both), set by auth.
    public static let cookieName = "__Host-tpcloud_session"

    private let storage: HTTPCookieStorage
    private let fixedHost: String?

    /// `host` nil: the network in use, whichever it is when asked.
    public init(storage: HTTPCookieStorage = .shared, host: String? = nil) {
        self.storage = storage
        self.fixedHost = host
    }

    /// A session to restore: the app's bearer token, or (from before tokens) the cookie.
    public var hasSessionCookie: Bool {
        TPConnection.shared.token != nil || sessionCookies.contains { $0.name == Self.cookieName }
    }

    public func clear() {
        TPConnection.shared.adopt(token: nil)
        for cookie in sessionCookies {
            storage.deleteCookie(cookie)
        }
    }

    private var sessionCookies: [HTTPCookie] {
        let host = fixedHost ?? TPServer.current.host
        return (storage.cookies ?? []).filter { $0.domain == host }
    }
}
