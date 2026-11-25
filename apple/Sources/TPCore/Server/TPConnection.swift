import CryptoKit
import Foundation
import Security

extension Notification.Name {
    /// The app moved to another network (or another address of the same one).
    public static let tpConnectionChanged = Notification.Name("TPConnectionChanged")
}

/// Where requests go right now: the active network's address, its session token and, for an
/// address served with the host's own certificate authority, that authority's fingerprint.
/// Everything that talks to a TPCloud server goes through `session` and `authorize(_:)`.
public final class TPConnection: @unchecked Sendable {
    /// Starts where the app left off (the network in use, its address, certificate and session),
    /// even when iOS wakes the app in the background with no screen to set it up.
    public static let shared: TPConnection = {
        let connection = TPConnection()
        connection.restoreSaved()
        return connection
    }()

    static let addressKey = "tpcloud.connection.url"

    private let lock = NSLock()
    private var _server: TPServer
    private var _token: String?
    /// Host → the certificate authority it's pinned to (the host's own, for a local address).
    private var _anchors: [String: SecCertificate] = [:]

    /// Called with each new session token (or nil when it's gone), so it can be kept somewhere
    /// safer than memory (`NetworkStore` puts it in the Keychain).
    public var onTokenChange: (@Sendable (String?) -> Void)?

    /// Whether this is the app's own connection (the one the network in use goes through), whose
    /// address is remembered for the next launch.
    private let isShared: Bool

    init(server: TPServer = TPServer(url: URL(string: "https://cloud.migueltaibo.com")!)) {
        _server = server
        isShared = true
    }

    /// A connection of its own to one network, apart from the app's (the Finder's file provider
    /// has one per network it shows).
    public init(network: SavedNetwork, address: URL, token: String?) {
        _server = TPServer(url: address)
        _token = token
        _anchors = network.anchors
        isShared = false
    }

    private func restoreSaved() {
        TPShared.migrateDefaults([NetworkStore.networksKey, NetworkStore.currentKey, Self.addressKey])
        let defaults = TPShared.defaults
        guard let id = defaults.string(forKey: NetworkStore.currentKey),
              let network = SavedNetwork.saved().first(where: { $0.id == id }) else { return }
        let url = defaults.url(forKey: Self.addressKey) ?? network.lastAddress ?? network.addresses.first?.url
        guard let url else { return }
        _server = TPServer(url: url)
        _token = SessionTokenStore().load(for: id)
        _anchors = network.anchors
    }

    public var server: TPServer {
        lock.withLock { _server }
    }

    public var token: String? {
        lock.withLock { _token }
    }

    /// Points every request at another address (and its session).
    public func use(_ server: TPServer, token: String?, anchors: [String: SecCertificate]) {
        lock.withLock {
            _server = server
            _token = token
            _anchors = anchors
        }
        guard isShared else { return }
        TPShared.defaults.set(server.url, forKey: Self.addressKey)
        NotificationCenter.default.post(name: .tpConnectionChanged, object: nil)
    }

    /// A sign-in answered with a session token (`X-TP-Session`), or a sign-out dropped it.
    public func adopt(token: String?) {
        let changed = lock.withLock {
            defer { _token = token }
            return _token != token
        }
        if changed { onTokenChange?(token) }
    }

    public func anchor(_ certificate: SecCertificate, for host: String) {
        lock.withLock { _anchors[host.lowercased()] = certificate }
    }

    func anchor(for host: String) -> SecCertificate? {
        lock.withLock { _anchors[host.lowercased()] }
    }

    /// The session token and which app this is, for a request to the active network. Requests to
    /// anywhere else get neither.
    public func authorize(_ request: inout URLRequest) {
        let (server, token) = lock.withLock { (_server, _token) }
        guard request.url?.host()?.lowercased() == server.host.lowercased() else { return }
        if let token {
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        request.setValue(Self.clientPlatform, forHTTPHeaderField: "X-TP-Client")
    }

    /// A GET for one of the server's URLs, already authorized.
    public func request(_ url: URL) -> URLRequest {
        var request = URLRequest(url: url)
        authorize(&request)
        return request
    }

    /// The session every request to a TPCloud server uses: the shared caches and cookies, and
    /// a delegate that accepts the host's own certificate authority where it's pinned.
    public lazy var session: URLSession = URLSession(
        configuration: .default,
        delegate: isShared ? TPTrustDelegate.shared : TPTrustDelegate(connection: self),
        delegateQueue: nil
    )

    #if os(iOS)
    public static let clientPlatform = "ios"
    #elseif os(macOS)
    public static let clientPlatform = "macos"
    #else
    public static let clientPlatform = "native"
    #endif
}

/// Accepts a server whose certificate the system trusts, or one that checks out against the
/// certificate authority pinned for its host (a local address with the host's own authority):
/// the system still checks the name, the dates and the chain, only against that anchor.
public final class TPTrustDelegate: NSObject, URLSessionDelegate, URLSessionTaskDelegate, @unchecked Sendable {
    public static let shared = TPTrustDelegate(connection: nil)

    /// Whose pinned authorities count; nil is the app's own connection.
    private weak var connection: TPConnection?

    init(connection: TPConnection?) {
        self.connection = connection
    }

    public func urlSession(
        _ session: URLSession,
        didReceive challenge: URLAuthenticationChallenge,
        completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
    ) {
        let (disposition, credential) = Self.evaluate(challenge, connection: connection ?? .shared)
        completionHandler(disposition, credential)
    }

    public func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didReceive challenge: URLAuthenticationChallenge,
        completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
    ) {
        let (disposition, credential) = Self.evaluate(challenge, connection: connection ?? .shared)
        completionHandler(disposition, credential)
    }

    /// What every TPCloud session answers to a server-trust challenge (also the background
    /// upload session, which has a delegate of its own).
    public static func evaluate(_ challenge: URLAuthenticationChallenge, connection: TPConnection = .shared) -> (URLSession.AuthChallengeDisposition, URLCredential?) {
        guard challenge.protectionSpace.authenticationMethod == NSURLAuthenticationMethodServerTrust,
              let trust = challenge.protectionSpace.serverTrust else {
            return (.performDefaultHandling, nil)
        }
        if SecTrustEvaluateWithError(trust, nil) {
            return (.useCredential, URLCredential(trust: trust))
        }
        guard let anchor = connection.anchor(for: challenge.protectionSpace.host),
              Self.trusts(trust, anchor: anchor) else {
            return (.cancelAuthenticationChallenge, nil)
        }
        return (.useCredential, URLCredential(trust: trust))
    }

    /// The chain is good with `anchor` as its only root.
    public static func trusts(_ trust: SecTrust, anchor: SecCertificate) -> Bool {
        SecTrustSetAnchorCertificates(trust, [anchor] as CFArray)
        SecTrustSetAnchorCertificatesOnly(trust, true)
        return SecTrustEvaluateWithError(trust, nil)
    }

    /// SHA-256 of a certificate (DER), lowercase hex: what the network card calls `ca_sha256`.
    public static func fingerprint(_ der: Data) -> String {
        SHA256.hash(data: der).map { String(format: "%02x", $0) }.joined()
    }
}
