import Foundation
import TPCore

/// The user's albums (Favorites and their own), kept in step with the server: making one, adding
/// or taking out photos, renaming, hiding it, keeping its photos out of the library, deleting it.
/// Each change replaces the album with the server's answer, so counts and covers stay right.
@MainActor
public final class PhotoAlbumsStore: ObservableObject {
    @Published public private(set) var albums: [PhotoAlbum] = []
    @Published public private(set) var isLoading = false
    @Published public var errorMessage: String?

    private let client: PhotosClient

    public init(client: PhotosClient) {
        self.client = client
    }

    /// Favorites first, then the visible ones by name.
    public var visible: [PhotoAlbum] { albums.filter { !$0.hidden } }
    public var hidden: [PhotoAlbum] { albums.filter(\.hidden) }

    public func album(id: Int) -> PhotoAlbum? { albums.first { $0.id == id } }

    public func load() async {
        isLoading = albums.isEmpty
        defer { isLoading = false }
        do {
            albums = try await client.albums()
        } catch {
            if albums.isEmpty { errorMessage = apiErrorMessage(error) }
        }
    }

    @discardableResult
    public func create(name: String, photoIDs: [Int] = []) async -> PhotoAlbum? {
        do {
            let album = try await client.createAlbum(name: name, photoIDs: photoIDs)
            albums.append(album)
            sort()
            return album
        } catch {
            errorMessage = apiErrorMessage(error)
            return nil
        }
    }

    public func add(_ photoIDs: [Int], to album: PhotoAlbum) async {
        await apply { try await self.client.addPhotos(photoIDs, toAlbum: album.id) }
    }

    public func remove(_ photoIDs: [Int], from album: PhotoAlbum) async {
        await apply { try await self.client.removePhotos(photoIDs, fromAlbum: album.id) }
    }

    public func rename(_ album: PhotoAlbum, to name: String) async {
        await apply { try await self.client.updateAlbum(id: album.id, name: name) }
        sort()
    }

    public func setHidden(_ album: PhotoAlbum, _ hidden: Bool) async {
        await apply { try await self.client.updateAlbum(id: album.id, hidden: hidden) }
    }

    public func setHidesFromLibrary(_ album: PhotoAlbum, _ hides: Bool) async {
        await apply { try await self.client.updateAlbum(id: album.id, hidesFromLibrary: hides) }
    }

    public func delete(_ album: PhotoAlbum) async {
        do {
            try await client.deleteAlbum(id: album.id)
            albums.removeAll { $0.id == album.id }
        } catch {
            errorMessage = apiErrorMessage(error)
        }
    }

    private func apply(_ change: @escaping () async throws -> PhotoAlbum) async {
        do {
            let updated = try await change()
            if let index = albums.firstIndex(where: { $0.id == updated.id }) {
                albums[index] = updated
            } else {
                albums.append(updated)
            }
        } catch {
            errorMessage = apiErrorMessage(error)
        }
    }

    private func sort() {
        albums.sort { a, b in
            if a.isFavorites != b.isFavorites { return a.isFavorites }
            return a.name.localizedCaseInsensitiveCompare(b.name) == .orderedAscending
        }
    }
}
