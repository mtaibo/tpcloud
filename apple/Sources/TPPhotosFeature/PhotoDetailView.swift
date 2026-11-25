import ImageIO
import SwiftUI
import TPCore
import TPUI
#if os(iOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

#if os(macOS)
/// The Mac's viewer, in the window itself (like Photos, not a sheet): one photo at a time over
/// black, ← and → (or a two-finger swipe) to move, Esc or the back button to return to the grid,
/// a pinch or a double click to zoom, I for the info, Delete to delete. Shows the thumbnail at
/// once and the host's 2048 px preview as soon as it's in; the neighbours are fetched ahead.
/// (iPhone and iPad use `PhotoViewerController`.)
public struct PhotoDetailView: View {
    let photos: [Photo]
    let client: PhotosClient
    let onDelete: (Photo) -> Void
    let onClose: () -> Void

    @State private var currentID: Int
    @State private var showsInfo = false
    @State private var photoPendingDelete: Photo?
    @State private var deletedIDs: Set<Int> = []
    @State private var zoom: CGFloat = 1
    @State private var swipe: CGFloat = 0
    @FocusState private var focused: Bool

    /// `photos` oldest first, as the grid shows them.
    public init(photos: [Photo], initialPhotoID: Int, client: PhotosClient, onDelete: @escaping (Photo) -> Void, onClose: @escaping () -> Void) {
        self.photos = photos
        self.client = client
        self.onDelete = onDelete
        self.onClose = onClose
        _currentID = State(initialValue: initialPhotoID)
    }

    private var visible: [Photo] { deletedIDs.isEmpty ? photos : photos.filter { !deletedIDs.contains($0.id) } }
    private var index: Int? { visible.firstIndex { $0.id == currentID } }
    private var current: Photo? { index.map { visible[$0] } }

    public var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            if let current {
                MacPhotoPage(photo: current, client: client, zoom: $zoom)
                    .id(current.id)
            }
        }
        .overlay(alignment: .top) { bar }
        .contentShape(.rect)
        .focusable()
        .focusEffectDisabled()
        .focused($focused)
        .onAppear { focused = true }
        .onKeyPress(.leftArrow) { step(-1); return .handled }
        .onKeyPress(.rightArrow) { step(1); return .handled }
        .onKeyPress(.escape) { onClose(); return .handled }
        .onKeyPress(.space) { onClose(); return .handled }
        .onKeyPress(characters: CharacterSet(charactersIn: "iI")) { _ in showsInfo.toggle(); return .handled }
        .onKeyPress(.delete) { photoPendingDelete = current; return .handled }
        .onKeyPress(.deleteForward) { photoPendingDelete = current; return .handled }
        .gesture(swipeGesture, including: zoom > 1 ? .subviews : .all)
        .task(id: currentID) { prefetchNeighbours() }
        .popover(isPresented: $showsInfo, attachmentAnchor: .point(.topTrailing), arrowEdge: .top) {
            if let current {
                PhotoInfoView(photo: current).frame(width: 340, height: 420)
            }
        }
        .confirmationDialog(
            "Delete this photo?",
            isPresented: Binding(get: { photoPendingDelete != nil }, set: { if !$0 { photoPendingDelete = nil } }),
            titleVisibility: .visible,
            presenting: photoPendingDelete
        ) { photo in
            Button("Delete Photo", role: .destructive) { delete(photo) }
        } message: { _ in
            Text("It's deleted from your TPCloud for good. The copy in this Mac's Photos library isn't affected.")
        }
        .preferredColorScheme(.dark)
    }

    private var bar: some View {
        HStack(spacing: 12) {
            Button(action: onClose) {
                Label("All Photos", systemImage: "chevron.left")
            }
            .keyboardShortcut(.cancelAction)
            Spacer()
            if let current {
                VStack(spacing: 1) {
                    Text(Self.title(for: current) ?? current.originalFilename).font(.headline)
                    if let index { Text("\(index + 1) of \(visible.count)").font(.caption).foregroundStyle(.secondary) }
                }
            }
            Spacer()
            HStack(spacing: 4) {
                Button { step(-1) } label: { Image(systemName: "chevron.backward") }
                    .disabled(index == 0)
                    .help("Previous (←)")
                Button { step(1) } label: { Image(systemName: "chevron.forward") }
                    .disabled(index == visible.count - 1)
                    .help("Next (→)")
                Button { showsInfo.toggle() } label: { Image(systemName: "info.circle") }
                    .help("Info (I)")
                Button { photoPendingDelete = current } label: { Image(systemName: "trash") }
                    .help("Delete")
            }
        }
        .buttonStyle(.glass)
        .controlSize(.large)
        .padding(14)
        .background(LinearGradient(colors: [.black.opacity(0.6), .clear], startPoint: .top, endPoint: .bottom).allowsHitTesting(false))
    }

    /// A two-finger swipe on the trackpad (or a drag) moves to the neighbour.
    private var swipeGesture: some Gesture {
        DragGesture(minimumDistance: 30)
            .onEnded { value in
                guard zoom <= 1, abs(value.translation.width) > abs(value.translation.height) else { return }
                step(value.translation.width < 0 ? 1 : -1)
            }
    }

    private func step(_ delta: Int) {
        guard let index else { return }
        let next = index + delta
        guard visible.indices.contains(next) else { return }
        zoom = 1
        currentID = visible[next].id
    }

    private func prefetchNeighbours() {
        guard let index else { return }
        for offset in [1, -1, 2, -2] where visible.indices.contains(index + offset) {
            MacPreviewLoader.shared.prefetch(visible[index + offset], client: client)
        }
    }

    private func delete(_ photo: Photo) {
        let remaining = visible
        guard let at = remaining.firstIndex(of: photo) else { return }
        onDelete(photo)
        guard remaining.count > 1 else { return onClose() }
        let next = at + 1 < remaining.count ? remaining[at + 1] : remaining[at - 1]
        deletedIDs.insert(photo.id)
        currentID = next.id
    }

    private static func title(for photo: Photo) -> String? {
        photo.takenAt.flatMap(PhotoDates.parse)?.formatted(date: .long, time: .shortened)
    }
}

/// One photo, fitted to the window; pinch or double click to zoom, drag to pan while zoomed.
private struct MacPhotoPage: View {
    let photo: Photo
    let client: PhotosClient
    @Binding var zoom: CGFloat

    @State private var preview: CGImage?
    @GestureState private var pinch: CGFloat = 1
    @State private var offset: CGSize = .zero
    @GestureState private var drag: CGSize = .zero

    var body: some View {
        GeometryReader { proxy in
            picture
                .frame(width: proxy.size.width, height: proxy.size.height)
                .scaleEffect(zoom * pinch)
                .offset(x: offset.width + drag.width, y: offset.height + drag.height)
                .contentShape(.rect)
                .gesture(MagnifyGesture()
                    .updating($pinch) { value, state, _ in state = value.magnification }
                    .onEnded { value in
                        withAnimation(.snappy) {
                            zoom = min(max(zoom * value.magnification, 1), 6)
                            if zoom == 1 { offset = .zero }
                        }
                    })
                .gesture(DragGesture()
                    .updating($drag) { value, state, _ in if zoom > 1 { state = value.translation } }
                    .onEnded { value in
                        guard zoom > 1 else { return }
                        offset.width += value.translation.width
                        offset.height += value.translation.height
                    }, including: zoom > 1 ? .all : .subviews)
                .onTapGesture(count: 2) {
                    withAnimation(.snappy) {
                        zoom = zoom > 1 ? 1 : 2.5
                        if zoom == 1 { offset = .zero }
                    }
                }
        }
        .task(id: photo.id) {
            preview = MacPreviewLoader.shared.cached(photo)
            if preview == nil { preview = await MacPreviewLoader.shared.image(for: photo, client: client) }
        }
        .accessibilityElement()
        .accessibilityLabel(photo.isScreenshot ? "Screenshot" : "Photo")
        .accessibilityAddTraits(.isImage)
    }

    @ViewBuilder
    private var picture: some View {
        if let preview {
            Image(decorative: preview, scale: 1).resizable().scaledToFit()
        } else if let thumbnail = PhotoThumbnailCache.anyCachedCGImage(forChecksum: photo.checksum),
                  let width = photo.width, let height = photo.height, width > 0, height > 0 {
            // The square thumbnail inside the photo's real shape, so nothing jumps when the preview lands.
            Color.clear
                .aspectRatio(CGFloat(width) / CGFloat(height), contentMode: .fit)
                .overlay { Image(decorative: thumbnail, scale: 1).resizable().scaledToFill() }
                .clipped()
                .blur(radius: 6)
                .overlay { ProgressView().tint(.white) }
        } else {
            ProgressView().tint(.white)
        }
    }
}

/// The host's previews, decoded off the main thread, the last few dozen kept in memory (and every
/// download in the shared URL cache, the URL carrying the photo's checksum).
@MainActor
final class MacPreviewLoader {
    static let shared = MacPreviewLoader()

    private let memory: NSCache<NSString, Box> = {
        let cache = NSCache<NSString, Box>()
        cache.totalCostLimit = 400 * 1024 * 1024
        return cache
    }()
    private var running: [String: Task<CGImage?, Never>] = [:]

    private final class Box {
        let image: CGImage
        init(_ image: CGImage) { self.image = image }
    }

    func cached(_ photo: Photo) -> CGImage? { memory.object(forKey: photo.checksum as NSString)?.image }

    func prefetch(_ photo: Photo, client: PhotosClient) {
        guard cached(photo) == nil, running[photo.checksum] == nil else { return }
        Task { _ = await image(for: photo, client: client) }
    }

    func image(for photo: Photo, client: PhotosClient) async -> CGImage? {
        if let image = cached(photo) { return image }
        let task = running[photo.checksum] ?? {
            let url = client.previewURL(for: photo)
            let task = Task.detached(priority: .userInitiated) { await Self.fetch(url) }
            running[photo.checksum] = task
            return task
        }()
        let image = await task.value
        running[photo.checksum] = nil
        if let image { memory.setObject(Box(image), forKey: photo.checksum as NSString, cost: image.bytesPerRow * image.height) }
        return image
    }

    nonisolated private static func fetch(_ url: URL) async -> CGImage? {
        guard let (data, response) = try? await TPConnection.shared.session.data(for: TPConnection.shared.request(url)),
              (response as? HTTPURLResponse).map({ (200..<300).contains($0.statusCode) }) == true,
              let source = CGImageSourceCreateWithData(data as CFData, nil) else { return nil }
        let options: [CFString: Any] = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceShouldCacheImmediately: true,
            kCGImageSourceThumbnailMaxPixelSize: 4096,
        ]
        return CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary)
    }
}

#endif

/// The ⓘ sheet: when, what, with which camera and where.
struct PhotoInfoView: View {
    let photo: Photo
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    if let takenAt = photo.takenAt.flatMap(PhotoDates.parse) {
                        LabeledContent("Taken", value: takenAt.formatted(date: .long, time: .shortened))
                    }
                    LabeledContent("File Name", value: photo.originalFilename)
                    if let width = photo.width, let height = photo.height {
                        LabeledContent("Dimensions", value: "\(width) × \(height)")
                    }
                    LabeledContent("Size", value: formatBytes(photo.fileSize))
                    if let mimeType = photo.mimeType {
                        LabeledContent("Format", value: mimeType)
                    }
                }
                if photo.cameraMake != nil || photo.cameraModel != nil {
                    Section("Camera") {
                        if let make = photo.cameraMake { LabeledContent("Make", value: make) }
                        if let model = photo.cameraModel { LabeledContent("Model", value: model) }
                    }
                }
                if let lat = photo.gpsLat, let lon = photo.gpsLon {
                    Section("Location") {
                        LabeledContent("Coordinates", value: String(format: "%.5f, %.5f", lat, lon))
                        if let url = URL(string: "https://maps.apple.com/?ll=\(lat),\(lon)") {
                            Link("Open in Maps", destination: url)
                        }
                    }
                }
                Section("Backup") {
                    LabeledContent("Status", value: statusText)
                    if let uploadedAt = PhotoDates.parse(photo.uploadedAt) {
                        LabeledContent("Uploaded", value: uploadedAt.formatted(date: .abbreviated, time: .shortened))
                    }
                }
            }
            .formStyle(.grouped)
            .navigationTitle("Info")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
        .presentationDetents([.medium, .large])
    }

    private var statusText: String {
        switch photo.status {
        case .verified: return "Verified"
        case .pending: return "Pending verification"
        case .checksumMismatch: return "Checksum mismatch"
        case .error: return "Error"
        }
    }
}
