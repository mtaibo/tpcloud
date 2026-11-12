#if os(iOS)
import UIKit

/// Drives the grid's zoom the way the Photos app does. The grid never loses its shape: the grid
/// being left is frozen into a snapshot that scales with the fingers about the middle of the grid
/// (tiles slide off both sides evenly when zooming in), the new grid scales underneath it at
/// exactly the same tile size and lattice, and the two cross-fade, so each photo fades into its
/// new cell instead of travelling there. One pinch can sweep through several column counts;
/// letting go springs on to whichever the pinch was heading for, carrying the fingers' speed;
/// past the densest and the largest levels the grid stretches and springs back.
///
/// Every frame of a zoom is only layer transforms and one opacity: the new grid is laid out once,
/// at rest, when the zoom starts, and scaled as a whole (`sublayerTransform`). It used to be
/// re-laid out on every frame through a scaled layout, which at 15 or 31 columns meant moving
/// 1,000–2,000 cells 120 times a second: zooming out stuttered.
@MainActor
final class PhotoGridZoomController: NSObject {
    weak var collectionView: PhotoGridView?
    private(set) var level: Int
    /// After the grid settles on a new level: refresh cells and remember the level.
    var onLevelCommitted: (Int) -> Void = { _ in }
    /// Just before the grid for a level is installed (a zoom starting, settling or going back):
    /// cells made from here on should be configured for it.
    var levelWillInstall: (Int) -> Void = { _ in }
    /// Right after: cells kept on screen across the swap should be refreshed for it.
    var levelDidInstall: () -> Void = {}
    /// After a zoom ends, either way: apply anything held back while it ran.
    var onZoomEnded: () -> Void = {}

    var isZooming: Bool {
        session != nil || isPinching || isStartingSession
    }

    /// Set while a zoom lays out its new level, so tiles placed then get the quick thumbnails.
    private var isStartingSession = false

    private struct Session {
        let targetLevel: Int
        /// New tile size ÷ old tile size: above 1 zooming in.
        let ratio: CGFloat
        /// The on-screen point both grids scale about.
        let pivot: CGPoint
        /// The grid being left: its tiles, moved into a layer over the collection view.
        let snapshot: CALayer
        let leavingTiles: [PhotoTile]
        /// The part of the snapshot's mask over the new grid's footprint, which fades out (see
        /// `startSession`); the rest of the mask stays solid.
        let fade: CALayer
        let offsetFrom: CGPoint
        let offsetTo: CGPoint
        var progress: CGFloat = 0
    }

    /// A critically damped spring from `from` to `to` (no overshoot), starting at `velocity`
    /// (progress per second): a quick flick lands sooner, a slow release eases in.
    private struct Settle {
        let start: CFTimeInterval
        let from: CGFloat
        let to: CGFloat
        let velocity: CGFloat
        let omega: CGFloat

        func value(at time: CFTimeInterval) -> (value: CGFloat, done: Bool) {
            let t = CGFloat(max(time - start, 0))
            let d = from - to
            let decay = exp(-omega * t)
            let value = to + (d + (velocity + omega * d) * t) * decay
            let speed = (velocity - (velocity + omega * d) * omega * t) * decay
            let done = (abs(value - to) < 0.002 && abs(speed) < 0.05) || t > 1.5
            return (done ? to : value, done)
        }
    }

    private var session: Session?
    private var isPinching = false
    private var baselineScale: CGFloat = 1
    private var focalItem = 0
    /// Where in the focal tile the fingers are, 0...1 from its top.
    private var focalUnitY: CGFloat = 0.5
    /// The fingers' position on screen.
    private var finger: CGPoint = .zero
    private var settle: Settle?
    private var displayLink: CADisplayLink?
    private var isStretched = false

    init(level: Int) {
        self.level = level
    }

    private static var levels: [Int] { PhotoGridDensity.columnLevels }

    private func geometry(level: Int) -> PhotoGridGeometry? {
        guard let collectionView else { return nil }
        return collectionView.geometry(columns: Self.levels[level])
    }

    // MARK: Pinch

    @objc func handlePinch(_ pinch: UIPinchGestureRecognizer) {
        guard let collectionView else { return }
        switch pinch.state {
        case .began:
            completeSettleNow()
            isPinching = true
            collectionView.isPinnedToBottom = false
            // Taking over from the scroll view's own pan for the length of the pinch; disabling
            // the recognizer also cancels a drag already underway.
            collectionView.panGestureRecognizer.isEnabled = false
            baselineScale = pinch.scale
            let location = pinch.location(in: collectionView)
            finger = CGPoint(x: location.x - collectionView.contentOffset.x, y: location.y - collectionView.contentOffset.y)
            setFocus(at: location)
        case .changed:
            pinchChanged(scale: pinch.scale / baselineScale, rawScale: pinch.scale)
        case .ended, .cancelled, .failed:
            isPinching = false
            collectionView.panGestureRecognizer.isEnabled = true
            pinchEnded(velocity: pinch.velocity, scale: pinch.scale / baselineScale, cancelled: pinch.state != .ended)
        default:
            break
        }
    }

    private func setFocus(at location: CGPoint) {
        guard let collectionView, let geometry = geometry(level: level), geometry.itemCount > 0 else { return }
        let item = collectionView.item(at: location) ?? nearestItem(to: location, in: geometry)
        focalItem = item
        let frame = geometry.frame(forItem: item)
        focalUnitY = frame.height > 0 ? min(max((location.y - frame.minY) / frame.height, 0), 1) : 0.5
    }

    /// The tile closest to a point that fell in a gap (or beyond the content).
    private func nearestItem(to point: CGPoint, in geometry: PhotoGridGeometry) -> Int {
        let y = min(max(point.y, 0), max(geometry.contentHeight - 1, 0))
        let row = geometry.items(fromY: y, toY: y)
        guard !row.isEmpty else { return geometry.itemCount - 1 }
        return row.min { abs(geometry.frame(forItem: $0).midX - point.x) < abs(geometry.frame(forItem: $1).midX - point.x) } ?? row.lowerBound
    }

    private func pinchChanged(scale: CGFloat, rawScale: CGFloat) {
        // Spreading the fingers zooms in: fewer, bigger columns.
        let target = level + (scale >= 1 ? -1 : 1)
        guard Self.levels.indices.contains(target) else {
            revertSession()
            stretch(scale)
            return
        }
        if isStretched { stretch(1) }
        if session?.targetLevel != target {
            revertSession()
            startSession(to: target)
        }
        guard let session else { return }
        let progress = log(scale) / log(session.ratio)
        if progress >= 1 {
            // Reached the next level with the fingers still going: land on it and carry on from
            // there in the same pinch.
            setProgress(1)
            commit()
            baselineScale = rawScale
        } else {
            setProgress(max(progress, 0))
        }
    }

    private func pinchEnded(velocity: CGFloat, scale: CGFloat, cancelled: Bool) {
        if isStretched { springBackStretch() }
        guard let session else {
            onZoomEnded()
            return
        }
        let zoomingIn = session.ratio > 1
        let flicked = abs(velocity) > 0.6
        let towardTarget = zoomingIn ? velocity > 0 : velocity < 0
        let finish: Bool
        if cancelled {
            finish = false
        } else if flicked {
            finish = towardTarget ? session.progress > 0.05 : session.progress > 0.9
        } else {
            finish = session.progress > 0.5
        }
        // The pinch's speed (scale per second) in progress per second, so the spring carries on
        // at the speed the fingers left off.
        let progressVelocity = scale > 0 ? velocity / scale / log(session.ratio) : 0
        settle(to: finish ? 1 : 0, velocity: cancelled ? 0 : progressVelocity, response: 0.3)
    }

    // MARK: Zoom In / Zoom Out (menu)

    /// Animates to `newLevel` around the middle of the screen, as if pinched there.
    func zoom(toLevel newLevel: Int) {
        guard let collectionView else { return }
        let insets = collectionView.adjustedContentInset
        let middle = CGPoint(
            x: collectionView.contentOffset.x + collectionView.bounds.width / 2,
            y: collectionView.contentOffset.y + insets.top + (collectionView.bounds.height - insets.top - insets.bottom) / 2
        )
        zoom(toLevel: newLevel, around: middle)
    }

    /// Animates to `newLevel` — any number of levels away, in one smooth step — around a point in
    /// content coordinates (a tapped photo), keeping that photo's row where it is on screen.
    func zoom(toLevel newLevel: Int, around point: CGPoint, response: CGFloat = 0.42) {
        guard let collectionView, newLevel != level, Self.levels.indices.contains(newLevel), !isPinching else { return }
        completeSettleNow()
        collectionView.isPinnedToBottom = false
        finger = CGPoint(x: point.x - collectionView.contentOffset.x, y: point.y - collectionView.contentOffset.y)
        setFocus(at: point)
        guard !UIAccessibility.isReduceMotionEnabled else {
            jump(to: newLevel)
            return
        }
        startSession(to: newLevel)
        settle(to: 1, velocity: 0, response: response)
    }

    // MARK: Sessions

    /// Freezes the current grid and lays the `target` grid out underneath it, scaled so its tiles
    /// match the current ones in size and sit on the same lattice, with the focal photo's row
    /// in the same place in both.
    private func startSession(to target: Int) {
        guard let collectionView, let container = collectionView.superview,
              let from = geometry(level: level), let to = geometry(level: target),
              from.itemCount > 0, from.pitch > 0, to.pitch != from.pitch else { return }
        let offsetFrom = collectionView.contentOffset
        // Where the new grid comes to rest: the focal photo as close under the fingers as the
        // scrollable range allows, so the zoom ends without any correcting jump.
        let focalTo = to.frame(forItem: focalItem)
        let restingY = focalTo.minY + focalTo.height * focalUnitY - finger.y
        let offsetTo = CGPoint(x: offsetFrom.x, y: clampedOffset(restingY, contentHeight: to.contentHeight))

        // The point both grids scale about. Across: the middle of the grid, so columns leave and
        // arrive evenly on both sides — and since every level has an odd column count, the middle
        // column and with it every column of the two grids line up. Down: the height that maps
        // the focal photo's row in the old grid onto its row in the new one, so rows line up too.
        let ratio = to.pitch / from.pitch
        let a = from.frame(forItem: focalItem).offsetBy(dx: -offsetFrom.x, dy: -offsetFrom.y)
        let b = focalTo.offsetBy(dx: -offsetTo.x, dy: -offsetTo.y)
        let pivot = CGPoint(x: to.width / 2 - offsetTo.x, y: (ratio * a.minY - b.minY) / (ratio - 1))

        // The grid being left sits over the collection view (not in it), so scaling the
        // collection view's contents leaves it alone.
        PhotoGridHitchMonitor.current?.zoomStart = CACurrentMediaTime()
        var mark = CACurrentMediaTime()
        func step(_ name: String) {
            let now = CACurrentMediaTime()
            PhotoGridHitchMonitor.current?.zoomStep(name, now - mark)
            mark = now
        }
        isStartingSession = true
        defer { isStartingSession = false }
        let (snapshot, leavingTiles) = collectionView.detachPlacedTiles()
        step("detach")
        container.layer.insertSublayer(snapshot, above: collectionView.layer)

        // The new grid, laid out once at rest; from here on it only moves as a whole.
        collectionView.layer.removeAnimation(forKey: "stretch")
        collectionView.showsVerticalScrollIndicator = false
        levelWillInstall(target)
        install(columns: Self.levels[target], offset: offsetTo)
        step("install")
        collectionView.layoutIfNeeded()
        levelDidInstall()
        step("layout")

        // Only the part of the snapshot over the new grid fades. Zooming in, the new grid starts
        // out smaller than the screen (its tiles are shrunk to the old size), so there's nothing
        // under the snapshot's outer rows and columns: fading those too would show the background.
        // They stay solid and slide off the edges as the grid grows. In the snapshot's own
        // (untransformed) coordinates that part is fixed: the screen scaled by 1/ratio about the
        // pivot. Zooming out it covers the whole screen, so all of it fades.
        let bounds = snapshot.bounds
        let footprint = CGRect(
            x: pivot.x - pivot.x / ratio,
            y: pivot.y - pivot.y / ratio,
            width: bounds.width / ratio,
            height: bounds.height / ratio
        ).intersection(bounds)
        // Zooming out the footprint is the whole screen: fade the whole layer, no mask (a mask
        // means an offscreen pass on every frame).
        let fade: CALayer
        let coversScreen = !footprint.isNull && footprint.minX <= 0.5 && footprint.minY <= 0.5
            && footprint.maxX >= bounds.maxX - 0.5 && footprint.maxY >= bounds.maxY - 0.5
        if coversScreen {
            fade = snapshot
        } else {
            let mask = CALayer()
            mask.frame = bounds
            let inner = footprint.isNull ? .zero : footprint
            for band in [
                CGRect(x: 0, y: 0, width: bounds.width, height: inner.minY),
                CGRect(x: 0, y: inner.maxY, width: bounds.width, height: bounds.height - inner.maxY),
                CGRect(x: 0, y: inner.minY, width: inner.minX, height: inner.height),
                CGRect(x: inner.maxX, y: inner.minY, width: bounds.width - inner.maxX, height: inner.height),
            ] where band.width > 0 && band.height > 0 {
                let layer = CALayer()
                layer.frame = band
                layer.backgroundColor = UIColor.black.cgColor
                mask.addSublayer(layer)
            }
            let fadeLayer = CALayer()
            // Its opacity follows the fingers frame by frame; no implicit fades.
            fadeLayer.actions = ["opacity": NSNull()]
            fadeLayer.frame = inner
            fadeLayer.backgroundColor = UIColor.black.cgColor
            mask.addSublayer(fadeLayer)
            snapshot.mask = mask
            fade = fadeLayer
        }

        session = Session(
            targetLevel: target,
            ratio: ratio,
            pivot: pivot,
            snapshot: snapshot,
            leavingTiles: leavingTiles,
            fade: fade,
            offsetFrom: offsetFrom,
            offsetTo: offsetTo
        )
        setProgress(0)
    }

    /// A scale about `pivot` (in the view's coordinates) for a layer that transforms about its
    /// centre.
    private func scale(_ scale: CGFloat, about pivot: CGPoint, in size: CGSize) -> CGAffineTransform {
        CGAffineTransform(
            a: scale, b: 0, c: 0, d: scale,
            tx: (pivot.x - size.width / 2) * (1 - scale),
            ty: (pivot.y - size.height / 2) * (1 - scale)
        )
    }

    private func setProgress(_ progress: CGFloat) {
        guard let collectionView, var session else { return }
        let progress = min(max(progress, 0), 1)
        session.progress = progress
        self.session = session
        // The old grid grows (or shrinks) by `scale` about the pivot, and the new one with it,
        // from the old tile size (1/ratio) to its own (1).
        let scale = pow(session.ratio, progress)
        let size = collectionView.bounds.size
        // No CATransaction of its own: from the display link that was a second full commit of
        // every layer on each frame. A view's own layer and the fade layer don't animate
        // implicitly anyway.
        session.snapshot.setAffineTransform(self.scale(scale, about: session.pivot, in: size))
        collectionView.layer.sublayerTransform = CATransform3DMakeAffineTransform(
            self.scale(scale / session.ratio, about: session.pivot, in: size)
        )
        session.fade.opacity = Float(1 - progress)
    }

    private func endSession(_ session: Session) {
        session.snapshot.removeFromSuperlayer()
        collectionView?.recycleDetached(session.leavingTiles)
        self.session = nil
        collectionView?.layer.sublayerTransform = CATransform3DIdentity
        collectionView?.showsVerticalScrollIndicator = true
    }

    /// Lands on the session's target: the new grid is already at rest underneath.
    private func commit() {
        guard let session else { return }
        let start = CACurrentMediaTime()
        endSession(session)
        level = session.targetLevel
        finishLevelChange()
        PhotoGridHitchMonitor.current?.zoomStep("commit", CACurrentMediaTime() - start)
    }

    /// Back to the grid the session started from, exactly as it was.
    private func revertSession() {
        guard let session else { return }
        endSession(session)
        levelWillInstall(level)
        install(columns: Self.levels[level], offset: session.offsetFrom)
        collectionView?.layoutIfNeeded()
        levelDidInstall()
    }

    /// Straight to a level with no animation (Reduce Motion, or a jump of several levels).
    private func jump(to newLevel: Int) {
        guard let collectionView, let to = geometry(level: newLevel), to.itemCount > 0 else { return }
        let frame = to.frame(forItem: min(focalItem, to.itemCount - 1))
        let y = clampedOffset(frame.minY + frame.height * focalUnitY - finger.y, contentHeight: to.contentHeight)
        level = newLevel
        levelWillInstall(newLevel)
        install(columns: Self.levels[newLevel], offset: CGPoint(x: collectionView.contentOffset.x, y: y))
        collectionView.layoutIfNeeded()
        levelDidInstall()
        finishLevelChange()
        onZoomEnded()
    }

    private func finishLevelChange() {
        onLevelCommitted(level)
    }

    private func install(columns: Int, offset: CGPoint) {
        collectionView?.setColumns(columns, offset: offset)
    }

    private func clampedOffset(_ y: CGFloat, contentHeight: CGFloat) -> CGFloat {
        guard let collectionView else { return y }
        let insets = collectionView.adjustedContentInset
        let minY = -insets.top
        let maxY = max(contentHeight + insets.bottom - collectionView.bounds.height, minY)
        return min(max(y, minY), maxY)
    }

    // MARK: Settling after the fingers lift

    private func settle(to toProgress: CGFloat, velocity: CGFloat, response: CGFloat) {
        guard let session else { return }
        settle = Settle(
            start: CACurrentMediaTime(),
            from: session.progress,
            to: toProgress,
            // Heading the wrong way, the speed is dropped rather than carried into a reversal.
            velocity: (toProgress - session.progress) * velocity >= 0 ? min(max(velocity, -12), 12) : 0,
            omega: 2 * .pi / response
        )
        if displayLink == nil {
            let link = CADisplayLink(target: self, selector: #selector(step(_:)))
            link.preferredFrameRateRange = CAFrameRateRange(minimum: 80, maximum: 120, preferred: 120)
            link.add(to: .main, forMode: .common)
            displayLink = link
        }
    }

    @objc private func step(_ link: CADisplayLink) {
        advanceSettle(to: link.targetTimestamp)
    }

    private func advanceSettle(to time: CFTimeInterval, finish: Bool = false) {
        guard let settle, session != nil else {
            stopDisplayLink()
            return
        }
        let (value, done) = finish ? (settle.to, true) : settle.value(at: time)
        setProgress(value)
        guard done else { return }
        self.settle = nil
        stopDisplayLink()
        if settle.to == 1 {
            commit()
        } else {
            revertSession()
        }
        onZoomEnded()
    }

    private func completeSettleNow() {
        guard settle != nil else { return }
        advanceSettle(to: CACurrentMediaTime(), finish: true)
    }

    private func stopDisplayLink() {
        displayLink?.invalidate()
        displayLink = nil
    }

    // MARK: Stretch past the ends

    /// Scales the grid slightly around the fingers, resisting more the further the pinch goes.
    private func stretch(_ scale: CGFloat) {
        guard let collectionView else { return }
        let amount = pow(scale, 0.2)
        isStretched = amount != 1
        let dx = finger.x - collectionView.bounds.width / 2
        let dy = finger.y - collectionView.bounds.height / 2
        var transform = CATransform3DMakeTranslation(dx, dy, 0)
        transform = CATransform3DScale(transform, amount, amount, 1)
        transform = CATransform3DTranslate(transform, -dx, -dy, 0)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        collectionView.layer.sublayerTransform = transform
        CATransaction.commit()
    }

    private func springBackStretch() {
        guard let layer = collectionView?.layer else { return }
        isStretched = false
        let animation = CASpringAnimation(keyPath: "sublayerTransform")
        animation.fromValue = layer.presentation()?.sublayerTransform ?? layer.sublayerTransform
        animation.toValue = CATransform3DIdentity
        animation.damping = 26
        animation.stiffness = 300
        animation.duration = animation.settlingDuration
        layer.sublayerTransform = CATransform3DIdentity
        layer.add(animation, forKey: "stretch")
    }
}
#endif
