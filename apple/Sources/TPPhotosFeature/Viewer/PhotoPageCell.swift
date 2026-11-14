#if os(iOS)
import TPCore
import UIKit

/// One page of the viewer: the on-device thumbnail at once, the preview a moment later (or at
/// once, when it was fetched ahead), and the original when zoomed in beyond the preview.
final class PhotoPageCell: UICollectionViewCell {
    static let reuseIdentifier = "PhotoPageCell"

    let zoomView = PhotoZoomView()
    private(set) var photo: Photo?
    private var client: PhotosClient?
    private var previewTask: Task<Void, Never>?
    private var originalTask: Task<Void, Never>?
    private var hasOriginal = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        zoomView.frame = contentView.bounds
        zoomView.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        contentView.addSubview(zoomView)
        zoomView.onZoom = { [weak self] in self?.loadOriginalIfNeeded() }
        isAccessibilityElement = true
        accessibilityTraits = .image
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override var accessibilityLabel: String? {
        get { photo?.isScreenshot == true ? "Screenshot" : "Photo" }
        set {}
    }

    func configure(with photo: Photo, client: PhotosClient) {
        guard photo.id != self.photo?.id else { return }
        cancelLoads()
        self.photo = photo
        self.client = client
        let loader = PhotoViewerImageLoader.shared

        if let original = loader.cachedImage(for: photo, kind: .original) {
            hasOriginal = true
            zoomView.setImage(original, aspect: original.size, crossfade: false)
            return
        }
        if let preview = loader.cachedImage(for: photo, kind: .preview) {
            zoomView.setImage(preview, aspect: preview.size, crossfade: false)
            return
        }
        zoomView.setImage(Self.thumbnail(for: photo), aspect: Self.aspect(of: photo), crossfade: false)
        previewTask = Task { [weak self] in
            guard let image = await loader.image(for: photo, kind: .preview, client: client),
                  let self, self.photo?.id == photo.id, !self.hasOriginal else { return }
            self.zoomView.setImage(image, aspect: image.size, crossfade: true)
            PhotoGridHitchMonitor.current?.sharpPhotoShown()
        }
    }

    /// The grid's thumbnail: decoded already if it was on screen, otherwise read from the phone.
    private static func thumbnail(for photo: Photo) -> UIImage? {
        if let image = PhotoThumbnailCache.anyCachedCGImage(forChecksum: photo.checksum) {
            return UIImage(cgImage: image)
        }
        return UIImage(contentsOfFile: PhotoThumbnailCache.fileURL(forChecksum: photo.checksum).path)
    }

    /// The photo's proportions before any of it has loaded (thumbnails are square crops).
    private static func aspect(of photo: Photo) -> CGSize {
        if let width = photo.width, let height = photo.height, width > 0, height > 0 {
            return CGSize(width: width, height: height)
        }
        return CGSize(width: 3, height: 4)
    }

    /// Zoomed in past the preview's own pixels: fetch the original and swap it in, keeping the
    /// zoom where it is.
    private func loadOriginalIfNeeded() {
        guard !hasOriginal, originalTask == nil, let photo, let client,
              let shown = zoomView.imageView.image, zoomView.isZoomedIn,
              zoomView.displayedPixelWidth > shown.size.width * shown.scale * 1.15 else { return }
        originalTask = Task { [weak self] in
            let image = await PhotoViewerImageLoader.shared.image(for: photo, kind: .original, client: client)
            guard let self, self.photo?.id == photo.id else { return }
            self.originalTask = nil
            guard let image else { return }
            self.hasOriginal = true
            self.zoomView.setImage(image, aspect: image.size, crossfade: false)
        }
    }

    private func cancelLoads() {
        previewTask?.cancel()
        originalTask?.cancel()
        previewTask = nil
        originalTask = nil
        hasOriginal = false
    }

    override func prepareForReuse() {
        super.prepareForReuse()
        cancelLoads()
        photo = nil
        zoomView.setImage(nil, aspect: .zero, crossfade: false)
        zoomView.reset()
    }
}
#endif
