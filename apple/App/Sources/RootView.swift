import SwiftUI
import TPCore
import TPCloudFeature
import TPStoreFeature
import TPHomeFeature
import TPPhotosFeature

private enum AppTab: Hashable {
    case store, photos, home, more
}

struct RootView: View {
    @ObservedObject var networks: NetworkStore
    @ObservedObject var session: AuthSession
    let accountClient: CloudAccountClient
    let storeClient: StoreClient
    // Plain references, not @ObservedObject: each feature view observes its own store. Observing
    // them here re-rendered the whole app — every tab — on each photo page that loaded during the
    // background drain and on every Home WebSocket event, which made launch and Photos crawl.
    let homeStore: HomeStore
    let photosClient: PhotosClient
    let photoGalleryStore: PhotoGalleryStore
    let photoSyncStore: PhotoSyncStore

    /// `-openTab photos` (or home, more) at launch opens that tab, for checking a screen straight away.
    @State private var selectedTab: AppTab = {
        switch UserDefaults.standard.string(forKey: "openTab") {
        case "photos": .photos
        case "home": .home
        case "more": .more
        default: .store
        }
    }()
    @State private var moreResetSignal = 0
    @State private var photosScrollSignal = 0

    var body: some View {
        Group {
            switch session.state {
            case .checkingSession:
                ProgressView()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            case .signedOut, .totpRequired:
                LoginView(session: session, networks: networks)
            case .signedIn(let profile):
                tabs(for: profile)
            }
        }
        .task {
            await session.restoreIfPossible()
            await homeStore.start()
        }
        #if os(macOS)
        // While the app is open, the Finder's locations look for changes on their hosts every 30 s.
        .task {
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(30))
                FinderLocations.refresh()
            }
        }
        #endif
        .onChange(of: session.state) { _, newState in
            // The background upload engine has no SwiftUI view hierarchy to read `profile.email`
            // from when iOS wakes it directly (BGProcessingTask, or a background URLSession
            // completion) — this is the one place that keeps its persisted copy in sync with the
            // signed-in user, as the only two events that matter (signing in, signing out) happen.
            switch newState {
            case .signedIn(let profile):
                if let network = networks.current {
                    Task { await FinderLocations.signedIn(network, email: profile.email) }
                }
                #if os(iOS)
                PhotoBackgroundUploader.rememberSignedIn(ownerEmail: profile.email)
                PhotoBackupScheduler.scheduleNext()
                Task { await PhotoBackgroundUploader.shared.refill() }
                #endif
            case .signedOut:
                selectedTab = .store
                #if os(iOS)
                PhotoBackgroundUploader.forgetSignedIn()
                #endif
            case .checkingSession, .totpRequired:
                break
            }
        }
    }

    private func tabs(for profile: UserProfile) -> some View {
        TabView(selection: tabSelection) {
            Tab("Store", systemImage: "folder", value: AppTab.store) {
                StoreRootView(
                    storeClient: storeClient,
                    rootPath: "users/\(profile.email)"
                )
                #if os(macOS)
                .safeAreaInset(edge: .top, spacing: 0) { FinderLocationBanner() }
                #endif
            }
            Tab("Photos", systemImage: "photo.on.rectangle", value: AppTab.photos) {
                PhotoGalleryView(
                    store: photoGalleryStore,
                    syncStore: photoSyncStore,
                    client: photosClient,
                    ownerEmail: profile.email,
                    isAdmin: profile.isAdmin,
                    scrollToBottomSignal: photosScrollSignal
                )
            }
            Tab("Home", systemImage: "house", value: AppTab.home) {
                HomeRootView(store: homeStore)
            }
            Tab("More", systemImage: "ellipsis", value: AppTab.more) {
                MoreView(
                    networks: networks,
                    session: session,
                    profile: profile,
                    accountClient: accountClient,
                    resetSignal: moreResetSignal
                )
            }
        }
        // A sidebar on iPad and Mac, the bottom tab bar on iPhone.
        .tabViewStyle(.sidebarAdaptable)
        #if os(iOS)
        // The tab bar stays as it is while scrolling: Miguel found the shrinking one odd.
        .tabBarMinimizeBehavior(.never)
        #endif
    }

    /// Re-selecting the current tab pops it back to its root, like every system app.
    private var tabSelection: Binding<AppTab> {
        Binding(
            get: { selectedTab },
            set: { newValue in
                if newValue == selectedTab {
                    switch newValue {
                    case .more: moreResetSignal += 1
                    // Photos keeps its newest photos at the bottom, so "back to the start" is down.
                    case .photos: photosScrollSignal += 1
                    // Store pops itself back to Browse; resetting it here too raced that pop.
                    case .store, .home: break
                    }
                }
                selectedTab = newValue
            }
        )
    }
}
