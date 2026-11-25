import SwiftUI
import TPCore
import TPHomeFeature
import TPPhotosFeature

@main
struct TPAppApp: App {
    @StateObject private var networks = NetworkStore()

    #if os(iOS)
    @UIApplicationDelegateAdaptor(PhotoBackupAppDelegate.self) private var appDelegate
    #endif

    init() {
        // `URLSession`s with the default configuration honor `URLCache.shared` under its default
        // cache policy, and `/api/files/thumbnail` already serves cache-friendly
        // `Cache-Control: max-age` responses — but the system's default cache is sized for
        // occasional use, not a gallery grid scrolling back and forth over thousands of already-
        // seen thumbnails. Sizing it explicitly avoids re-fetching those over the network.
        URLCache.shared = URLCache(memoryCapacity: 50 * 1024 * 1024, diskCapacity: 500 * 1024 * 1024)
    }

    var body: some Scene {
        WindowGroup {
            AppRoot(networks: networks)
        }
    }
}

/// No network yet: adding one. Otherwise the network in use, its stores made afresh whenever the
/// app moves to another network (nothing of one network shows in another).
struct AppRoot: View {
    @ObservedObject var networks: NetworkStore
    @State private var started = false

    var body: some View {
        Group {
            if !started {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let id = networks.currentID {
                NetworkScope(networks: networks).id(id)
            } else {
                WelcomeView(networks: networks)
            }
        }
        .task {
            await networks.start()
            started = true
            await FinderLocations.keep(only: networks.networks)
        }
    }
}

/// Everything that belongs to the network in use.
struct NetworkScope: View {
    @ObservedObject var networks: NetworkStore

    @StateObject private var session = AuthSession()
    private let accountClient = CloudAccountClient(client: HTTPClient(.auth))
    private let storeClient = StoreClient(client: HTTPClient(.store))
    private let photosClient: PhotosClient
    @StateObject private var homeStore = HomeStore(client: HomeClient(client: HTTPClient(.home)), wsClient: HomeWebSocketClient())
    @StateObject private var photoGalleryStore: PhotoGalleryStore
    @StateObject private var photoSyncStore: PhotoSyncStore

    init(networks: NetworkStore) {
        self.networks = networks
        let photos = PhotosClient(client: HTTPClient(.store))
        photosClient = photos
        _photoGalleryStore = StateObject(wrappedValue: PhotoGalleryStore(client: photos))
        _photoSyncStore = StateObject(wrappedValue: PhotoSyncStore(client: photos))
    }

    var body: some View {
        RootView(
            networks: networks,
            session: session,
            accountClient: accountClient,
            storeClient: storeClient,
            homeStore: homeStore,
            photosClient: photosClient,
            photoGalleryStore: photoGalleryStore,
            photoSyncStore: photoSyncStore
        )
    }
}
