import CryptoKit
import ImageIO
@preconcurrency import Photos
import UniformTypeIdentifiers

public struct ExtractedPhotoMetadata: Sendable {
    public let sourceAssetId: String
    public let originalFilename: String
    public let fileSize: Int
    public let checksum: String
    public let mimeType: String?
    public let width: Int
    public let height: Int
    public let takenAt: Date?
    public let isScreenshot: Bool
    public let isCameraOriginal: Bool
    public let sourceType: String
    public let cameraMake: String?
    public let cameraModel: String?
    public let gpsLat: Double?
    public let gpsLon: Double?
    /// The original, written to a temporary file the caller owns (moves or deletes): videos run
    /// to gigabytes, far too much to hold in memory the way photos used to be.
    public let fileURL: URL
    public var isVideo: Bool { mimeType?.hasPrefix("video/") ?? false }
}

public struct LightweightPhotoMetadata: Sendable {
    public let sourceAssetId: String
    public let originalFilename: String?
    public let width: Int
    public let height: Int
    public let takenAt: Date?
}

public enum PhotoMetadataExtractor {
    public enum ExtractionError: Error {
        case noPhotoResource
        case timedOut
    }

    /// iCloud-sourced originals can stall indefinitely inside PhotoKit's own download queue;
    /// without a timeout a single stuck asset would hang `backupNow()` forever (stuck
    /// "uploading" button, no retry possible). Each network-capable request below races
    /// against this and cancels itself on expiry so the backup loop can move on. A video can be
    /// gigabytes coming down from iCloud, so it gets far longer.
    private static let photoTimeout: Duration = .seconds(90)
    private static let videoTimeout: Duration = .seconds(45 * 60)

    /// Runs `operation` against a timeout, invoking `onTimeout` (e.g. to cancel the underlying
    /// PhotoKit request) and throwing if the timeout wins the race.
    private static func withTimeout<T: Sendable>(
        _ timeout: Duration = photoTimeout,
        onTimeout: @escaping @Sendable () -> Void,
        operation: @escaping @Sendable () async throws -> T
    ) async throws -> T {
        try await withThrowingTaskGroup(of: T.self) { group in
            group.addTask { try await operation() }
            group.addTask {
                try await Task.sleep(for: timeout)
                onTimeout()
                throw ExtractionError.timedOut
            }
            defer { group.cancelAll() }
            return try await group.next()!
        }
    }

    /// The asset's original resource: the photo (a Live Photo's still; its paired clip stays on
    /// the phone, like Photos' own export) or the video.
    static func originalResource(of asset: PHAsset) -> PHAssetResource? {
        let resources = PHAssetResource.assetResources(for: asset)
        switch asset.mediaType {
        case .image: return resources.first { $0.type == .photo } ?? resources.first { $0.type == .fullSizePhoto }
        case .video: return resources.first { $0.type == .video } ?? resources.first { $0.type == .fullSizeVideo }
        default: return nil
        }
    }

    public static func extract(from asset: PHAsset) async throws -> ExtractedPhotoMetadata {
        guard let resource = originalResource(of: asset) else { throw ExtractionError.noPhotoResource }
        let isVideo = asset.mediaType == .video

        let (fileURL, size, checksum) = try await writeFileAndChecksum(for: resource, timeout: isVideo ? videoTimeout : photoTimeout)

        let isScreenshot = asset.mediaSubtypes.contains(.photoScreenshot)
        let sourceType = classifySourceType(asset.sourceType)
        var exif = isVideo ? EXIFInfo() : await loadEXIF(for: asset)
        // Videos carry no EXIF; the library knows where they were made all the same.
        if exif.gpsLat == nil, let location = asset.location {
            exif.gpsLat = location.coordinate.latitude
            exif.gpsLon = location.coordinate.longitude
        }
        let filename = resource.filename ?? "untitled"
        let isCameraOriginal = isVideo
            ? sourceType == "user_library" && !filename.hasPrefix("RPReplay")
            : !isScreenshot && sourceType != "cloud_shared" && exif.cameraMake != nil

        return ExtractedPhotoMetadata(
            sourceAssetId: asset.localIdentifier,
            // `filename` (iOS 27) is optional where the deprecated `originalFilename` wasn't.
            originalFilename: filename,
            fileSize: size,
            checksum: checksum,
            mimeType: mimeType(for: resource),
            width: asset.pixelWidth,
            height: asset.pixelHeight,
            takenAt: asset.creationDate,
            isScreenshot: isScreenshot,
            isCameraOriginal: isCameraOriginal,
            sourceType: sourceType,
            cameraMake: exif.cameraMake,
            cameraModel: exif.cameraModel,
            gpsLat: exif.gpsLat,
            gpsLon: exif.gpsLon,
            fileURL: fileURL
        )
    }

    /// Cheap, network-free metadata for comparison only. `PHAssetResource.assetResources(for:)` does
    /// NOT trigger a download (it only reads already-known resource descriptors), unlike `extract(from:)`'s
    /// `requestData`/`requestContentEditingInput` calls — safe to call for every asset, even iCloud-only ones.
    public static func extractLightweight(from asset: PHAsset) -> LightweightPhotoMetadata {
        let originalFilename = originalResource(of: asset)?.filename
        return LightweightPhotoMetadata(
            sourceAssetId: asset.localIdentifier,
            originalFilename: originalFilename,
            width: asset.pixelWidth,
            height: asset.pixelHeight,
            takenAt: asset.creationDate
        )
    }

    private static func classifySourceType(_ sourceType: PHAssetSourceType) -> String {
        if sourceType.contains(.typeCloudShared) { return "cloud_shared" }
        if sourceType.contains(.typeiTunesSynced) { return "synced" }
        if sourceType.contains(.typeUserLibrary) { return "user_library" }
        return "unknown"
    }

    private static func mimeType(for resource: PHAssetResource) -> String? {
        UTType(resource.uniformTypeIdentifier)?.preferredMIMEType
    }

    /// Streams the asset's bytes once into a temporary file, hashing them as they go by: the same
    /// read gives the upload's file, its size, and the checksum the server checks again on disk.
    /// Nothing is held in memory but the chunk going past.
    private static func writeFileAndChecksum(for resource: PHAssetResource, timeout: Duration) async throws -> (URL, Int, String) {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("tpextract-\(UUID().uuidString)", isDirectory: false)
        FileManager.default.createFile(atPath: url.path, contents: nil)
        let writer = try FileHasher(url: url)
        let requestBox = Box<PHAssetResourceDataRequestID>()
        let manager = PHAssetResourceManager.default()
        let options = PHAssetResourceRequestOptions()
        options.isNetworkAccessAllowed = true

        do {
            try await withTimeout(timeout, onTimeout: {
                if let id = requestBox.value { manager.cancelDataRequest(id) }
            }) {
                try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
                    requestBox.value = manager.requestData(
                        for: resource,
                        options: options,
                        dataReceivedHandler: { chunk in writer.append(chunk) },
                        completionHandler: { error in
                            if let error {
                                continuation.resume(throwing: error)
                            } else {
                                continuation.resume()
                            }
                        }
                    )
                }
            }
            let (size, checksum) = try writer.finish()
            return (url, size, checksum)
        } catch {
            try? writer.finish()
            try? FileManager.default.removeItem(at: url)
            throw error
        }
    }

    private struct EXIFInfo: Sendable {
        var cameraMake: String?
        var cameraModel: String?
        var gpsLat: Double?
        var gpsLon: Double?
    }

    /// EXIF is best-effort metadata: on timeout or any failure we fall back to an empty
    /// `EXIFInfo()` rather than failing the whole asset, same as the original behavior.
    private static func loadEXIF(for asset: PHAsset) async -> EXIFInfo {
        let requestBox = Box<PHContentEditingInputRequestID>()

        let result: EXIFInfo? = try? await withTimeout(onTimeout: {
            if let id = requestBox.value { asset.cancelContentEditingInputRequest(id) }
        }) {
            await withCheckedContinuation { continuation in
                let options = PHContentEditingInputRequestOptions()
                options.isNetworkAccessAllowed = true
                requestBox.value = asset.requestContentEditingInput(with: options) { input, _ in
                    guard let url = input?.fullSizeImageURL,
                          let source = CGImageSourceCreateWithURL(url as CFURL, nil),
                          let props = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any] else {
                        continuation.resume(returning: EXIFInfo())
                        return
                    }

                    let tiff = props[kCGImagePropertyTIFFDictionary] as? [CFString: Any]
                    let gps = props[kCGImagePropertyGPSDictionary] as? [CFString: Any]

                    var info = EXIFInfo()
                    info.cameraMake = tiff?[kCGImagePropertyTIFFMake] as? String
                    info.cameraModel = tiff?[kCGImagePropertyTIFFModel] as? String

                    if let gps,
                       let latitude = gps[kCGImagePropertyGPSLatitude] as? Double,
                       let latRef = gps[kCGImagePropertyGPSLatitudeRef] as? String,
                       let longitude = gps[kCGImagePropertyGPSLongitude] as? Double,
                       let lonRef = gps[kCGImagePropertyGPSLongitudeRef] as? String {
                        info.gpsLat = latRef == "S" ? -latitude : latitude
                        info.gpsLon = lonRef == "W" ? -longitude : longitude
                    }

                    continuation.resume(returning: info)
                }
            }
        }

        return result ?? EXIFInfo()
    }
}

/// Serially mutated by PhotoKit's own callback queue; the timeout task only reads it to issue
/// a cancellation, so a plain `@unchecked Sendable` box is safe here without extra locking.
private final class Box<T>: @unchecked Sendable {
    var value: T?
}

/// Writes PhotoKit's chunks to a file as they arrive, hashing them on the way. Called serially
/// from PhotoKit's callback queue.
private final class FileHasher: @unchecked Sendable {
    private var hasher = SHA256()
    private let handle: FileHandle
    private var size = 0
    private var writeError: Error?
    private var closed = false

    init(url: URL) throws {
        handle = try FileHandle(forWritingTo: url)
    }

    func append(_ chunk: Data) {
        guard writeError == nil else { return }
        hasher.update(data: chunk)
        size += chunk.count
        do { try handle.write(contentsOf: chunk) } catch { writeError = error }
    }

    @discardableResult
    func finish() throws -> (Int, String) {
        if !closed {
            closed = true
            try? handle.close()
        }
        if let writeError { throw writeError }
        return (size, hasher.finalize().map { String(format: "%02x", $0) }.joined())
    }
}
