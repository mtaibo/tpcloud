#if os(iOS)
import UIKit

/// One photo, zoomable the way Photos is: a plain `UIScrollView` zoom, so pinching scales about
/// the fingers, panning a zoomed photo carries on with momentum and stops softly at its edges,
/// and zooming past the limits rubber-bands back. (The SwiftUI viewer scaled a view with
/// `scaleEffect`: it zoomed about the middle whatever the fingers did, panned with no momentum or
/// edges, and snapped into place when let go.)
final class PhotoZoomView: UIScrollView, UIScrollViewDelegate {
    let imageView = UIImageView()
    /// Called as the zoom changes, to fetch the original once the preview runs out of detail.
    var onZoom: (() -> Void)?

    /// The proportions the photo is laid out with.
    private var aspect: CGSize = .zero
    private var laidOutSize: CGSize = .zero

    override init(frame: CGRect) {
        super.init(frame: frame)
        delegate = self
        showsHorizontalScrollIndicator = false
        showsVerticalScrollIndicator = false
        contentInsetAdjustmentBehavior = .never
        decelerationRate = .fast
        bouncesZoom = true
        // A square thumbnail stands in until the preview arrives, cropped to the photo's shape.
        imageView.contentMode = .scaleAspectFill
        imageView.clipsToBounds = true
        addSubview(imageView)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    var isZoomedIn: Bool {
        zoomScale > minimumZoomScale + 0.01
    }

    /// The photo's size on screen at the current zoom, in pixels.
    var displayedPixelWidth: CGFloat {
        imageView.frame.width * traitCollection.displayScale
    }

    /// Shows `image` laid out with the proportions of `aspect`. A new image of the same shape
    /// (the preview after the thumbnail, the original after the preview) keeps the zoom.
    func setImage(_ image: UIImage?, aspect newAspect: CGSize, crossfade: Bool) {
        if crossfade, imageView.image != nil {
            UIView.transition(with: imageView, duration: 0.2, options: [.transitionCrossDissolve, .allowUserInteraction]) {
                self.imageView.image = image
            }
        } else {
            imageView.image = image
        }
        guard newAspect.width > 0, newAspect.height > 0 else { return }
        let changed = aspect.width <= 0 || abs(newAspect.width / newAspect.height - aspect.width / aspect.height) > 0.01
        aspect = newAspect
        if changed { layoutPhoto() }
    }

    /// Back to the whole photo, for a page that's been swiped away or reused.
    func reset() {
        layoutPhoto()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        if bounds.size != laidOutSize { layoutPhoto() }
    }

    private func layoutPhoto() {
        laidOutSize = bounds.size
        guard bounds.width > 0, bounds.height > 0, aspect.width > 0, aspect.height > 0 else { return }
        minimumZoomScale = 1
        zoomScale = 1
        let fit = min(bounds.width / aspect.width, bounds.height / aspect.height)
        let size = CGSize(width: (aspect.width * fit).rounded(), height: (aspect.height * fit).rounded())
        imageView.frame = CGRect(origin: .zero, size: size)
        contentSize = size
        // Deep enough to fill the screen twice over, and at least 4×.
        let fill = max(bounds.width / size.width, bounds.height / size.height)
        maximumZoomScale = max(4, fill * 2)
        centerPhoto()
        contentOffset = CGPoint(x: -contentInset.left, y: -contentInset.top)
    }

    /// Keeps a photo smaller than the screen in the middle of it.
    private func centerPhoto() {
        let x = max((bounds.width - imageView.frame.width) / 2, 0)
        let y = max((bounds.height - imageView.frame.height) / 2, 0)
        contentInset = UIEdgeInsets(top: y, left: x, bottom: y, right: x)
    }

    /// Double tap: in on the tapped spot (far enough to fill the screen), or back out.
    func toggleZoom(at point: CGPoint) {
        if isZoomedIn {
            setZoomScale(minimumZoomScale, animated: true)
            return
        }
        let base = imageView.bounds.size
        guard base.width > 0, base.height > 0 else { return }
        let fill = max(bounds.width / base.width, bounds.height / base.height)
        let scale = min(max(fill, 2.5), maximumZoomScale)
        let size = CGSize(width: bounds.width / scale, height: bounds.height / scale)
        zoom(to: CGRect(x: point.x - size.width / 2, y: point.y - size.height / 2, width: size.width, height: size.height), animated: true)
    }

    /// Where the photo is on screen, for the zoom transition to land on.
    func photoFrame(in view: UIView) -> CGRect {
        imageView.convert(imageView.bounds, to: view)
    }

    // MARK: UIScrollViewDelegate

    func viewForZooming(in scrollView: UIScrollView) -> UIView? {
        imageView
    }

    func scrollViewDidZoom(_ scrollView: UIScrollView) {
        centerPhoto()
        onZoom?()
    }
}
#endif
