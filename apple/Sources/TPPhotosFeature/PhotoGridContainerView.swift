#if os(iOS)
import UIKit

/// Hosts the grid, with no blur or band behind the navigation bar: photos run straight up under
/// it.
final class PhotoGridContainerView: UIView {
    let collectionView: PhotoGridView
    let dateHeader = PhotoGridDateHeader()
    let dateChips = PhotoGridDateChips()

    init(collectionView: PhotoGridView) {
        self.collectionView = collectionView
        super.init(frame: .zero)
        collectionView.topEdgeEffect.isHidden = true
        collectionView.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        addSubview(collectionView)
        addSubview(dateHeader)
        // Over the header's blur, so the pinned one stays sharp under the title.
        addSubview(dateChips)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    /// The hitch monitor's readout (performance tests only), hung on the window so it's found
    /// whatever the grid's accessibility looks like at the time.
    var statsView: UIView?

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if let statsView, let window { window.addSubview(statsView) }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        collectionView.frame = bounds
        dateChips.frame = bounds
        // Under the bars, which only settle after the first layout.
        dateHeader.layout(in: bounds, top: safeAreaInsets.top)
    }

    override func safeAreaInsetsDidChange() {
        super.safeAreaInsetsDidChange()
        setNeedsLayout()
    }
}
#endif
