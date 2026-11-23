import Foundation

@MainActor
public final class AuthSession: ObservableObject {
    public enum State: Equatable, Sendable {
        case checkingSession
        case signedOut
        case totpRequired(email: String)
        case signedIn(UserProfile)
    }

    @Published public private(set) var state: State = .checkingSession

    private let authenticator: PasswordTOTPAuthenticator
    private let cookieStore: SessionCookieStore

    public init(authenticator: PasswordTOTPAuthenticator, cookieStore: SessionCookieStore = HTTPCookieSessionStore()) {
        self.authenticator = authenticator
        self.cookieStore = cookieStore
        NotificationCenter.default.addObserver(
            forName: .tpSessionExpired,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor in
                self?.state = .signedOut
            }
        }
    }

    public convenience init(baseURL: URL, cookieStore: SessionCookieStore = HTTPCookieSessionStore()) {
        self.init(authenticator: PasswordTOTPAuthenticator(client: HTTPClient(baseURL: baseURL)), cookieStore: cookieStore)
    }

    /// Signs in against whichever network is in use.
    public convenience init(cookieStore: SessionCookieStore = HTTPCookieSessionStore()) {
        self.init(authenticator: PasswordTOTPAuthenticator(client: HTTPClient(.auth)), cookieStore: cookieStore)
    }

    /// The app moved to another network: its own session, if it has one.
    public func networkChanged() async {
        state = .checkingSession
        await restoreIfPossible()
    }

    /// Call on app launch: restores a signed-in state from the persisted session cookie, if any, without prompting for credentials.
    public func restoreIfPossible() async {
        guard cookieStore.hasSessionCookie else {
            state = .signedOut
            return
        }
        do {
            state = .signedIn(try await authenticator.currentProfile())
        } catch {
            state = .signedOut
        }
    }

    public func login(email: String, password: String) async throws {
        let outcome = try await authenticator.login(email: email, password: password)
        if outcome.totpRequired {
            state = .totpRequired(email: email)
        } else {
            state = .signedIn(try await authenticator.currentProfile())
        }
    }

    /// Returns from the TOTP step to the password step without contacting the server, mirroring the web login's "Back" button.
    public func cancelTOTP() {
        state = .signedOut
    }

    public func verifyTOTP(_ code: String) async throws {
        try await authenticator.verifyTOTP(code: code)
        state = .signedIn(try await authenticator.currentProfile())
    }

    public func signOut() async {
        try? await authenticator.logout()
        cookieStore.clear()
        state = .signedOut
    }
}
