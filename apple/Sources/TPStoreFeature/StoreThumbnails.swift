import AVFoundation
import CryptoKit
import ImageIO
import Synchronization
import SwiftUI
import TPCore
import TPUI
import UniformTypeIdentifiers

/// What a file shows in place of its icon: a photo's server-side thumbnail, or a frame of a video.
struct FileThumbnailSource: Hashable {
    enum Kind {
        case image, video
    }

    let url: URL
    let kind: Kind
    /// The file's modification time, so a replaced video gets a fresh frame instead of the cached one.
    let version: Double

    /// Nil for anything without a picture to show, including uploads still on their way.
    init?(entry: FileEntry, in folder: StoreFolder, storeClient: StoreClient) {
        guard entry.type == .file else { return nil }
        let path = folder.entryPath(entry.name)
        switch FileKind(name: entry.name) {
        case .image:
            url = storeClient.thumbnailURL(path: path, location: folder.location, size: 240)
            kind = .image
        case .video:
            url = storeClient.viewURL(path: path, location: folder.location)
            kind = .video
        default:
            return nil
        }
        version = entry.modified
    }
}

/// A square, cropped preview. Videos get a play glyph in the corner, and keep the film icon while
/// their frame loads or if it can't be read (an .mkv, say, which AVFoundation doesn't open).
struct FileThumbnail: View {
    let source: FileThumbnailSource
    var badgeFont: Font = .caption

    var body: some View {
        switch source.kind {
        case .image:
            RemoteImage(url: source.url)
        case .video:
            VideoThumbnail(source: source, badgeFont: badgeFont)
        }
    }
}

private struct VideoThumbnail: View {
    let source: FileThumbnailSource
    let badgeFont: Font

    @State private var image: Image?

    init(source: FileThumbnailSource, badgeFont: Font) {
        self.source = source
        self.badgeFont = badgeFont
        _image = State(initialValue: VideoThumbnailer.shared.cachedImage(for: source))
    }

    var body: some View {
        ZStack(alignment: .bottomLeading) {
            if let image {
                image
                    .resizable()
                    .aspectRatio(contentMode: .fill)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .clipped()
                Image(systemName: "play.fill")
                    .font(badgeFont)
                    .foregroundStyle(.white)
                    .shadow(color: .black.opacity(0.5), radius: 2)
                    .padding(6)
            } else {
                Rectangle()
                    .fill(.fill.quaternary)
                Image(systemName: FileKind.video.systemImage)
                    .font(.title2)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .task(id: source) {
            if let loaded = await VideoThumbnailer.shared.image(for: source) {
                image = loaded
            }
        }
    }
}

/// Grabs one frame near the start of a video on the server. AVFoundation asks only for the byte
/// ranges it needs (the header and the frame), not the whole file, and every frame is kept on
/// disk afterwards, so each video is read at most once.
final class VideoThumbnailer: Sendable {
    static let shared = VideoThumbnailer()

    // NSCache is internally thread-safe.
    nonisolated(unsafe) private let memory = NSCache<NSString, CGImageBox>()
    /// Videos whose frame couldn't be read this session, so scrolling past them doesn't retry.
    private let failed = Mutex<Set<String>>([])
    private let directory: URL

    private init() {
        memory.countLimit = 300
        directory = URL.cachesDirectory.appending(path: "StoreVideoThumbnails", directoryHint: .isDirectory)
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    }

    func cachedImage(for source: FileThumbnailSource) -> Image? {
        memory.object(forKey: Self.key(for: source) as NSString).map { Image(decorative: $0.image, scale: 1) }
    }

    @concurrent
    func image(for source: FileThumbnailSource) async -> Image? {
        let key = Self.key(for: source)
        if let cached = memory.object(forKey: key as NSString) {
            return Image(decorative: cached.image, scale: 1)
        }
        guard !failed.withLock({ $0.contains(key) }) else { return nil }

        let file = directory.appending(path: "\(key).jpg")
        let frame: CGImage
        if let data = try? Data(contentsOf: file), let decoded = Self.decode(data) {
            frame = decoded
        } else {
            do {
                frame = try await Self.generate(from: source.url)
            } catch {
                // A scrolled-away tile cancels its load; that's not a broken video.
                if !Task.isCancelled { _ = failed.withLock { $0.insert(key) } }
                return nil
            }
            if let data = Self.encode(frame) { try? data.write(to: file, options: .atomic) }
        }
        memory.setObject(CGImageBox(frame), forKey: key as NSString)
        return Image(decorative: frame, scale: 1)
    }

    private static func generate(from url: URL) async throws -> CGImage {
        // AVFoundation loads remote media out of process, without the app's cookie store, so the
        // session cookie the gateway checks has to be handed over explicitly.
        let cookies = HTTPCookieStorage.shared.cookies(for: url) ?? []
        let asset = AVURLAsset(url: url, options: [AVURLAssetHTTPCookiesKey: cookies])
        let generator = AVAssetImageGenerator(asset: asset)
        generator.appliesPreferredTrackTransform = true
        generator.maximumSize = CGSize(width: 480, height: 480)
        // Any frame close to the requested time will do, which spares decoding up to an exact one.
        let tolerance = CMTime(seconds: 1, preferredTimescale: 600)
        generator.requestedTimeToleranceBefore = tolerance
        generator.requestedTimeToleranceAfter = tolerance

        // A second in skips the black first frame many clips start with; short clips use their middle.
        let duration = try await asset.load(.duration).seconds
        let time = CMTime(seconds: duration.isFinite ? min(1, duration / 2) : 0, preferredTimescale: 600)
        nonisolated(unsafe) let cancellable = generator
        return try await withTaskCancellationHandler {
            try await generator.image(at: time).image
        } onCancel: {
            cancellable.cancelAllCGImageGeneration()
        }
    }

    private static func key(for source: FileThumbnailSource) -> String {
        let digest = SHA256.hash(data: Data("\(source.url.absoluteString)|\(source.version)".utf8))
        return digest.prefix(12).map { String(format: "%02x", $0) }.joined()
    }

    private static func encode(_ image: CGImage) -> Data? {
        let data = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(data, UTType.jpeg.identifier as CFString, 1, nil) else { return nil }
        CGImageDestinationAddImage(destination, image, [kCGImageDestinationLossyCompressionQuality: 0.75] as CFDictionary)
        guard CGImageDestinationFinalize(destination) else { return nil }
        return data as Data
    }

    private static func decode(_ data: Data) -> CGImage? {
        guard let source = CGImageSourceCreateWithData(data as CFData, nil) else { return nil }
        return CGImageSourceCreateImageAtIndex(source, 0, nil)
    }
}

private final class CGImageBox: @unchecked Sendable {
    let image: CGImage
    init(_ image: CGImage) { self.image = image }
}
