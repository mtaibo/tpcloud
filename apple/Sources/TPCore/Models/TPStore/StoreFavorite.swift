import Foundation

/// A folder pinned to Favorites in the file browser, stored per account by store-api
/// (`/api/favorites`) so the app and the web on every browser show the same list.
public struct StoreFavorite: Codable, Identifiable, Equatable, Sendable {
    /// "<location>:<path>", the same id the web uses.
    public let id: String
    public let location: StoreLocation
    public let path: String
    /// The folder's own name (the last part of `path`).
    public let label: String
    public let iconName: String?
    /// False once the folder was renamed, moved or deleted.
    public let exists: Bool

    public init(location: StoreLocation, path: String, label: String, iconName: String? = nil, exists: Bool = true) {
        self.id = Self.id(location: location, path: path)
        self.location = location
        self.path = path
        self.label = label
        self.iconName = iconName
        self.exists = exists
    }

    public static func id(location: StoreLocation, path: String) -> String {
        "\(location.rawValue):\(path)"
    }

    public init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        location = try c.decode(StoreLocation.self, forKey: .location)
        path = try c.decode(String.self, forKey: .path)
        id = try c.decodeIfPresent(String.self, forKey: .id) ?? Self.id(location: location, path: path)
        label = try c.decodeIfPresent(String.self, forKey: .label) ?? path.split(separator: "/").last.map(String.init) ?? path
        iconName = try c.decodeIfPresent(String.self, forKey: .iconName)
        exists = try c.decodeIfPresent(Bool.self, forKey: .exists) ?? true
    }
}
