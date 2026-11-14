import Foundation

/// An album of backed-up photos: Favoritos (everyone has it) or one of the user's own. A hidden
/// album lives in the Hidden section instead of the albums list; one that hides from the library
/// keeps its photos out of the library, so they're only seen inside it.
public struct PhotoAlbum: Codable, Equatable, Hashable, Sendable, Identifiable {
    public struct Cover: Codable, Equatable, Hashable, Sendable {
        public let id: Int?
        public let path: String
        public let checksum: String
    }

    public let id: Int
    public let name: String
    public let kind: String
    public let count: Int
    public let hidden: Bool
    public let hidesFromLibrary: Bool
    public let cover: Cover?

    public var isFavorites: Bool { kind == "favorites" }

    /// "Favoritos" is how the server stores it; the app's chrome is in English.
    public var displayName: String { isFavorites ? "Favorites" : name }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int.self, forKey: .id)
        name = try c.decode(String.self, forKey: .name)
        kind = try c.decode(String.self, forKey: .kind)
        count = try c.decode(Int.self, forKey: .count)
        // Older servers don't send these.
        hidden = try c.decodeIfPresent(Bool.self, forKey: .hidden) ?? false
        hidesFromLibrary = try c.decodeIfPresent(Bool.self, forKey: .hidesFromLibrary) ?? false
        cover = try c.decodeIfPresent(Cover.self, forKey: .cover)
    }
}
