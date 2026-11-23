import Foundation
import Security

/// What a TPCloud network says about itself at `/.well-known/tpcloud`, before anyone signs in.
public struct NetworkCard: Codable, Sendable, Equatable {
    public struct Address: Codable, Sendable, Equatable, Hashable {
        public let url: URL
        /// `local` (only at home), `tailscale` or `public`.
        public let kind: String
    }

    public let tpcloud: Int
    public let id: String
    public let name: String
    public let version: String
    public let addresses: [Address]
    public let modules: [String]
    /// SHA-256 of the host's own certificate authority (for local addresses), if it has one.
    public let caSha256: String?
}

public enum NetworkCardError: LocalizedError {
    case notTPCloud
    case unreachable(String)
    case certificateMismatch

    public var errorDescription: String? {
        switch self {
        case .notTPCloud: "There's no TPCloud at that address."
        case .unreachable(let why): "Couldn't reach it: \(why)"
        case .certificateMismatch: "Its certificate isn't the one it says it uses. Don't add it if you're not sure where you are."
        }
    }
}

/// Reads a network's card from an address the app may not trust yet. The first time, a local
/// address's certificate is unknown: the card names its authority's fingerprint, the authority
/// itself comes from `/auth/network/ca.crt`, both have to match, and the connection has to check
/// out against it before it's kept (trust on first use, like SSH; a QR from a signed-in device
/// would carry the fingerprint instead).
public enum NetworkCardLoader {
    public struct Loaded: Sendable {
        public let card: NetworkCard
        /// The certificate authority to pin, when the address isn't trusted by the system.
        public let anchor: Data?
    }

    /// `address` is a URL or a bare host (`tphome.local`, `cloud.example.com`).
    public static func load(_ address: String) async throws -> Loaded {
        guard let base = normalized(address) else { throw NetworkCardError.notTPCloud }
        let probe = Probe()
        let session = URLSession(configuration: .ephemeral, delegate: probe, delegateQueue: nil)
        defer { session.finishTasksAndInvalidate() }
        let card: NetworkCard
        do {
            var request = URLRequest(url: base.appending(path: ".well-known/tpcloud"))
            request.timeoutInterval = 8
            let (data, response) = try await session.data(for: request)
            guard (response as? HTTPURLResponse)?.statusCode == 200 else { throw NetworkCardError.notTPCloud }
            let decoder = JSONDecoder()
            decoder.keyDecodingStrategy = .convertFromSnakeCase
            guard let decoded = try? decoder.decode(NetworkCard.self, from: data), decoded.tpcloud >= 1 else {
                throw NetworkCardError.notTPCloud
            }
            card = decoded
        } catch let error as NetworkCardError {
            throw error
        } catch {
            throw NetworkCardError.unreachable(error.localizedDescription)
        }
        guard let trust = probe.trust, !probe.systemTrusted else {
            // A public address with a real certificate (or plain HTTP): nothing to pin.
            return Loaded(card: card, anchor: nil)
        }
        guard let expected = card.caSha256 else { throw NetworkCardError.certificateMismatch }
        var request = URLRequest(url: base.appending(path: "auth/network/ca.crt"))
        request.timeoutInterval = 8
        guard let (pem, _) = try? await session.data(for: request),
              let der = derFromPEM(pem),
              TPTrustDelegate.fingerprint(der) == expected.lowercased(),
              let anchor = SecCertificateCreateWithData(nil, der as CFData),
              TPTrustDelegate.trusts(trust, anchor: anchor) else {
            throw NetworkCardError.certificateMismatch
        }
        return Loaded(card: card, anchor: der)
    }

    public static func normalized(_ address: String) -> URL? {
        var text = address.trimmingCharacters(in: .whitespacesAndNewlines)
        if text.isEmpty { return nil }
        if !text.contains("://") { text = "https://" + text }
        guard var components = URLComponents(string: text), components.host?.isEmpty == false else { return nil }
        components.path = ""
        components.query = nil
        return components.url
    }

    static func derFromPEM(_ data: Data) -> Data? {
        guard let text = String(data: data, encoding: .utf8),
              let start = text.range(of: "-----BEGIN CERTIFICATE-----"),
              let end = text.range(of: "-----END CERTIFICATE-----") else { return nil }
        let body = text[start.upperBound..<end.lowerBound].filter { !$0.isWhitespace }
        return Data(base64Encoded: String(body))
    }

    /// Lets the first request through whatever its certificate, remembering it to be checked
    /// against the card afterwards. Used only to read the card and the authority.
    private final class Probe: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
        var trust: SecTrust?
        var systemTrusted = false

        func urlSession(
            _ session: URLSession,
            task: URLSessionTask,
            didReceive challenge: URLAuthenticationChallenge,
            completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
        ) {
            guard challenge.protectionSpace.authenticationMethod == NSURLAuthenticationMethodServerTrust,
                  let trust = challenge.protectionSpace.serverTrust else {
                completionHandler(.performDefaultHandling, nil)
                return
            }
            if self.trust == nil {
                self.trust = trust
                systemTrusted = SecTrustEvaluateWithError(trust, nil)
            }
            completionHandler(.useCredential, URLCredential(trust: trust))
        }
    }
}
