import SwiftUI
import TPCore

/// An album (or Camera, or Screenshots) on a screen of its own, pushed from Albums like in Photos,
/// so the library tab always shows everything. The same grid and select mode as the library, with
/// the album's own actions in the ••• menu.
struct PhotoListScreen: View {
    let filter: PhotosClient.Filter
    /// For Camera and Screenshots; an album shows its name.
    let fallbackTitle: String
    let client: PhotosClient
    @ObservedObject var albums: PhotoAlbumsStore
    let albumActions: AlbumActions

    @StateObject private var store: PhotoGalleryStore
    @Environment(\.dismiss) private var dismiss
    @State private var detailPhotoID: Int?
    #if os(iOS)
    @State private var selection = PhotoSelection()
    #endif
    @AppStorage(PhotoGridDensity.storageKey) private var columnLevel = 0

    init(filter: PhotosClient.Filter, fallbackTitle: String, client: PhotosClient, albums: PhotoAlbumsStore, albumActions: AlbumActions) {
        self.filter = filter
        self.fallbackTitle = fallbackTitle
        self.client = client
        self.albums = albums
        self.albumActions = albumActions
        _store = StateObject(wrappedValue: PhotoGalleryStore(client: client, filter: filter))
    }

    private var album: PhotoAlbum? {
        filter.albumID.flatMap { albums.album(id: $0) }
    }

    private var title: String {
        album?.displayName ?? fallbackTitle
    }

    var body: some View {
        content
            .navigationTitle(title)
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            .modifier(PhotoSelectionChrome(selection: selection, store: store, client: client))
            #endif
            .toolbar { toolbarContent }
            .task { if store.photos.isEmpty { await store.reload() } }
            // Deleted from its own ••• menu: back to Albums.
            .onChange(of: album == nil) { _, gone in
                if gone, filter.albumID != nil, !albums.albums.isEmpty { dismiss() }
            }
            #if os(macOS)
            .sheet(item: Binding(get: { detailPhotoID.map(DetailID.init) }, set: { detailPhotoID = $0?.id })) { item in
                PhotoDetailView(photos: store.photos.reversed(), initialPhotoID: item.id, client: client) { photo in
                    Task { await store.delete(photo) }
                }
                .frame(minWidth: 720, minHeight: 540)
            }
            #endif
    }

    private struct DetailID: Identifiable { let id: Int }

    @ViewBuilder
    private var content: some View {
        if store.isLoading && store.photos.isEmpty {
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let error = store.errorMessage, store.photos.isEmpty {
            ContentUnavailableView {
                Label("Couldn't Load Photos", systemImage: "exclamationmark.triangle")
            } description: {
                Text(error)
            } actions: {
                Button("Try Again") { Task { await store.reload() } }
            }
        } else if store.photos.isEmpty {
            ContentUnavailableView {
                Label(album != nil ? "No Photos in This Album" : "No Photos", systemImage: "photo.on.rectangle")
            } description: {
                if album != nil {
                    Text("Select photos in All and add them from the ••• menu.")
                }
            }
        } else {
            PhotoGrid(
                photos: store.photos,
                client: client,
                scrollToBottomSignal: 0,
                title: title,
                titleLeadingInset: 72,
                selection: gridSelection,
                onSelect: { detailPhotoID = $0.id },
                onDelete: { photo in Task { await store.delete(photo) } }
            )
            .refreshable { await store.reload() }
        }
    }

    #if os(iOS)
    private var gridSelection: PhotoSelection? { selection }
    #else
    private var gridSelection: PhotoSelection? { nil }
    #endif

    @ToolbarContentBuilder
    private var toolbarContent: some ToolbarContent {
        #if os(iOS)
        // The grid draws the title large over the photos.
        ToolbarItem(placement: .principal) { Color.clear.frame(width: 1, height: 1) }
        if selection.isActive {
            ToolbarItem(placement: .primaryAction) { menu }
            ToolbarSpacer(.fixed, placement: .primaryAction)
            ToolbarItem(placement: .primaryAction) {
                Button("Done Selecting", systemImage: "xmark") { selection.isActive = false }
            }
        } else {
            ToolbarItem(placement: .primaryAction) { menu }
            if !store.photos.isEmpty {
                ToolbarSpacer(.fixed, placement: .primaryAction)
                ToolbarItem(placement: .primaryAction) {
                    Button("Select") { selection.isActive = true }
                }
            }
        }
        #else
        ToolbarItem(placement: .primaryAction) { menu }
        #endif
    }

    private var menu: some View {
        Menu {
            #if os(iOS)
            if selection.isActive {
                PhotoSelectionMenuItems(
                    selection: selection,
                    store: store,
                    albums: albums,
                    currentAlbum: album,
                    actions: albumActions,
                    onAdded: { target, ids in Task { await albums.add(Array(ids), to: target) } },
                    onRemoved: { current, ids in
                        store.drop(ids)
                        Task { await albums.remove(Array(ids), from: current) }
                    }
                )
            } else if let album {
                AlbumMenuItems(album: album, albums: albums, actions: albumActions)
            }
            #else
            if let album {
                AlbumMenuItems(album: album, albums: albums, actions: albumActions)
            }
            #endif
            Section {
                Button("Zoom In", systemImage: "plus.magnifyingglass") {
                    withAnimation(.snappy) { columnLevel = max(columnLevel - 1, 0) }
                }
                .disabled(columnLevel <= 0)
                Button("Zoom Out", systemImage: "minus.magnifyingglass") {
                    withAnimation(.snappy) { columnLevel = min(columnLevel + 1, PhotoGridDensity.columnLevels.count - 1) }
                }
                .disabled(columnLevel >= PhotoGridDensity.columnLevels.count - 1)
            }
        } label: {
            Label("More", systemImage: "ellipsis")
        }
        .menuIndicator(.hidden)
    }
}
