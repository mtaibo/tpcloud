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

    /// Photos that just left the library (put in, or now in, an album that keeps its photos out
    /// of it): the library takes them off at once instead of waiting for its next sync.
    public var onPhotosLeftLibrary: (Set<Int>) -> Void = { _ in }
    /// Photos may have come back to the library: it syncs again.
    public var onLibraryNeedsReload: () -> Void = {}

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
        guard let updated = await apply({ try await self.client.addPhotos(photoIDs, toAlbum: album.id) }) else { return }
        if updated.hidesFromLibrary { onPhotosLeftLibrary(Set(photoIDs)) }
    }

    public func remove(_ photoIDs: [Int], from album: PhotoAlbum) async {
        guard let updated = await apply({ try await self.client.removePhotos(photoIDs, fromAlbum: album.id) }) else { return }
        if updated.hidesFromLibrary { onLibraryNeedsReload() }
    }

    public func rename(_ album: PhotoAlbum, to name: String) async {
        await apply { try await self.client.updateAlbum(id: album.id, name: name) }
        sort()
    }

    public func setHidden(_ album: PhotoAlbum, _ hidden: Bool) async {
        await apply { try await self.client.updateAlbum(id: album.id, hidden: hidden) }
    }

    public func setHidesFromLibrary(_ album: PhotoAlbum, _ hides: Bool) async {
        guard await apply({ try await self.client.updateAlbum(id: album.id, hidesFromLibrary: hides) }) != nil else { return }
        if hides {
            if let ids = await photoIDs(inAlbum: album.id) { onPhotosLeftLibrary(ids) } else { onLibraryNeedsReload() }
        } else {
            onLibraryNeedsReload()
        }
    }

    public func delete(_ album: PhotoAlbum) async {
        do {
            try await client.deleteAlbum(id: album.id)
            albums.removeAll { $0.id == album.id }
            if album.hidesFromLibrary { onLibraryNeedsReload() }
        } catch {
            errorMessage = apiErrorMessage(error)
        }
    }

    /// Every photo id in an album (albums are small next to the library: a few pages at most).
    private func photoIDs(inAlbum id: Int) async -> Set<Int>? {
        var ids: Set<Int> = []
        var cursor: String?
        do {
            repeat {
                let page = try await client.list(filter: .album(id), cursor: cursor, limit: 200)
                ids.formUnion(page.items.map(\.id))
                cursor = page.nextCursor
            } while cursor != nil
            return ids
        } catch {
            return nil
        }
    }

    @discardableResult
    private func apply(_ change: @escaping () async throws -> PhotoAlbum) async -> PhotoAlbum? {
        do {
            let updated = try await change()
            if let index = albums.firstIndex(where: { $0.id == updated.id }) {
                albums[index] = updated
            } else {
                albums.append(updated)
            }
            return updated
        } catch {
            errorMessage = apiErrorMessage(error)
            return nil
        }
    }

    private func sort() {
        albums.sort { a, b in
            if a.isFavorites != b.isFavorites { return a.isFavorites }
            return a.name.localizedCaseInsensitiveCompare(b.name) == .orderedAscending
        }
    }
}
