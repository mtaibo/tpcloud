#if os(iOS)
import CoreGraphics
import TPCore
import UIKit

/// Draws the dense grids' rows (see `PhotoGridView.drawsRows`): one picture per row, made off the
/// main thread from the thumbnail table's ready-made bitmaps, and cached. Photos not in the table
/// yet are drawn as a grey square and reported back, so the row can be drawn again once they land.
@MainActor
final class PhotoGridRowRenderer {
    struct Request: Sendable {
        let key: String
        /// Each photo's checksum and frame, relative to the row's top-left corner, in points.
        let cells: [(checksum: String, frame: CGRect)]
        let size: CGSize
        let scale: CGFloat
        let placeholder: CGColor
    }

    struct Result: @unchecked Sendable {
        let image: CGImage?
        let missing: [String]
    }

    private let cache: NSCache<NSString, CGImage> = {
        let cache = NSCache<NSString, CGImage>()
        // A 31-column row is ~0.5 MB and a 15-column one ~1 MB on an iPhone: a few screens of
        // each, around where the grid is.
        cache.totalCostLimit = 120 << 20
        return cache
    }()
    private var inFlight: [String: [(Result) -> Void]] = [:]
    private let queue = DispatchQueue(label: "PhotoGridRowRenderer", qos: .userInitiated, attributes: .concurrent)
    private let limit = DispatchSemaphore(value: 2)

    func cachedImage(forKey key: String) -> CGImage? {
        cache.object(forKey: key as NSString)
    }

    func forget(_ key: String) {
        cache.removeObject(forKey: key as NSString)
    }

    /// Draws `request` unless it's cached or already being drawn; `completion` runs on the main
    /// thread (also when it was drawn for someone else).
    func render(_ request: Request, completion: @escaping (Result) -> Void = { _ in }) {
        if let image = cache.object(forKey: request.key as NSString) {
            completion(Result(image: image, missing: []))
            return
        }
        if inFlight[request.key] != nil {
            inFlight[request.key]?.append(completion)
            return
        }
        inFlight[request.key] = [completion]
        let limit = self.limit
        queue.async { [weak self] in
            limit.wait()
            let result = Self.draw(request)
            limit.signal()
            DispatchQueue.main.async {
                MainActor.assumeIsolated {
                    guard let self else { return }
                    if let image = result.image {
                        self.cache.setObject(image, forKey: request.key as NSString, cost: image.bytesPerRow * image.height)
                    }
                    let waiters = self.inFlight.removeValue(forKey: request.key) ?? []
                    for waiter in waiters { waiter(result) }
                }
            }
        }
    }

    nonisolated private static func draw(_ request: Request) -> Result {
        let width = Int((request.size.width * request.scale).rounded(.up))
        let height = Int((request.size.height * request.scale).rounded(.up))
        guard width > 0, height > 0,
              let context = CGContext(
                  data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
                  space: CGColorSpace(name: CGColorSpace.sRGB)!,
                  bitmapInfo: CGBitmapInfo.byteOrder32Little.rawValue | CGImageAlphaInfo.premultipliedFirst.rawValue
              ) else { return Result(image: nil, missing: []) }
        // Core Graphics' own orientation (origin at the bottom left), in which images draw upright;
        // each cell's rect is flipped into it below. (Flipping the context as well drew every
        // photo upside down.)
        context.scaleBy(x: request.scale, y: request.scale)
        context.interpolationQuality = .medium
        var missing: [String] = []
        for cell in request.cells {
            let rect = CGRect(x: cell.frame.minX, y: request.size.height - cell.frame.maxY, width: cell.frame.width, height: cell.frame.height)
            if let image = PhotoThumbnailTable.shared.image(forChecksum: cell.checksum) {
                context.draw(image, in: rect)
            } else {
                context.setFillColor(request.placeholder)
                context.fill(rect)
                missing.append(cell.checksum)
            }
        }
        return Result(image: context.makeImage(), missing: missing)
    }
}
#endif
