import Foundation

/// The TPCloud server the app talks to. Every module lives under one address, each on its own
/// path (`/store`, `/home`, `/admin`; auth at the root), so one URL is all a server needs.
public struct TPServer: Sendable, Equatable {
    public let url: URL

    public init(url: URL) {
        self.url = url
    }

    public var authURL: URL { url }
    public var storeURL: URL { url.appending(path: "store") }
    public var homeURL: URL { url.appending(path: "home") }

    public var homeWebSocketURL: URL {
        var components = URLComponents(url: url.appending(path: "home/api/ws"), resolvingAgainstBaseURL: false)!
        components.scheme = url.scheme == "http" ? "ws" : "wss"
        return components.url!
    }

    public var host: String { url.host() ?? "" }

    /// Until the app lets you pick your own server, it's this one.
    public static let current = TPServer(url: URL(string: "https://cloud.migueltaibo.com")!)
}
