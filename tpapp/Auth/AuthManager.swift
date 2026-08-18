import Foundation
import AuthenticationServices
import Observation

enum AuthState {
    case loading
    case unauthenticated
    case authenticated(MeResponse)
}

@Observable
class AuthManager: NSObject {
    var state: AuthState = .loading
    private var pendingContinuation: CheckedContinuation<[String: Any], Error>?

    func checkSession() async {
        do {
            let me = try await CloudAPI.me()
            state = .authenticated(me)
        } catch {
            state = .unauthenticated
        }
    }

    func login(email: String) async throws {
        // 1. Fetch challenge from server
        let challengeData = try await CloudAPI.loginBegin(email: email)

        // 2. Parse challenge options
        guard let options = try? JSONSerialization.jsonObject(with: challengeData) as? [String: Any],
              let challengeStr = options["challenge"] as? String else {
            throw APIError.serverError(0)
        }

        let challengeBytes = base64URLDecode(challengeStr)
        let rpId = (options["rpId"] as? String) ?? "migueltaibo.com"

        // 3. Build the assertion request
        let provider = ASAuthorizationPlatformPublicKeyCredentialProvider(relyingPartyIdentifier: rpId)
        let assertionReq = provider.createCredentialAssertionRequest(challenge: Data(challengeBytes))

        if let allowCreds = options["allowCredentials"] as? [[String: Any]] {
            assertionReq.allowedCredentials = allowCreds.compactMap { cred -> ASAuthorizationPlatformPublicKeyCredentialDescriptor? in
                guard let idStr = cred["id"] as? String else { return nil }
                return ASAuthorizationPlatformPublicKeyCredentialDescriptor(
                    credentialID: Data(base64URLDecode(idStr))
                )
            }
        }

        // 4. Present the passkey sheet and await the result
        let assertion = try await withCheckedThrowingContinuation { (cont: CheckedContinuation<[String: Any], Error>) in
            self.pendingContinuation = cont
            let controller = ASAuthorizationController(authorizationRequests: [assertionReq])
            controller.delegate = self
            controller.presentationContextProvider = self
            controller.performRequests()
        }

        // 5. Send assertion to server
        let bodyData = try JSONSerialization.data(withJSONObject: assertion)
        try await CloudAPI.loginComplete(email: email, body: bodyData)

        // 6. Fetch me to confirm session and update state
        let me = try await CloudAPI.me()
        state = .authenticated(me)
    }

    func logout() async {
        try? await CloudAPI.logout()
        state = .unauthenticated
    }

    // MARK: - Base64URL helpers

    private func base64URLDecode(_ str: String) -> [UInt8] {
        var s = str
            .replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/")
        let pad = (4 - s.count % 4) % 4
        s += String(repeating: "=", count: pad)
        return Array(Data(base64Encoded: s) ?? Data())
    }

    private func base64URLEncode(_ data: Data) -> String {
        data.base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }
}

// MARK: - ASAuthorizationControllerDelegate

extension AuthManager: ASAuthorizationControllerDelegate {
    func authorizationController(
        controller: ASAuthorizationController,
        didCompleteWithAuthorization authorization: ASAuthorization
    ) {
        guard let cred = authorization.credential as? ASAuthorizationPlatformPublicKeyCredentialAssertion else {
            pendingContinuation?.resume(throwing: APIError.serverError(0))
            pendingContinuation = nil
            return
        }

        let userHandleValue: Any
        if let userID = cred.userID {
            userHandleValue = base64URLEncode(userID)
        } else {
            userHandleValue = NSNull()
        }

        let assertion: [String: Any] = [
            "id": base64URLEncode(cred.credentialID),
            "rawId": base64URLEncode(cred.credentialID),
            "type": "public-key",
            "response": [
                "clientDataJSON": base64URLEncode(cred.rawClientDataJSON),
                "authenticatorData": base64URLEncode(cred.rawAuthenticatorData),
                "signature": base64URLEncode(cred.signature),
                "userHandle": userHandleValue,
            ],
            "clientExtensionResults": [String: Any]()
        ]

        pendingContinuation?.resume(returning: assertion)
        pendingContinuation = nil
    }

    func authorizationController(
        controller: ASAuthorizationController,
        didCompleteWithError error: Error
    ) {
        pendingContinuation?.resume(throwing: error)
        pendingContinuation = nil
    }
}

// MARK: - ASAuthorizationControllerPresentationContextProviding

extension AuthManager: ASAuthorizationControllerPresentationContextProviding {
    func presentationAnchor(for controller: ASAuthorizationController) -> ASPresentationAnchor {
        UIApplication.shared.connectedScenes
            .compactMap { $0 as? UIWindowScene }
            .flatMap { $0.windows }
            .first { $0.isKeyWindow } ?? UIWindow()
    }
}
