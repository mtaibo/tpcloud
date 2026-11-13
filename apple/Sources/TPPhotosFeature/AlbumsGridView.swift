import ImageIO
import SwiftUI
import TPCore
#if os(iOS)
import UIKit
#endif

/// The Albums section, like the Photos app's: the user's albums (Favorites first) with their
/// covers, the server's filters as "Media Types", and the hidden albums behind "Hidden". Picking
/// one jumps back to the grid, showing it. Holding an album offers its actions.
struct AlbumsListView: View {
    private struct MediaType: Identifiable {
        let title: String
        let systemImage: String
        let filter: PhotosClient.Filter

        var id: PhotosClient.Filter { filter }
    }

    @ObservedObject var albums: PhotoAlbumsStore
    let selected: PhotosClient.Filter
    let onSelect: (PhotosClient.Filter) -> Void
    let actions: AlbumActions

    private let mediaTypes: [MediaType] = [
        MediaType(title: "All Photos", systemImage: "photo.on.rectangle", filter: .all),
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
                    Button {
                        onSelect(type.filter)
                    } label: {
                        HStack {
                            Label(type.title, systemImage: type.systemImage)
                                .foregroundStyle(.primary)
                            Spacer()
                            checkmark(type.filter == selected)
                        }
                        .contentShape(.rect)
                    }
                    .accessibilityAddTraits(type.filter == selected ? .isSelected : [])
                }
            }

            Section("Utilities") {
                NavigationLink {
                    HiddenAlbumsView(albums: albums, selected: selected, onSelect: onSelect, actions: actions)
                } label: {
                    HStack {
                        Label("Hidden", systemImage: "eye.slash")
                        Spacer()
                        if !albums.hidden.isEmpty {
                            Text("\(albums.hidden.count)")
                                .foregroundStyle(.secondary)
                                .monospacedDigit()
                        }
                    }
                }
            }
        }
        .refreshable { await albums.load() }
    }

    private func albumRow(_ album: PhotoAlbum) -> some View {
        AlbumRow(album: album, isSelected: selected == .album(album.id)) {
            onSelect(.album(album.id))
        }
        .contextMenu { AlbumMenuItems(album: album, albums: albums, actions: actions) }
    }
}

/// The albums taken out of the albums list, each still a tap away.
struct HiddenAlbumsView: View {
    @ObservedObject var albums: PhotoAlbumsStore
    let selected: PhotosClient.Filter
    let onSelect: (PhotosClient.Filter) -> Void
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
                        AlbumRow(album: album, isSelected: selected == .album(album.id)) {
                            onSelect(.album(album.id))
                        }
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
    let isSelected: Bool
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
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
                Spacer()
                checkmark(isSelected)
            }
            .contentShape(.rect)
        }
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }
}

@ViewBuilder
private func checkmark(_ on: Bool) -> some View {
    if on {
        Image(systemName: "checkmark")
            .foregroundStyle(.tint)
            .accessibilityHidden(true)
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
