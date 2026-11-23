import Foundation
import Security
import Testing
@testable import TPCore

struct NetworkCardTests {
    @Test func addressesAreNormalized() {
        #expect(NetworkCardLoader.normalized("tphome.local")?.absoluteString == "https://tphome.local")
        #expect(NetworkCardLoader.normalized(" https://cloud.example.com/login?x=1 ")?.absoluteString == "https://cloud.example.com")
        #expect(NetworkCardLoader.normalized("http://box.local:8090")?.absoluteString == "http://box.local:8090")
        #expect(NetworkCardLoader.normalized("") == nil)
    }

    @Test func cardDecodes() throws {
        let json = #"{"tpcloud":1,"id":"abc","name":"Casa","version":"0.4.0","addresses":[{"url":"https://box.local","kind":"local"}],"modules":["store"],"ca_sha256":null}"#
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        let card = try decoder.decode(NetworkCard.self, from: Data(json.utf8))
        #expect(card.addresses.first?.kind == "local")
        #expect(card.caSha256 == nil)
    }

    @Test func pemBecomesDER() {
        let pem = "-----BEGIN CERTIFICATE-----\nMAMCAQE=\n-----END CERTIFICATE-----\n"
        #expect(NetworkCardLoader.derFromPEM(Data(pem.utf8)) == Data([0x30, 0x03, 0x02, 0x01, 0x01]))
    }

    /// Against a real host with its own certificate authority: TPCLOUD_TEST_URL=https://localhost
    /// (a VM's local site with `localhost` among its names). Skipped otherwise.
    @Test(.enabled(if: ProcessInfo.processInfo.environment["TPCLOUD_TEST_URL"] != nil))
    func pinsTheHostsOwnAuthority() async throws {
        let address = ProcessInfo.processInfo.environment["TPCLOUD_TEST_URL"]!
        let loaded = try await NetworkCardLoader.load(address)
        let der = try #require(loaded.anchor, "a local address should come with its authority")
        let anchor = try #require(SecCertificateCreateWithData(nil, der as CFData))
        let url = try #require(NetworkCardLoader.normalized(address))

        TPConnection.shared.use(TPServer(url: url), token: nil, anchors: [url.host()!: anchor])
        let (_, ok) = try await TPConnection.shared.session.data(for: TPConnection.shared.request(url.appending(path: ".well-known/tpcloud")))
        #expect((ok as? HTTPURLResponse)?.statusCode == 200)

        // Signing in through the app's own client leaves a bearer token that works on its own.
        if let password = ProcessInfo.processInfo.environment["TPCLOUD_TEST_PASSWORD"] {
            let auth = PasswordTOTPAuthenticator(client: HTTPClient(.auth))
            TPConnection.shared.adopt(token: nil)
            _ = try await auth.login(email: ProcessInfo.processInfo.environment["TPCLOUD_TEST_EMAIL"] ?? "owner@example.com", password: password)
            #expect(TPConnection.shared.token != nil)
            for cookie in HTTPCookieStorage.shared.cookies ?? [] { HTTPCookieStorage.shared.deleteCookie(cookie) }
            let me = try await auth.currentProfile()
            #expect(!me.email.isEmpty)
            try await auth.logout()
        }

        // Any other authority: refused (a fresh session, so the TLS check isn't skipped by reusing
        // the connection the right anchor opened).
        let other = try #require(SecCertificateCreateWithData(nil, Data(base64Encoded: Self.unrelated)! as CFData))
        TPConnection.shared.use(TPServer(url: url), token: nil, anchors: [url.host()!: other])
        let fresh = URLSession(configuration: .ephemeral, delegate: TPTrustDelegate.shared, delegateQueue: nil)
        await #expect(throws: (any Error).self) {
            _ = try await fresh.data(for: URLRequest(url: url.appending(path: ".well-known/tpcloud")))
        }
        // And with nothing pinned, the system alone doesn't trust it either.
        TPConnection.shared.use(TPServer(url: url), token: nil, anchors: [:])
        let bare = URLSession(configuration: .ephemeral, delegate: TPTrustDelegate.shared, delegateQueue: nil)
        await #expect(throws: (any Error).self) {
            _ = try await bare.data(for: URLRequest(url: url.appending(path: ".well-known/tpcloud")))
        }
    }

    /// An unrelated self-signed certificate (CN=not-this).
    static let unrelated = "MIICCjCCAbACCQDVd4yLNJ7RXjAKBggqhkjOPQQDAjATMREwDwYDVQQDDAhub3QtdGhpczAeFw0yNjExMjMxOTE5NThaFw0zNjExMjAxOTE5NThaMBMxETAPBgNVBAMMCG5vdC10aGlzMIIBSzCCAQMGByqGSM49AgEwgfcCAQEwLAYHKoZIzj0BAQIhAP////8AAAABAAAAAAAAAAAAAAAA////////////////MFsEIP////8AAAABAAAAAAAAAAAAAAAA///////////////8BCBaxjXYqjqT57PrvVV2mIa8ZR0GsMxTsPY7zjw+J9JgSwMVAMSdNgiG5wSTamZ44ROdJreBn36QBEEEaxfR8uEsQkf4vOblY6RA8ncDfYEt6zOg9KE5RdiYwpZP40Li/hp/m47n60p8D54WK84zV2sxXs7LtkBoN79R9QIhAP////8AAAAA//////////+85vqtpxeehPO5ysL8YyVRAgEBA0IABA1nyzWsuJqGrmVaZJPSvpmjSbWxtYq/gssWjZ0ioNWUFKiscL0gucsiXwhVf9LYYxsbckU6d2dsB0gRfOS94QgwCgYIKoZIzj0EAwIDSAAwRQIgWe0COQDUHjZN+WxBtHYox+2HS5iKXODDsy+pK59DNToCIQCYMMHiJHdbz5MGldUwnUYb8kqv385ZduKcN5plczVC3A=="
}
