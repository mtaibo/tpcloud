import Foundation

/// A TPCloud server's address. Every module lives under it, each on its own path (`/store`,
/// `/home`; auth at the root), so one URL is all a server needs.
public struct TPServer: Sendable, Equatable {
    public let url: URL

    public init(url: URL) {
        self.url = url
    }

    public enum Module: Sendable {
        case auth, store, home
    }

    public func url(for module: Module) -> URL {
        switch module {
        case .auth: url
        case .store: url.appending(path: "store")
        case .home: url.appending(path: "home")
        }
    }

    public var authURL: URL { url(for: .auth) }
    public var storeURL: URL { url(for: .store) }
    public var homeURL: URL { url(for: .home) }

    public var homeWebSocketURL: URL {
        var components = URLComponents(url: url.appending(path: "home/api/ws"), resolvingAgainstBaseURL: false)!
        components.scheme = url.scheme == "http" ? "ws" : "wss"
        return components.url!
    }

    public var host: String { url.host() ?? "" }

    /// The address of the network in use now (it changes when the app moves to another network
    /// or another address of the same one).
    public static var current: TPServer { TPConnection.shared.server }
}
