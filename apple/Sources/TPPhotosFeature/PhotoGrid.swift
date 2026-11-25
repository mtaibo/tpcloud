import SwiftUI
import TPCore
import TPUI
#if os(iOS)
import UIKit
#endif

/// The photo grid's density levels. One pinch steps one level, like the native Photos app.
/// No single-column level: tiles only ever show the small on-device thumbnail, which would look
/// blurry stretched to the full width — opening a photo is how to see it large.
/// Every count is odd, so zooming about the middle column keeps the columns of any two levels
/// lined up (see `PhotoGridZoomController`).
enum PhotoGridDensity {
    static let columnLevels = [3, 5, 9, 15, 31]
    static let storageKey = "photos.columnLevel.v2"

    /// Performance tests start the grid at a given level (`-photosStartLevel <n>`), written
    /// once into the real defaults: pinning it through the argument domain would keep the grid
    /// from saving levels it zooms to.
    @MainActor static func applyTestStartLevel() {
        guard !appliedTestStartLevel else { return }
        appliedTestStartLevel = true
        let arguments = ProcessInfo.processInfo.arguments
        guard let index = arguments.firstIndex(of: "-photosStartLevel"), index + 1 < arguments.count,
              let level = Int(arguments[index + 1]) else { return }
        UserDefaults.standard.set(level, forKey: storageKey)
    }

    @MainActor private static var appliedTestStartLevel = false

    static func columns(forLevel level: Int) -> Int {
        columnLevels[min(max(level, 0), columnLevels.count - 1)]
    }

    /// Where tapping a photo zooms to instead of opening it: dense grids are for finding a spot,
    /// so a tap closes in on it — the densest level to 15 columns, anything denser than 5 to 5 —
    /// and photos open from 5 columns or fewer. `nil` means open the photo.
    static func tapZoomLevel(fromLevel level: Int) -> Int? {
        let columns = columns(forLevel: level)
        let target = columns > 15 ? 15 : columns > 5 ? 5 : nil
        return target.flatMap { columnLevels.firstIndex(of: $0) }
    }
}

/// Newest-at-the-bottom grid with pinch-to-zoom. Its own view (rather than part of
/// `PhotoGalleryView`) so status/menu state changes up there don't recompute thousands of rows.
/// On iOS it's `PhotoCollectionGrid`, on the Mac `MacPhotoGrid`: both layer grids, not SwiftUI views per tile.
struct PhotoGrid: View {
    /// Newest first, as the server pages them.
    let photos: [Photo]
    let client: PhotosClient
    /// Bumped when the Photos tab is tapped again, to jump to the newest photos at the bottom.
    let scrollToBottomSignal: Int
    /// "All", "Shared" or the album's name, drawn large over the grid on iPhone and iPad.
    let title: String
    /// Where the large title starts: past the back button on a pushed screen (an album).
    var titleLeadingInset: CGFloat = 20
    /// Select mode (iPhone and iPad).
    let selection: PhotoSelection?
    /// Opens a photo on the Mac; on iPhone and iPad the grid opens its own viewer.
    let onSelect: (Photo) -> Void
    let onDelete: (Photo) -> Void
    /// For the viewer's heart (Favorites); not observed, the grid doesn't change with it.
    var albums: PhotoAlbumsStore?

    @AppStorage(PhotoGridDensity.storageKey) private var columnLevel = 0

    var body: some View {
        #if os(iOS)
        PhotoCollectionGrid(
            photos: photos,
            client: client,
            columnLevel: $columnLevel,
            scrollToBottomSignal: scrollToBottomSignal,
            title: title,
            titleLeadingInset: titleLeadingInset,
            selection: selection,
            onDelete: onDelete,
            albums: albums
        )
        // Scrolls under the bars; the collection view insets itself by the safe area.
        .ignoresSafeArea()
        #else
        MacPhotoGrid(
            photos: photos,
            client: client,
            columnLevel: $columnLevel,
            scrollToBottomSignal: scrollToBottomSignal,
            onOpen: onSelect,
            onDelete: onDelete
        )
        #endif
    }

    static func accessibilityLabel(for photo: Photo) -> String {
        var parts = [photo.isScreenshot ? "Screenshot" : "Photo"]
        if let takenAt = photo.takenAt.flatMap(PhotoDates.parse) {
            parts.append(takenAt.formatted(date: .long, time: .shortened))
        }
        return parts.joined(separator: ", ")
    }
}

