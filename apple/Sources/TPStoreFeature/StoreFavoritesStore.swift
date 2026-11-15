import Foundation
import TPCore

extension StoreFavorite {
    var folder: StoreFolder {
        StoreFolder(location: location, path: path, title: label)
    }
}

/// The Store's favorite folders, kept on the server (`/api/favorites`) so the app and the web on
/// every browser share one list. A copy is kept on the device to show at once while the list
/// loads; changes show straight away and go back if the server turns them down.
@MainActor
final class StoreFavoritesStore: ObservableObject {
    @Published private(set) var favorites: [StoreFavorite] = []

    private let storeClient: StoreClient
    /// The list each device used to keep on its own, before favorites were synced: handed to the
    /// server once, then deleted.
    private static let legacyKey = "tpcloud-favourites"
    private static let cacheKey = "store.favorites"

    init(storeClient: StoreClient) {
        self.storeClient = storeClient
        if let data = UserDefaults.standard.data(forKey: Self.cacheKey),
           let cached = try? JSONDecoder().decode([StoreFavorite].self, from: data) {
            favorites = cached
        }
    }

    /// Fetches the list (and hands over the old on-device one the first time).
    func refresh() async {
        do {
            if let legacy = Self.legacyFavorites(), !legacy.isEmpty {
                do {
                    set(try await storeClient.addFavorites(legacy))
                    UserDefaults.standard.removeObject(forKey: Self.legacyKey)
                    return
                } catch APIError.server(let status, _) where (400..<500).contains(status) {
                    // A folder it can no longer reach: drop the old list rather than retry forever.
                    UserDefaults.standard.removeObject(forKey: Self.legacyKey)
                }
            }
            set(try await storeClient.favorites())
        } catch {
            // Offline or signed out: keeps showing the last list it had.
        }
    }

    func contains(_ folder: StoreFolder) -> Bool {
        favorites.contains { $0.id == StoreFavorite.id(location: folder.location, path: folder.path) }
    }

    func toggle(_ folder: StoreFolder) {
        let id = StoreFavorite.id(location: folder.location, path: folder.path)
        if favorites.contains(where: { $0.id == id }) {
            remove(id)
            return
        }
        let before = favorites
        favorites.append(StoreFavorite(location: folder.location, path: folder.path, label: folder.title))
        Task {
            do {
                set(try await storeClient.addFavorites([(folder.location, folder.path)]))
            } catch {
                favorites = before
            }
        }
    }

    func remove(_ id: String) {
        guard let favorite = favorites.first(where: { $0.id == id }) else { return }
        let before = favorites
        favorites.removeAll { $0.id == id }
        Task {
            do {
                set(try await storeClient.removeFavorite(path: favorite.path, location: favorite.location))
            } catch {
                favorites = before
            }
        }
    }

    private func set(_ list: [StoreFavorite]) {
        favorites = list
        if let data = try? JSONEncoder().encode(list) {
            UserDefaults.standard.set(data, forKey: Self.cacheKey)
        }
    }

    private struct LegacyFavorite: Decodable {
        let location: StoreLocation
        let path: String
    }

    private static func legacyFavorites() -> [(location: StoreLocation, path: String)]? {
        guard let data = UserDefaults.standard.data(forKey: legacyKey),
              let decoded = try? JSONDecoder().decode([LegacyFavorite].self, from: data) else { return nil }
        return decoded.map { ($0.location, $0.path) }
    }
}
