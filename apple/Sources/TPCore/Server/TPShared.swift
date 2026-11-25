import Foundation
import Security

/// What the app shares with its extensions (the Finder's file provider) on this device: the
/// networks list, through the App Group's defaults, and each network's session, through a
/// keychain access group. On the iPhone everything stays the app's own until it has an extension.
public enum TPShared {
    /// Team-prefixed, the Mac's own kind of group: it needs nothing in the provisioning profile.
    /// A `group.` one has to be registered on the App ID too, and without it macOS refused the
    /// container (EPERM) and nothing was shared. The iPhone, when it shares, needs a `group.` one.
    public static let appGroup = "G3YVULBVD2.com.migueltaibo.tpcloud"
    /// The team's prefix and the group named in the entitlements' `keychain-access-groups`.
    static let keychainGroup = "G3YVULBVD2.com.migueltaibo.tpcloud.shared"

    #if os(macOS)
    public static let isShared = true
    #else
    public static let isShared = false
    #endif

    /// The App Group's defaults on the Mac, the app's own elsewhere.
    // UserDefaults is thread-safe; it just isn't marked Sendable.
    nonisolated(unsafe) public static let defaults: UserDefaults = {
        guard isShared, let group = UserDefaults(suiteName: appGroup) else { return .standard }
        return group
    }()

    /// The App Group's folder (the extension's snapshots and identifiers live in it).
    public static var container: URL {
        if isShared, let url = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroup) {
            return url
        }
        return FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
    }

    /// Moves what the app kept in its own defaults before it shared them. The old copies stay
    /// where they were, so going back to an older build loses nothing.
    static func migrateDefaults(_ keys: [String]) {
        guard isShared, defaults !== UserDefaults.standard, !defaults.bool(forKey: "tpcloud.shared.migrated") else { return }
        for key in keys where defaults.object(forKey: key) == nil {
            if let value = UserDefaults.standard.object(forKey: key) { defaults.set(value, forKey: key) }
        }
        defaults.set(true, forKey: "tpcloud.shared.migrated")
    }
}

/// Session tokens, one per network, in the Keychain (the shared group on the Mac, so the Finder's
/// file provider signs in with the app's session).
public struct SessionTokenStore: Sendable {
    private let service = "com.migueltaibo.tpcloud.session"

    public init() {}

    public func load(for network: String) -> String? {
        if let token = read(shared(network)) { return token }
        // Saved before the group existed: carried over the first time it's read.
        guard TPShared.isShared, let old = read(base(network)) else { return nil }
        write(old, shared(network))
        return old
    }

    public func save(_ token: String?, for network: String) {
        SecItemDelete(shared(network) as CFDictionary)
        if TPShared.isShared { SecItemDelete(base(network) as CFDictionary) }
        guard let token else { return }
        write(token, shared(network))
    }

    private func read(_ query: [String: Any]) -> String? {
        var query = query
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess, let data = result as? Data else { return nil }
        return String(data: data, encoding: .utf8)
    }

    private func write(_ token: String, _ query: [String: Any]) {
        var item = query
        item[kSecValueData as String] = Data(token.utf8)
        item[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        SecItemAdd(item as CFDictionary, nil)
    }

    private func base(_ network: String) -> [String: Any] {
        [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service, kSecAttrAccount as String: network]
    }

    private func shared(_ network: String) -> [String: Any] {
        var query = base(network)
        guard TPShared.isShared else { return query }
        query[kSecUseDataProtectionKeychain as String] = true
        query[kSecAttrAccessGroup as String] = TPShared.keychainGroup
        return query
    }
}
