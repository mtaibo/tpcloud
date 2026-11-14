#if os(iOS)
import ImageIO
import TPCore
import UIKit

/// What the viewer shows, loaded the way the web viewer does it — which is why photos opened in
/// the browser came up at once while the app's dragged:
///
/// - the server's 2048 px preview (a few hundred KB) instead of the original (3–10 MB of HEIC
///   through the tunnel from home), and the original only once a photo is zoomed in past what
///   the preview can show;
/// - kept on disk: the preview URL carries the photo's checksum, so a photo seen once never
///   touches the network again;
/// - decoded off the main thread, straight to the size that's drawn, and kept decoded for the
///   photos around the one on screen, which are fetched ahead of a swipe.
@MainActor
final class PhotoViewerImageLoader {
    enum Kind {
        case preview
        case original
    }

    static let shared = PhotoViewerImageLoader()

    /// The preview is 2048 px on its long side; an original is decoded to at most this, enough
    /// for the deepest zoom on any iPhone without holding a 48 MP bitmap.
    static let originalMaxPixels: CGFloat = 4096

    private let session: URLSession
    private let decoded = NSCache<NSString, UIImage>()
    private var inFlight: [NSString: Task<UIImage?, Never>] = [:]
    /// Previews already fetched to disk ahead of a tap (or being fetched).
    private var warmed: Set<String> = []

    private init() {
        let configuration = URLSessionConfiguration.default
        let directory = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("PhotoViewer", isDirectory: true)
        configuration.urlCache = URLCache(memoryCapacity: 4 << 20, diskCapacity: 1 << 30, directory: directory)
        // Both URLs name one version of a photo that never changes.
        configuration.requestCachePolicy = .returnCacheDataElseLoad
        // A stalled request gives up instead of leaving a page on its thumbnail for a minute.
        configuration.timeoutIntervalForRequest = 15
        session = URLSession(configuration: configuration)
        // Decoded bitmaps: a 2048 px photo is ~12 MB, so this holds the current one and the
        // neighbours either side, plus an original being zoomed into.
        decoded.totalCostLimit = 160 << 20
    }

    private static func key(_ photo: Photo, _ kind: Kind) -> NSString {
        "\(photo.checksum)-\(kind == .preview ? "p" : "o")" as NSString
    }

    func cachedImage(for photo: Photo, kind: Kind) -> UIImage? {
        decoded.object(forKey: Self.key(photo, kind))
    }

    /// Shares one load per photo and kind between everyone asking. The load itself isn't tied to
    /// the caller, so a page swiped past keeps loading for when it comes back.
    func image(for photo: Photo, kind: Kind, client: PhotosClient) async -> UIImage? {
        let key = Self.key(photo, kind)
        if let image = decoded.object(forKey: key) { return image }
        if let task = inFlight[key] { return await task.value }
        let url = kind == .preview ? client.previewURL(for: photo) : client.originalURL(for: photo)
        let maxPixels = kind == .preview ? 2048 : Self.originalMaxPixels
        let session = self.session
        let task = Task { await Self.fetch(url, session: session, maxPixels: maxPixels) }
        inFlight[key] = task
        let image = await task.value
        inFlight[key] = nil
        if let image, let cgImage = image.cgImage {
            decoded.setObject(image, forKey: key, cost: cgImage.bytesPerRow * cgImage.height)
        }
        return image
    }

    /// Starts the previews of the photos about to be swiped to.
    func prefetch(_ photos: [Photo], client: PhotosClient) {
        for photo in photos {
            let key = Self.key(photo, .preview)
            guard decoded.object(forKey: key) == nil, inFlight[key] == nil else { continue }
            Task { _ = await image(for: photo, kind: .preview, client: client) }
        }
    }

    /// Previews waiting to be fetched ahead, nearest the middle of the screen first.
    private var warmQueue: [URL] = []
    private var warming = 0
    /// Few at a time, at low priority: thirty at once (16 MB through the home uplink) used to
    /// leave the photo actually tapped waiting behind them.
    private let maxWarming = 3

    /// Fetches the previews of the photos on screen to disk, without decoding them, so the one
    /// that gets tapped opens from the phone instead of the network (see `waitForPreview`).
    /// Replaces whatever was still queued from the last time the grid stopped.
    func warm(_ photos: [Photo], client: PhotosClient) {
        warmQueue = photos
            .filter { !warmed.contains($0.checksum) && decoded.object(forKey: Self.key($0, .preview)) == nil }
            .map { client.previewURL(for: $0) }
        pumpWarming()
    }

    /// The grid started moving again: what's queued is for photos no longer on screen.
    func cancelWarming() {
        warmQueue.removeAll()
    }

    private func pumpWarming() {
        while warming < maxWarming, !warmQueue.isEmpty {
            let url = warmQueue.removeFirst()
            guard let checksum = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems?.first(where: { $0.name == "v" })?.value,
                  !warmed.contains(checksum) else { continue }
            warmed.insert(checksum)
            warming += 1
            let session = self.session
            Task { [weak self] in
                _ = await Self.load(url, session: session, priority: URLSessionTask.lowPriority)
                guard let self else { return }
                self.warming -= 1
                self.pumpWarming()
            }
        }
    }

    /// Starts decoding a photo's preview (from disk if it was warmed) as it's being opened.
    func prepare(_ photo: Photo, client: PhotosClient) {
        guard decoded.object(forKey: Self.key(photo, .preview)) == nil else { return }
        Task { _ = await image(for: photo, kind: .preview, client: client) }
    }

    /// Waits until the preview is decoded, or `timeout` has passed, whichever is first: opening a
    /// photo a beat later but sharp looks better than opening it blurry and swapping.
    func waitForPreview(_ photo: Photo, timeout: Duration) async {
        guard decoded.object(forKey: Self.key(photo, .preview)) == nil,
              let task = inFlight[Self.key(photo, .preview)] else { return }
        // A plain race. (A task group waits for all of its children, and the load doesn't stop
        // for cancellation, so it used to wait out the whole download — forever, if it hung.)
        let once = Once()
        await withCheckedContinuation { (continuation: CheckedContinuation<Void, Never>) in
            Task {
                _ = await task.value
                if once.claim() { continuation.resume() }
            }
            Task {
                try? await Task.sleep(for: timeout)
                if once.claim() { continuation.resume() }
            }
        }
    }

    @concurrent
    nonisolated private static func fetch(_ url: URL, session: URLSession, maxPixels: CGFloat) async -> UIImage? {
        guard let data = await load(url, session: session, priority: URLSessionTask.highPriority) else { return nil }
        return decode(data, maxPixels: maxPixels)
    }

    /// A GET with a priority (the async `data(from:)` has none): what's being opened goes ahead
    /// of what's only being fetched in case.
    nonisolated private static func load(_ url: URL, session: URLSession, priority: Float) async -> Data? {
        await withCheckedContinuation { (continuation: CheckedContinuation<Data?, Never>) in
            let task = session.dataTask(with: url) { data, response, _ in
                let ok = (response as? HTTPURLResponse).map { (200..<300).contains($0.statusCode) } ?? false
                continuation.resume(returning: ok ? data : nil)
            }
            task.priority = priority
            task.resume()
        }
    }

    /// Decodes straight to at most `maxPixels` on the long side, upright, and fully (not lazily
    /// on first draw, which is what used to stall the first frame of a swipe).
    nonisolated static func decode(_ data: Data, maxPixels: CGFloat) -> UIImage? {
        guard let source = CGImageSourceCreateWithData(data as CFData, [kCGImageSourceShouldCache: false] as CFDictionary) else {
            return nil
        }
        let options: [CFString: Any] = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceShouldCacheImmediately: true,
            kCGImageSourceThumbnailMaxPixelSize: maxPixels,
        ]
        guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else { return nil }
        return UIImage(cgImage: image)
    }
}
/// Lets exactly one of several racers resume a continuation.
private final class Once: @unchecked Sendable {
    private let lock = NSLock()
    private var done = false

    func claim() -> Bool {
        lock.lock()
        defer { lock.unlock() }
        guard !done else { return false }
        done = true
        return true
    }
}
#endif
