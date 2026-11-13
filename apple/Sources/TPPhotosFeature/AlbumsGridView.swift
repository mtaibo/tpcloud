import ImageIO
import SwiftUI
import TPCore
#if os(iOS)
import UIKit
#endif

/// The Albums section, like the Photos app's: the user's albums (Favorites first) with their
/// covers, the server's filters as "Media Types", and the hidden albums behind "Hidden". Each opens
/// on a screen of its own (`PhotoListScreen`, through the stack's destination for
/// `PhotosClient.Filter`); the library tab keeps showing everything. Holding an album offers its
/// actions.
struct AlbumsListView: View {
    private struct MediaType: Identifiable {
        let title: String
        let systemImage: String
        let filter: PhotosClient.Filter

        var id: PhotosClient.Filter { filter }
    }

    @ObservedObject var albums: PhotoAlbumsStore
    let actions: AlbumActions

    private let mediaTypes: [MediaType] = [
        MediaType(title: "Camera", systemImage: "camera", filter: .camera),
        MediaType(title: "Screenshots", systemImage: "camera.viewfinder", filter: .screenshots),
    ]

    var body: some View {
        List {
            Section("My Albums") {
                ForEach(albums.visible) { album in
                    albumRow(album)
                }
                Button {
                    actions.create([])
                } label: {
                    Label("New Album", systemImage: "plus")
                }
            }

            Section("Media Types") {
                ForEach(mediaTypes) { type in
                    NavigationLink(value: PhotosRoute.photos(type.filter)) {
                        Label(type.title, systemImage: type.systemImage)
                    }
                }
            }

            Section("Utilities") {
                // By value, like the albums: mixing a link that carries its view with the stack's
                // value links made opening an album inside Hidden push Hidden again on top of it.
                NavigationLink(value: PhotosRoute.hidden) {
                    Label("Hidden", systemImage: "eye.slash")
                }
            }
        }
        .refreshable { await albums.load() }
    }

    private func albumRow(_ album: PhotoAlbum) -> some View {
        AlbumRow(album: album)
            .contextMenu { AlbumMenuItems(album: album, albums: albums, actions: actions) }
    }
}

/// The screens the Photos tab pushes on top of All (or Shared): Albums, Hidden, and an album or
/// media type of its own. Being pushed, each gets the system back button and the swipe from the
/// left edge.
enum PhotosRoute: Hashable {
    case albums
    case hidden
    case photos(PhotosClient.Filter)
}

/// The albums taken out of the albums list, each still a tap away.
struct HiddenAlbumsView: View {
    @ObservedObject var albums: PhotoAlbumsStore
    let actions: AlbumActions

    var body: some View {
        Group {
            if albums.hidden.isEmpty {
                ContentUnavailableView {
                    Label("No Hidden Albums", systemImage: "eye.slash")
                } description: {
                    Text("Hide an album from its ••• menu to keep it here instead of in My Albums.")
                }
            } else {
                List {
                    ForEach(albums.hidden) { album in
                        AlbumRow(album: album)
                            .contextMenu { AlbumMenuItems(album: album, albums: albums, actions: actions) }
                    }
                }
            }
        }
        .navigationTitle("Hidden")
    }
}

private struct AlbumRow: View {
    let album: PhotoAlbum

    var body: some View {
        NavigationLink(value: PhotosRoute.photos(.album(album.id))) {
            HStack(spacing: 12) {
                AlbumCover(album: album)
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 6) {
                        Text(album.displayName)
                            .foregroundStyle(.primary)
                        if album.hidesFromLibrary {
                            Image(systemName: "rectangle.stack.badge.minus")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .accessibilityLabel("Not in Library")
                        }
                    }
                    Text(album.count == 1 ? "1 photo" : "\(album.count) photos")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .monospacedDigit()
                }
            }
        }
    }
}

/// The album's newest photo, from the thumbnails kept on this device, or a symbol.
private struct AlbumCover: View {
    let album: PhotoAlbum
    @State private var image: CGImage?

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .fill(.quaternary)
            if let image {
                Image(decorative: image, scale: 1)
                    .resizable()
                    .scaledToFill()
            } else {
                Image(systemName: album.isFavorites ? "heart.fill" : "rectangle.stack")
                    .foregroundStyle(album.isFavorites ? AnyShapeStyle(.pink) : AnyShapeStyle(.secondary))
            }
        }
        .frame(width: 48, height: 48)
        .clipShape(.rect(cornerRadius: 8, style: .continuous))
        .task(id: album.cover?.checksum) {
            guard let checksum = album.cover?.checksum else { image = nil; return }
            if let cached = PhotoThumbnailCache.anyCachedCGImage(forChecksum: checksum) {
                image = cached
                return
            }
            let url = PhotoThumbnailCache.fileURL(forChecksum: checksum)
            image = await Task.detached(priority: .utility) {
                guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else { return nil as CGImage? }
                let options: [CFString: Any] = [
                    kCGImageSourceCreateThumbnailFromImageAlways: true,
                    kCGImageSourceCreateThumbnailWithTransform: true,
                    kCGImageSourceThumbnailMaxPixelSize: 144,
                ]
                return CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary)
            }.value
        }
    }
}

/// What the album screens and the grid's ••• menu need from the gallery to act on albums: the
/// prompts (name, confirmation) live there.
struct AlbumActions {
    /// Asks for a name and makes an album with these photos in it.
    let create: ([Int]) -> Void
    let rename: (PhotoAlbum) -> Void
    let delete: (PhotoAlbum) -> Void
}

/// An album's actions, for its context menu and for the ••• menu while it's on screen.
struct AlbumMenuItems: View {
    let album: PhotoAlbum
    @ObservedObject var albums: PhotoAlbumsStore
    let actions: AlbumActions

    var body: some View {
        if !album.isFavorites {
            Section {
                if album.hidesFromLibrary {
                    Button("Show in Library", systemImage: "rectangle.stack.badge.plus") {
                        Task { await albums.setHidesFromLibrary(album, false) }
                    }
                } else {
                    Button("Remove from Library", systemImage: "rectangle.stack.badge.minus") {
                        Task { await albums.setHidesFromLibrary(album, true) }
                    }
                }
                if album.hidden {
                    Button("Unhide Album", systemImage: "eye") {
                        Task { await albums.setHidden(album, false) }
                    }
                } else {
                    Button("Hide Album", systemImage: "eye.slash") {
                        Task { await albums.setHidden(album, true) }
                    }
                }
                Button("Rename Album", systemImage: "pencil") { actions.rename(album) }
            }
            Section {
                Button("Delete Album", systemImage: "trash", role: .destructive) { actions.delete(album) }
            }
        }
    }
}
