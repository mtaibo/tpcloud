import AVFoundation
import TPCore
import UniformTypeIdentifiers

/// Streams a video from the host into AVPlayer through the app's own connection. AVFoundation
/// fetches media out of process, with neither the session's bearer token nor the host's own
/// certificate authority (which `tpcloud.local` uses at home), so the player is handed a URL in a
/// scheme of our own and every range it asks for is fetched here, through `TPConnection`, and
/// passed back. The host serves byte ranges, so playback starts at once and seeking works.
final class AuthorizedVideoLoader: NSObject, AVAssetResourceLoaderDelegate, @unchecked Sendable {
    static let shared = AuthorizedVideoLoader()
    private static let scheme = "tpcloud-video"
    /// The most fetched per request: the player asks for «to the end» and gets it piece by piece.
    private static let maxPiece: Int64 = 4 * 1024 * 1024

    private let queue = DispatchQueue(label: "AuthorizedVideoLoader")
    private let lock = NSLock()
    private var tasks: [ObjectIdentifier: Task<Void, Never>] = [:]

    /// A player item for a host URL (`/api/files/preview` or `/view`).
    func playerItem(for url: URL) -> AVPlayerItem {
        var components = URLComponents(url: url, resolvingAgainstBaseURL: false)!
        components.scheme = Self.scheme + "-" + (components.scheme ?? "https")
        let asset = AVURLAsset(url: components.url!)
        asset.resourceLoader.setDelegate(self, queue: queue)
        return AVPlayerItem(asset: asset)
    }

    private static func realURL(_ url: URL) -> URL? {
        guard var components = URLComponents(url: url, resolvingAgainstBaseURL: false),
              let scheme = components.scheme, scheme.hasPrefix(Self.scheme + "-") else { return nil }
        components.scheme = String(scheme.dropFirst(Self.scheme.count + 1))
        return components.url
    }

    func resourceLoader(_ resourceLoader: AVAssetResourceLoader, shouldWaitForLoadingOfRequestedResource loadingRequest: AVAssetResourceLoadingRequest) -> Bool {
        guard let url = loadingRequest.request.url.flatMap(Self.realURL) else { return false }
        let key = ObjectIdentifier(loadingRequest)
        let task = Task { [weak self] in
            await self?.load(loadingRequest, from: url)
            self?.forget(key)
        }
        lock.withLock { tasks[key] = task }
        return true
    }

    func resourceLoader(_ resourceLoader: AVAssetResourceLoader, didCancel loadingRequest: AVAssetResourceLoadingRequest) {
        let key = ObjectIdentifier(loadingRequest)
        lock.withLock { tasks.removeValue(forKey: key) }?.cancel()
    }

    private func forget(_ key: ObjectIdentifier) {
        lock.withLock { _ = tasks.removeValue(forKey: key) }
    }

    private func load(_ loadingRequest: AVAssetResourceLoadingRequest, from url: URL) async {
        let dataRequest = loadingRequest.dataRequest
        let start = dataRequest.map { $0.currentOffset != 0 ? $0.currentOffset : $0.requestedOffset } ?? 0
        var length: Int64 = 2
        if let dataRequest {
            length = dataRequest.requestsAllDataToEndOfResource ? Self.maxPiece : Int64(dataRequest.requestedLength) - (start - dataRequest.requestedOffset)
            length = min(max(length, 1), Self.maxPiece)
        }
        var request = TPConnection.shared.request(url)
        request.setValue("bytes=\(start)-\(start + length - 1)", forHTTPHeaderField: "Range")
        do {
            let (data, response) = try await TPConnection.shared.session.data(for: request)
            guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
                loadingRequest.finishLoading(with: URLError(.badServerResponse))
                return
            }
            if let info = loadingRequest.contentInformationRequest {
                info.contentType = Self.contentType(http)
                info.contentLength = Self.totalLength(http) ?? Int64(data.count)
                info.isByteRangeAccessSupported = http.statusCode == 206
            }
            if let dataRequest {
                // A host that ignored the range sent the whole file: hand over the part asked for.
                if http.statusCode == 200, start > 0 {
                    let from = Int(min(start, Int64(data.count)))
                    dataRequest.respond(with: data.subdata(in: from..<min(from + Int(length), data.count)))
                } else {
                    dataRequest.respond(with: data)
                }
            }
            loadingRequest.finishLoading()
        } catch {
            if !Task.isCancelled { loadingRequest.finishLoading(with: error) }
        }
    }

    private static func totalLength(_ response: HTTPURLResponse) -> Int64? {
        // «bytes 0-1/123456»
        if let range = response.value(forHTTPHeaderField: "Content-Range"), let total = range.split(separator: "/").last {
            return Int64(total)
        }
        return response.expectedContentLength > 0 ? response.expectedContentLength : nil
    }

    private static func contentType(_ response: HTTPURLResponse) -> String {
        let mime = response.mimeType ?? "video/mp4"
        return UTType(mimeType: mime)?.identifier ?? UTType.mpeg4Movie.identifier
    }
}
