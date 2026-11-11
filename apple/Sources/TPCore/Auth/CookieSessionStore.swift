import Foundation

public protocol SessionCookieStore: Sendable {
    var hasSessionCookie: Bool { get }
    func clear()
}

public struct HTTPCookieSessionStore: SessionCookieStore {
    /// Host-only and Secure (the `__Host-` prefix makes the browser enforce both), set by auth.
    public static let cookieName = "__Host-tpcloud_session"

    private let storage: HTTPCookieStorage
    private let host: String

    public init(storage: HTTPCookieStorage = .shared, host: String = TPServer.current.host) {
        self.storage = storage
        self.host = host
    }

    public var hasSessionCookie: Bool {
        sessionCookies.contains { $0.name == Self.cookieName }
    }

    public func clear() {
        for cookie in sessionCookies {
            storage.deleteCookie(cookie)
        }
    }

    private var sessionCookies: [HTTPCookie] {
        (storage.cookies ?? []).filter { $0.domain == host }
    }
}
