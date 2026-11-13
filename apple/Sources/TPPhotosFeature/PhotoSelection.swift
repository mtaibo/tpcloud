import Observation
import SwiftUI
import TPCore
#if os(iOS)
import UIKit
#endif

/// Select mode, shared by the gallery's bars (SwiftUI) and the grid (UIKit), which does the
/// tapping and dragging. Observable property by property, so dragging across photos only
/// redraws what shows the count.
@MainActor
@Observable
final class PhotoSelection {
    var isActive = false {
        didSet { if !isActive { ids.removeAll() } }
    }
    var ids: Set<Int> = []
    /// Bumped by "Show Selected": the grid opens the viewer on the selected photos.
    var showSelectedSignal = 0
}

#if os(iOS)

/// Select mode's bottom bar, in place of the tab bar, like Photos': share, how many are selected
/// (tap to see them), delete.
struct PhotoSelectionBar: View {
    let selection: PhotoSelection
    let isWorking: Bool
    let onShare: () -> Void
    let onDelete: () -> Void

    var body: some View {
        let count = selection.ids.count
        HStack(spacing: 12) {
            Button(action: onShare) {
                Group {
                    if isWorking { ProgressView() } else { Image(systemName: "square.and.arrow.up") }
                }
                .frame(width: 24, height: 24)
            }
            .buttonStyle(.glass)
            .buttonBorderShape(.circle)
            .controlSize(.large)
            .disabled(count == 0 || isWorking)
            .accessibilityLabel("Share")

            Spacer(minLength: 0)
            Button {
                if count > 0 { selection.showSelectedSignal += 1 }
            } label: {
                // Never wider than the room it has: with a fixed size, a count growing a digit made
                // the bar a few points wider than the screen, SwiftUI widened the whole screen to
                // fit it, and the grid's tiles grew with it — the grid jumped by rows mid-selection.
                Text(count == 0 ? "Select Items" : "Show Selected (\(count))")
                    .font(.body.weight(.semibold))
                    .monospacedDigit()
                    .lineLimit(1)
                    .minimumScaleFactor(0.7)
                    .padding(.horizontal, 6)
            }
            .buttonStyle(.glass)
            .controlSize(.large)
            .allowsHitTesting(count > 0)
            Spacer(minLength: 0)

            Button(role: .destructive, action: onDelete) {
                Image(systemName: "trash")
                    .frame(width: 24, height: 24)
            }
            .buttonStyle(.glass)
            .buttonBorderShape(.circle)
            .controlSize(.large)
            .disabled(count == 0 || isWorking)
            .accessibilityLabel("Delete")
        }
        .padding(.horizontal, 20)
        .padding(.bottom, 4)
        .frame(maxWidth: .infinity)
    }
}

/// Select mode's chrome around a grid, the same on the library and in an album: the tab bar
/// swapped for `PhotoSelectionBar`, sharing the originals, and deleting with a confirmation. It
/// only reads whether select mode is on; the count is read where it's shown (the bar) and when a
/// button is pressed, so selecting more photos never re-renders the screen around the grid.
struct PhotoSelectionChrome: ViewModifier {
    let selection: PhotoSelection
    let store: PhotoGalleryStore
    let client: PhotosClient

    @State private var deleteCount = 0
    @State private var isConfirmingDelete = false
    @State private var isPreparingShare = false
    @State private var shareURLs: [URL] = []
    @State private var isSharing = false

    func body(content: Content) -> some View {
        content
            .toolbar(selection.isActive ? .hidden : .visible, for: .tabBar)
            .safeAreaInset(edge: .bottom, spacing: 0) {
                if selection.isActive {
                    PhotoSelectionBar(
                        selection: selection,
                        isWorking: isPreparingShare,
                        onShare: share,
                        onDelete: {
                            deleteCount = selection.ids.count
                            isConfirmingDelete = deleteCount > 0
                        }
                    )
                    .transition(.move(edge: .bottom).combined(with: .opacity))
                }
            }
            .animation(.snappy, value: selection.isActive)
            .confirmationDialog(
                deleteCount == 1 ? "Delete this photo?" : "Delete \(deleteCount) photos?",
                isPresented: $isConfirmingDelete,
                titleVisibility: .visible
            ) {
                Button(deleteCount == 1 ? "Delete Photo" : "Delete \(deleteCount) Photos", role: .destructive) {
                    let photos = selected()
                    selection.isActive = false
                    Task { await store.delete(photos) }
                }
            } message: {
                Text("They'll be removed from your server. The copies on this device aren't affected.")
            }
            .sheet(isPresented: $isSharing) {
                PhotoShareSheet(items: shareURLs)
                    .presentationDetents([.medium, .large])
                    .ignoresSafeArea()
            }
    }

    private func selected() -> [Photo] {
        let ids = selection.ids
        return store.photos.reversed().filter { ids.contains($0.id) }
    }

    private func share() {
        let photos = selected()
        guard !photos.isEmpty else { return }
        isPreparingShare = true
        Task {
            let urls = await PhotoShareDownloads.originals(of: photos, client: client)
            isPreparingShare = false
            guard !urls.isEmpty else { return }
            shareURLs = urls
            isSharing = true
        }
    }
}

/// The ••• menu's part for select mode: Select All / Deselect All, Add to Album (an album or a new
/// one) and, inside an album, Remove from Album. Its own view so only it follows the count.
struct PhotoSelectionMenuItems: View {
    let selection: PhotoSelection
    let store: PhotoGalleryStore
    @ObservedObject var albums: PhotoAlbumsStore
    /// The album on screen, if any (it can't be added to itself; its photos can be taken out).
    let currentAlbum: PhotoAlbum?
    let actions: AlbumActions
    let onAdded: (PhotoAlbum, Set<Int>) -> Void
    let onRemoved: (PhotoAlbum, Set<Int>) -> Void

    var body: some View {
        let isEmpty = selection.ids.isEmpty
        Section {
            Button("Select All", systemImage: "checkmark.circle") {
                selection.ids = Set(store.photos.map(\.id))
            }
            Button("Deselect All", systemImage: "circle") { selection.ids.removeAll() }
                .disabled(isEmpty)
        }
        Section {
            Menu("Add to Album", systemImage: "rectangle.stack.badge.plus") {
                Button("New Album…", systemImage: "plus") {
                    let ids = selection.ids
                    selection.isActive = false
                    actions.create(Array(ids))
                }
                Section {
                    ForEach(albums.albums.filter { $0.id != currentAlbum?.id }) { album in
                        Button(album.displayName, systemImage: album.isFavorites ? "heart" : (album.hidden ? "eye.slash" : "rectangle.stack")) {
                            let ids = selection.ids
                            selection.isActive = false
                            onAdded(album, ids)
                        }
                    }
                }
            }
            .disabled(isEmpty)
            if let album = currentAlbum {
                Button("Remove from Album", systemImage: "rectangle.stack.badge.minus") {
                    let ids = selection.ids
                    selection.isActive = false
                    onRemoved(album, ids)
                }
                .disabled(isEmpty)
            }
        }
    }
}

/// The system share sheet, for the selected photos' originals.
struct PhotoShareSheet: UIViewControllerRepresentable {
    let items: [URL]

    func makeUIViewController(context: Context) -> UIActivityViewController {
        UIActivityViewController(activityItems: items, applicationActivities: nil)
    }

    func updateUIViewController(_ controller: UIActivityViewController, context: Context) {}
}

enum PhotoShareDownloads {
    /// Downloads the originals (a few at a time) into a fresh temporary folder, under their own
    /// names, for the share sheet.
    static func originals(of photos: [Photo], client: PhotosClient) async -> [URL] {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("Share-\(UUID().uuidString)", isDirectory: true)
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        return await withTaskGroup(of: (Int, URL?).self) { group in
            var results: [(Int, URL?)] = []
            var iterator = photos.enumerated().makeIterator()
            func addNext() {
                guard let (index, photo) = iterator.next() else { return }
                let url = client.originalURL(for: photo)
                let name = photo.originalFilename.isEmpty ? "\(photo.id).jpg" : photo.originalFilename
                let destination = folder.appendingPathComponent("\(index)-\(name)")
                group.addTask {
                    guard let (temp, response) = try? await URLSession.shared.download(from: url),
                          let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else { return (index, nil) }
                    try? FileManager.default.moveItem(at: temp, to: destination)
                    return (index, destination)
                }
            }
            for _ in 0..<4 { addNext() }
            for await result in group {
                results.append(result)
                addNext()
            }
            return results.sorted { $0.0 < $1.0 }.compactMap(\.1)
        }
    }
}
#endif
