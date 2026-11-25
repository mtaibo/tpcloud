import Foundation
@preconcurrency import Photos
import TPCore

/// Shared "what still needs backing up" logic, used by both the foreground `PhotoSyncStore`
/// (user taps "Backup now") and `PhotoBackgroundUploader` (the unattended background-session
/// engine) so the two paths can never drift into different ideas of what "pending" means.
public enum PhotoBackupPlanner {
    public struct AssetScan: Sendable {
        /// Photos and videos, newest first.
        public let assets: [PHAsset]
    }

    /// Every photo and video in the library, newest first, so a long backup makes the newest show
    /// up almost at once instead of only after the whole backlog. (Stills only until Nov 2026.)
    /// Enumerates the whole library, which takes a noticeable while on a large one — `@concurrent`
    /// so it never runs on the main thread, whoever calls it.
    @concurrent
    public static func scanAssets() async -> AssetScan {
        let options = PHFetchOptions()
        options.sortDescriptors = [NSSortDescriptor(key: "creationDate", ascending: false)]
        let result = PHAsset.fetchAssets(with: options)

        var assets: [PHAsset] = []
        result.enumerateObjects { asset, _, _ in
            if asset.mediaType == .image || asset.mediaType == .video { assets.append(asset) }
        }
        return AssetScan(assets: assets)
    }

    public static func checkKnownAssetIds(_ ids: [String], client: PhotosClient) async throws -> Set<String> {
        var known = Set<String>()
        for batch in ids.chunked(into: 200) {
            let result = try await client.checkExisting(assetIds: batch)
            known.formUnion(result)
        }
        return known
    }

    /// Matches the server's `photos/<email>/<year>/<month>` canonical layout.
    public static func destinationFolder(for takenAt: Date?, ownerEmail: String) -> String {
        let date = takenAt ?? Date()
        let calendar = Calendar(identifier: .gregorian)
        let year = calendar.component(.year, from: date)
        let month = calendar.component(.month, from: date)
        return "photos/\(ownerEmail)/\(year)/\(String(format: "%02d", month))"
    }

    public static func fileExtension(for originalFilename: String) -> String {
        let ext = (originalFilename as NSString).pathExtension
        return ext.isEmpty ? "jpg" : ext.lowercased()
    }
}

extension Array {
    func chunked(into size: Int) -> [[Element]] {
        stride(from: 0, to: count, by: size).map { Array(self[$0..<Swift.min($0 + size, count)]) }
    }
}
