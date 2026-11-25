#if os(macOS)
import AppKit
import os
import SwiftUI
import TPCore

/// The Mac's photo grid, built like the iPhone's (`PhotoGridView`): a scroll view whose tiles are
/// bare layers placed by arithmetic (`PhotoGridGeometry`), recycled as they scroll in and out,
/// filled from the thumbnail table or the loader (which downloads what's missing in bulk).
/// The SwiftUI grid it replaced built a view per tile and asked the host for each thumbnail on
/// its own, and a big library crawled.
///
/// Mac habits: a click selects, a double click (or Space/Return) opens, the arrow keys move the
/// selection, ⌘+ and ⌘− (or a pinch on the trackpad) change the size, Delete asks before deleting.
struct MacPhotoGrid: NSViewRepresentable {
    /// Newest first, as the server pages them.
    let photos: [Photo]
    let client: PhotosClient
    @Binding var columnLevel: Int
    let scrollToBottomSignal: Int
    let onOpen: (Photo) -> Void
    let onDelete: (Photo) -> Void

    func makeCoordinator() -> Coordinator { Coordinator(client: client) }

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSScrollView()
        scrollView.hasVerticalScroller = true
        scrollView.autohidesScrollers = true
        scrollView.drawsBackground = false
        scrollView.automaticallyAdjustsContentInsets = true
        let document = MacPhotoGridDocument()
        document.coordinator = context.coordinator
        scrollView.documentView = document
        scrollView.contentView.postsBoundsChangedNotifications = true
        let dates = MacPhotoGridDates(frame: scrollView.bounds)
        dates.autoresizingMask = [.width, .height]
        scrollView.addSubview(dates, positioned: .above, relativeTo: nil)
        context.coordinator.attach(scrollView, document: document, dates: dates)
        return scrollView
    }

    /// Whatever it's offered: left to itself SwiftUI works the size out from the subviews'
    /// frames, so a date label further down raised the grid's minimum height, which resized the
    /// split view, which laid the grid out again and moved the label — a loop AppKit ends by
    /// throwing.
    func sizeThatFits(_ proposal: ProposedViewSize, nsView: NSScrollView, context: Context) -> CGSize? {
        proposal.replacingUnspecifiedDimensions(by: CGSize(width: 400, height: 300))
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        let coordinator = context.coordinator
        coordinator.onOpen = onOpen
        coordinator.onDelete = onDelete
        coordinator.setLevel = { level in columnLevel = level }
        coordinator.update(photos: photos, level: columnLevel)
        if coordinator.scrollSignal != scrollToBottomSignal {
            coordinator.scrollSignal = scrollToBottomSignal
            coordinator.scrollToNewest()
        }
    }

    @MainActor
    final class Coordinator: NSObject {
        let loader: PhotoThumbnailLoader
        var onOpen: (Photo) -> Void = { _ in }
        var onDelete: (Photo) -> Void = { _ in }
        var setLevel: (Int) -> Void = { _ in }
        var scrollSignal = 0

        private weak var scrollView: NSScrollView?
        private weak var document: MacPhotoGridDocument?
        private weak var dates: MacPhotoGridDates?
        private var monthGroups: [MacPhotoGridDates.Group] = []
        private var yearGroups: [MacPhotoGridDates.Group] = []
        private var monthStartsTask: Task<Void, Never>?
        /// Every tile, the selection outline and nothing else: what a zoom scales.
        private let container = CALayer()
        private let selectionLayer = CALayer()
        private var photos: [Photo] = []
        /// The level the grid is at: during a zoom still the one it started from, until it lands.
        private var level = 0 {
            didSet { shownLevel = level }
        }
        /// The level laid out right now: a zoom's target while it runs (under the snapshot).
        /// Keeping it apart from `level` is what the iPhone does; mixing the two made each event
        /// of a pinch think the grid was already a level further and start another zoom from
        /// there, sweeping through every level sixty times a second.
        private var shownLevel = 0
        private(set) var geometry = PhotoGridGeometry(columns: 5, width: 0, itemCount: 0, scale: 2)
        /// One tile per photo (up to 9 columns), or one per row (15 and 31, see `drawsRows`).
        private var tiles: [Int: MacPhotoTile] = [:]
        private var rowTiles: [Int: MacPhotoTile] = [:]
        private var spare: [MacPhotoTile] = []
        private let rows = PhotoGridRowRenderer()
        private var rowsAwaitingThumbnails: Set<String> = []
        private var rowRedrawScheduled = false
        /// Stays at the newest photos (the bottom) until the person scrolls away.
        private var pinnedToBottom = true
        private var selectedID: Int?

        init(client: PhotosClient) {
            loader = PhotoThumbnailLoader(client: client)
            super.init()
            for layer in [container, selectionLayer] {
                layer.actions = ["position": NSNull(), "bounds": NSNull(), "hidden": NSNull(), "transform": NSNull(), "opacity": NSNull()]
            }
            container.anchorPoint = .zero
            selectionLayer.borderWidth = 3
            selectionLayer.borderColor = NSColor.controlAccentColor.cgColor
            selectionLayer.isHidden = true
            selectionLayer.zPosition = 10
            container.addSublayer(selectionLayer)
        }

        func attach(_ scrollView: NSScrollView, document: MacPhotoGridDocument, dates: MacPhotoGridDates? = nil) {
            self.scrollView = scrollView
            self.document = document
            self.dates = dates
            document.layer?.addSublayer(container)
            NotificationCenter.default.addObserver(self, selector: #selector(scrolled), name: NSView.boundsDidChangeNotification, object: scrollView.contentView)
            NotificationCenter.default.addObserver(self, selector: #selector(resized), name: NSView.frameDidChangeNotification, object: scrollView)
            scrollView.postsFrameChangedNotifications = true
        }

        private var columns: Int { PhotoGridDensity.columns(forLevel: shownLevel) }

        /// From 15 columns each row is one picture, drawn off the main thread from the thumbnail
        /// table, like the iPhone's grid: 31 columns would otherwise be some 2,000 layers on
        /// screen, and that's what made the densest grid crawl.
        private var drawsRows: Bool { columns >= 15 }

        /// Item 0 is the oldest (top left), the last one the newest (bottom right).
        private func photo(at item: Int) -> Photo { photos[photos.count - 1 - item] }

        private func item(of photoID: Int) -> Int? {
            photos.firstIndex { $0.id == photoID }.map { photos.count - 1 - $0 }
        }

        func update(photos newPhotos: [Photo], level newLevel: Int) {
            let photosChanged = newPhotos.count != photos.count || newPhotos.first?.id != photos.first?.id || newPhotos.last?.id != photos.last?.id
            let levelChanged = newLevel != level
            guard photosChanged || levelChanged || geometry.width == 0 else { return }
            // The level changed from outside (the ••• menu's Zoom In / Zoom Out): zoom there.
            if levelChanged, !photosChanged, geometry.width > 0 {
                // While our own level waits to be saved, the setting still says the old one.
                if saveTask == nil, session == nil, settle == nil, !isPinching { zoom(toLevel: newLevel) }
                return
            }
            // Photos arriving mid-zoom wait until it's over; relaying out under the fingers is what
            // made the grid jump.
            if isZooming || settle != nil {
                pendingPhotos = newPhotos
                return
            }
            let fromBottom = distanceFromBottom()
            if photosChanged { computeMonthStarts(newPhotos) }
            photos = newPhotos
            if saveTask == nil { level = newLevel }
            relayout()
            scheduleRestWork()
            if pinnedToBottom {
                scrollToNewest()
            } else if let fromBottom {
                // Older photos came in above: keep what's on screen where it was.
                scroll(toY: documentHeight - visibleHeight - fromBottom)
            }
        }

        // MARK: Zoom (the iPhone's, `PhotoGridZoomController`, on the trackpad)

        /// One zoom from `level` to `target`. The grid being left is frozen into `snapshot` and
        /// scales about `pivot` with the fingers; the new grid, laid out once at rest, scales under
        /// it at exactly the same tile size and lattice (the container's transform), and the two
        /// cross-fade, so each photo fades into its new cell instead of travelling there. Every
        /// frame is only two transforms and one opacity.
        private struct Session {
            let from: Int
            let target: Int
            /// New tile size ÷ old: above 1 zooming in.
            let ratio: CGFloat
            /// On screen (the visible area's coordinates).
            let pivot: CGPoint
            let snapshot: CALayer
            /// What fades: the snapshot, or (zooming in) the part of its mask over the new grid.
            let fade: CALayer
            let scrollFrom: CGFloat
            let scrollTo: CGFloat
            var progress: CGFloat = 0
        }

        /// Critically damped (no overshoot), from `from` to `to`, starting at `velocity` (progress per second).
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

        private static let zoomLog = Logger(subsystem: "com.migueltaibo.tpcloud", category: "zoom")
        private var session: Session?
        private var settle: Settle?
        private var displayLink: CADisplayLink?
        private var isPinching = false
        private var pinchScale: CGFloat = 1
        private var baselineScale: CGFloat = 1
        private var pinchSamples: [(time: CFTimeInterval, scale: CGFloat)] = []
        private var focalItem = 0
        /// Where in the focal tile the pointer is, 0...1 from its top.
        private var focalUnitY: CGFloat = 0.5
        /// The pointer, in the visible area's coordinates.
        private var finger: CGPoint = .zero
        private var isStretched = false

        private var isZooming: Bool { session != nil || isPinching }
        private var drawsRowsNow = false
        private var pendingPhotos: [Photo]?

        private func applyPendingPhotos() {
            guard let pending = pendingPhotos, !isZooming, settle == nil else { return }
            pendingPhotos = nil
            update(photos: pending, level: level)
        }
        private var scrollY: CGFloat { scrollView?.contentView.bounds.minY ?? 0 }

        private func geometry(level: Int) -> PhotoGridGeometry {
            PhotoGridGeometry(columns: PhotoGridDensity.columns(forLevel: level), width: geometry.width, itemCount: photos.count, scale: geometry.scale)
        }

        func magnify(_ event: NSEvent, at point: CGPoint) {
            switch event.phase {
            case .began:
                completeSettleNow()
                isPinching = true
                pinchScale = 1
                baselineScale = 1
                pinchSamples = []
                pinnedToBottom = false
                finger = CGPoint(x: point.x, y: point.y - scrollY)
                setFocus(at: point)
            case .changed:
                guard isPinching else { return }
                pinchScale *= 1 + event.magnification
                let now = CACurrentMediaTime()
                pinchSamples.append((now, pinchScale))
                pinchSamples.removeAll { now - $0.time > 0.1 }
                pinchChanged(scale: pinchScale / baselineScale, rawScale: pinchScale)
            case .ended, .cancelled:
                guard isPinching else { return }
                isPinching = false
                var velocity: CGFloat = 0
                if let first = pinchSamples.first, let last = pinchSamples.last, last.time - first.time > 0.01 {
                    velocity = (last.scale - first.scale) / CGFloat(last.time - first.time)
                }
                pinchEnded(velocity: velocity, scale: pinchScale / baselineScale, cancelled: event.phase == .cancelled)
            default:
                break
            }
        }

        private func setFocus(at point: CGPoint) {
            guard geometry.itemCount > 0 else { return }
            let item = item(at: point) ?? nearestItem(to: point)
            focalItem = item
            let frame = geometry.frame(forItem: item)
            focalUnitY = frame.height > 0 ? min(max((point.y - frame.minY) / frame.height, 0), 1) : 0.5
        }

        private func nearestItem(to point: CGPoint) -> Int {
            let y = min(max(point.y, 0), max(geometry.contentHeight - 1, 0))
            let row = geometry.items(fromY: y, toY: y)
            guard !row.isEmpty else { return geometry.itemCount - 1 }
            return row.min { abs(geometry.frame(forItem: $0).midX - point.x) < abs(geometry.frame(forItem: $1).midX - point.x) } ?? row.lowerBound
        }

        private func pinchChanged(scale: CGFloat, rawScale: CGFloat) {
            let logScale = log(scale)
            // A slow pinch hovers around 1 for a while: nothing starts until it clearly goes one
            // way, and a zoom underway doesn't turn into the opposite one until the fingers are
            // clearly past where they started. Without this, every wobble tore the grid down and
            // laid it out again, dozens of times a second.
            if session == nil, !isStretched, abs(logScale) < 0.03 { return }
            if let session, (session.ratio > 1) != (logScale >= 0), abs(logScale) < 0.06 {
                setProgress(0)
                return
            }
            // Spreading the fingers zooms in: fewer, bigger columns.
            let target = level + (logScale >= 0 ? -1 : 1)
            guard PhotoGridDensity.columnLevels.indices.contains(target) else {
                revertSession()
                stretch(scale)
                return
            }
            if isStretched { stretch(1) }
            if session?.target != target {
                revertSession()
                startSession(to: target)
            }
            guard let session else { return }
            let progress = log(scale) / log(session.ratio)
            if progress >= 1 {
                // Reached the next level with the fingers still going: land on it and carry on.
                setProgress(1)
                commit()
                baselineScale = rawScale
            } else {
                setProgress(max(progress, 0))
            }
        }

        private func pinchEnded(velocity: CGFloat, scale: CGFloat, cancelled: Bool) {
            if isStretched { springBackStretch() }
            guard let session else { return applyPendingPhotos() }
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
            let progressVelocity = scale > 0 ? velocity / scale / log(session.ratio) : 0
            settle(to: finish ? 1 : 0, velocity: cancelled ? 0 : progressVelocity, response: 0.3)
        }

        /// ⌘+ / ⌘−, the menu, a double click: animated to `newLevel` about a point (the middle of
        /// the screen if none), as if pinched there.
        func zoom(toLevel newLevel: Int, around point: CGPoint? = nil, response: CGFloat = 0.42) {
            guard let clip = scrollView?.contentView, newLevel != level, PhotoGridDensity.columnLevels.indices.contains(newLevel), !isPinching else { return }
            completeSettleNow()
            let point = point ?? CGPoint(x: clip.bounds.midX, y: clip.bounds.midY)
            pinnedToBottom = false
            finger = CGPoint(x: point.x, y: point.y - clip.bounds.minY)
            setFocus(at: point)
            if NSWorkspace.shared.accessibilityDisplayShouldReduceMotion {
                jump(to: newLevel)
                return
            }
            startSession(to: newLevel)
            settle(to: 1, velocity: 0, response: response)
        }

        /// Freezes the current grid and lays the `target` grid out underneath, scaled so its tiles
        /// match the current ones in size and sit on the same lattice, with the focal photo's row
        /// in the same place in both.
        private func startSession(to target: Int) {
            #if DEBUG
            debugStarts += 1
            #endif
            let started = CACurrentMediaTime()
            defer { Self.zoomLog.debug("start \(self.level, privacy: .public)→\(target, privacy: .public) took \(Int((CACurrentMediaTime() - started) * 1000), privacy: .public) ms, tiles \(self.tiles.count + self.rowTiles.count, privacy: .public), scroll \(self.scrollY, privacy: .public)") }
            guard let clip = scrollView?.contentView, let host = document?.layer, geometry.itemCount > 0, geometry.pitch > 0 else { return }
            let from = geometry
            let to = geometry(level: target)
            guard to.pitch > 0, to.pitch != from.pitch else { return }
            let scrollFrom = clip.bounds.minY
            let fromLevel = level
            // Where the new grid comes to rest: the focal photo under the pointer as far as the
            // scrollable range allows, so the zoom ends without a correcting jump.
            let focalTo = to.frame(forItem: focalItem)
            let scrollTo = clampedScroll(focalTo.minY + focalTo.height * focalUnitY - finger.y, contentHeight: to.contentHeight)
            // Across, the middle of the grid (odd column counts: the middle column, and with it
            // every column, lines up); down, the height that maps the focal row onto its new row.
            let ratio = to.pitch / from.pitch
            let a = from.frame(forItem: focalItem).offsetBy(dx: 0, dy: -scrollFrom)
            let b = focalTo.offsetBy(dx: 0, dy: -scrollTo)
            let pivot = CGPoint(x: to.width / 2, y: (ratio * a.minY - b.minY) / (ratio - 1))

            let visible = clip.bounds
            let snapshot = CALayer()
            snapshot.anchorPoint = .zero
            snapshot.actions = ["transform": NSNull(), "opacity": NSNull(), "position": NSNull(), "bounds": NSNull()]
            snapshot.zPosition = 20
            for tile in Array(tiles.values) + Array(rowTiles.values) where tile.layer.frame.intersects(visible) {
                let copy = CALayer()
                copy.actions = ["position": NSNull(), "bounds": NSNull()]
                copy.frame = tile.layer.frame.offsetBy(dx: 0, dy: -scrollFrom)
                copy.contents = tile.layer.contents
                copy.contentsGravity = tile.layer.contentsGravity
                copy.contentsScale = tile.layer.contentsScale
                copy.masksToBounds = true
                copy.cornerRadius = tile.layer.cornerRadius
                copy.backgroundColor = tile.layer.contents == nil ? tile.layer.backgroundColor : nil
                snapshot.addSublayer(copy)
            }

            CATransaction.begin()
            CATransaction.setDisableActions(true)
            shownLevel = target
            // Rows not drawn yet are drawn right now (from the table, ~1 ms each) rather than
            // showing grey through the zoom.
            drawsRowsNow = true
            relayout()
            scroll(toY: scrollTo)
            drawsRowsNow = false
            snapshot.frame = CGRect(x: 0, y: clip.bounds.minY, width: visible.width, height: visible.height)
            host.addSublayer(snapshot)

            // Zooming in the new grid starts out smaller than the screen, so only the part of the
            // snapshot over its footprint fades; the rest stays solid and slides off the edges.
            // Zooming out it covers everything, and the whole snapshot fades (no mask to draw).
            let bounds = CGRect(origin: .zero, size: visible.size)
            let footprint = CGRect(x: pivot.x - pivot.x / ratio, y: pivot.y - pivot.y / ratio,
                                   width: bounds.width / ratio, height: bounds.height / ratio).intersection(bounds)
            let coversScreen = !footprint.isNull && footprint.minX <= 0.5 && footprint.minY <= 0.5
                && footprint.maxX >= bounds.maxX - 0.5 && footprint.maxY >= bounds.maxY - 0.5
            let fade: CALayer
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
                    layer.backgroundColor = NSColor.black.cgColor
                    mask.addSublayer(layer)
                }
                let fadeLayer = CALayer()
                fadeLayer.actions = ["opacity": NSNull()]
                fadeLayer.frame = inner
                fadeLayer.backgroundColor = NSColor.black.cgColor
                mask.addSublayer(fadeLayer)
                snapshot.mask = mask
                fade = fadeLayer
            }
            session = Session(from: fromLevel, target: target, ratio: ratio, pivot: pivot, snapshot: snapshot, fade: fade,
                              scrollFrom: scrollFrom, scrollTo: clip.bounds.minY)
            setProgress(0)
            CATransaction.commit()
        }

        private func setProgress(_ progress: CGFloat) {
            guard var session else { return }
            let progress = min(max(progress, 0), 1)
            session.progress = progress
            self.session = session
            let scale = pow(session.ratio, progress)
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            session.snapshot.transform = Self.scale(scale, about: session.pivot)
            // The container is in the document's coordinates: the same pivot, moved by the scroll.
            container.transform = Self.scale(scale / session.ratio, about: CGPoint(x: session.pivot.x, y: session.pivot.y + session.scrollTo))
            session.fade.opacity = Float(1 - progress)
            CATransaction.commit()
        }

        /// p′ = c + s (p − c), for a layer whose anchor is its origin.
        private static func scale(_ s: CGFloat, about c: CGPoint) -> CATransform3D {
            var t = CATransform3DMakeScale(s, s, 1)
            t.m41 = c.x * (1 - s)
            t.m42 = c.y * (1 - s)
            return t
        }

        private func endSession(_ session: Session) {
            session.snapshot.removeFromSuperlayer()
            self.session = nil
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            container.transform = CATransform3DIdentity
            CATransaction.commit()
        }

        private func endSessionIfAny() {
            if let session { endSession(session) }
        }

        /// Lands on the target: the new grid is already at rest underneath.
        private func commit() {
            guard let session else { return }
            Self.zoomLog.debug("commit \(session.target, privacy: .public)")
            endSession(session)
            level = session.target
            levelCommitted()
        }

        /// Back to the grid the session started from, exactly as it was.
        private func revertSession() {
            guard let session else { return }
            Self.zoomLog.debug("revert to \(session.from, privacy: .public) at progress \(session.progress, privacy: .public)")
            #if DEBUG
            debugReverts += 1
            #endif
            endSession(session)
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            shownLevel = level
            relayout()
            scroll(toY: session.scrollFrom)
            CATransaction.commit()
        }

        private func jump(to newLevel: Int) {
            let to = geometry(level: newLevel)
            let frame = to.frame(forItem: min(focalItem, max(to.itemCount - 1, 0)))
            level = newLevel
            relayout()
            scroll(toY: frame.minY + frame.height * focalUnitY - finger.y)
            levelCommitted()
        }

        private func levelCommitted() {
            dateHeaderKey = nil
            DispatchQueue.main.async { [weak self] in self?.updateDates() }
            if let fromBottom = distanceFromBottom() { pinnedToBottom = fromBottom < 4 }
            saveLevelSoon()
            prepareNeighbourRows()
        }

        private var saveTask: Task<Void, Never>?

        /// The level is saved 0.4 s after the grid comes to rest, like the iPhone's: writing the
        /// setting re-renders the gallery in SwiftUI, and doing it mid-pinch (one pinch can pass
        /// through several levels) made the zoom stutter.
        private func saveLevelSoon() {
            saveTask?.cancel()
            saveTask = Task { [weak self] in
                try? await Task.sleep(for: .milliseconds(400))
                guard !Task.isCancelled, let self else { return }
                if self.isZooming || self.settle != nil { return self.saveLevelSoon() }
                self.saveTask = nil
                self.setLevel(self.level)
            }
        }

        private func clampedScroll(_ y: CGFloat, contentHeight: CGFloat) -> CGFloat {
            guard let clip = scrollView?.contentView else { return y }
            let height = max(contentHeight, clip.bounds.height)
            return min(max(y, 0), max(height - clip.bounds.height, 0))
        }

        private func settle(to target: CGFloat, velocity: CGFloat, response: CGFloat) {
            guard let session else { return }
            settle = Settle(
                start: CACurrentMediaTime(),
                from: session.progress,
                to: target,
                velocity: (target - session.progress) * velocity >= 0 ? min(max(velocity, -12), 12) : 0,
                omega: 2 * .pi / response
            )
            if displayLink == nil, let document {
                let link = document.displayLink(target: self, selector: #selector(step(_:)))
                link.preferredFrameRateRange = CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
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
            if settle.to == 1 { commit() } else { revertSession() }
            applyPendingPhotos()
        }

        private func completeSettleNow() {
            guard settle != nil else { return }
            advanceSettle(to: CACurrentMediaTime(), finish: true)
        }

        private func stopDisplayLink() {
            displayLink?.invalidate()
            displayLink = nil
        }

        /// Past the largest or the densest level the grid stretches a little around the pointer
        /// and springs back on release.
        private func stretch(_ scale: CGFloat) {
            let amount = pow(scale, 0.2)
            isStretched = amount != 1
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            container.transform = Self.scale(amount, about: CGPoint(x: finger.x, y: finger.y + scrollY))
            CATransaction.commit()
        }

        private func springBackStretch() {
            isStretched = false
            let animation = CASpringAnimation(keyPath: "transform")
            animation.fromValue = container.presentation()?.transform ?? container.transform
            animation.toValue = CATransform3DIdentity
            animation.damping = 26
            animation.stiffness = 300
            animation.duration = animation.settlingDuration
            container.transform = CATransform3DIdentity
            container.add(animation, forKey: "stretch")
        }

        /// At rest, the row pictures of the levels a pinch would go to next, around what's on
        /// screen, so the dense grids are drawn before a zoom reaches them.
        private func prepareNeighbourRows() {
            guard let clip = scrollView?.contentView, !photos.isEmpty, geometry.pitch > 0 else { return }
            let visible = geometry.items(fromY: clip.bounds.minY, toY: clip.bounds.maxY)
            guard !visible.isEmpty else { return }
            for neighbour in [level - 1, level + 1, level] where PhotoGridDensity.columnLevels.indices.contains(neighbour) {
                let target = geometry(level: neighbour)
                guard target.columns >= 15, target.pitch > 0 else { continue }
                let middle = target.frame(forItem: visible.lowerBound + visible.count / 2)
                let span = clip.bounds.height * 1.5
                for row in rowRange(in: target, fromY: middle.midY - span, toY: middle.midY + span) {
                    if let request = rowRequest(row: row, in: target) { rows.render(request) }
                }
            }
        }

        #if DEBUG
        /// Tests: starts a zoom to `target` about `point` (document coordinates) and puts it at
        /// `progress`. Reports how far the new grid's cells are, on screen, from the old grid's
        /// lattice (at the start they must lie exactly on it), and where the photo under the
        /// pointer ends up relative to the pointer (at the end it must be back under it).
        func debugMismatch(to target: Int, around point: CGPoint, progress: CGFloat) -> (latticeError: CGFloat, focalError: CGFloat, compared: Int) {
            guard let clip = scrollView?.contentView else { return (.infinity, .infinity, 0) }
            let old = geometry
            let oldScroll = clip.bounds.minY
            finger = CGPoint(x: point.x, y: point.y - oldScroll)
            setFocus(at: point)
            let focal = focalItem
            startSession(to: target)
            guard let session else { return (.infinity, .infinity, 0) }
            setProgress(progress)
            let t = container.transform
            func screen(_ r: CGRect) -> CGRect {
                CGRect(x: t.m11 * r.minX + t.m41, y: t.m22 * r.minY + t.m42 - clip.bounds.minY, width: t.m11 * r.width, height: t.m22 * r.height)
            }
            var worst: CGFloat = 0
            var compared = 0
            for (_, tile) in tiles {
                let r = screen(tile.layer.frame)
                guard r.maxY > 0, r.minY < clip.bounds.height else { continue }
                // The old lattice in screen coordinates: columns at k·pitch, rows at k·pitch − scroll.
                let column = (r.minX / old.pitch).rounded()
                let row = ((r.minY + oldScroll) / old.pitch).rounded()
                worst = max(worst, abs(r.minX - column * old.pitch), abs(r.minY + oldScroll - row * old.pitch), abs(r.width - (old.pitch - old.spacing)))
                compared += 1
            }
            let focalRect = screen(geometry.frame(forItem: focal))
            let focalError = abs(focalRect.minY + focalRect.height * focalUnitY - finger.y)
            revertSession()
            _ = session
            return (worst, focalError, compared)
        }

        /// Tests: each photo's tile painted a colour of its own, so a rendered frame shows where
        /// every photo is.
        static var debugTint: ((Int) -> CGColor)?

        /// Tests: the visible area as it would be drawn at `progress` of a zoom to `target`.
        func debugRender(to target: Int, around point: CGPoint, progress: CGFloat) -> CGImage? {
            guard let clip = scrollView?.contentView, let host = document?.layer else { return nil }
            finger = CGPoint(x: point.x, y: point.y - clip.bounds.minY)
            setFocus(at: point)
            startSession(to: target)
            setProgress(progress)
            let size = clip.bounds.size
            let context = CGContext(data: nil, width: Int(size.width), height: Int(size.height), bitsPerComponent: 8, bytesPerRow: 0,
                                    space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
            context.setFillColor(NSColor.white.cgColor)
            context.fill(CGRect(origin: .zero, size: size))
            // Layer coordinates are top-down here; Core Graphics' are bottom-up.
            context.translateBy(x: 0, y: size.height)
            context.scaleBy(x: 1, y: -1)
            context.translateBy(x: 0, y: -clip.bounds.minY)
            host.render(in: context)
            let image = context.makeImage()
            revertSession()
            return image
        }

        /// Tests: a pinch as a run of cumulative scales (like the trackpad's events), and how many
        /// zooms were started and taken back along the way.
        var debugReverts = 0
        var debugStarts = 0
        func debugPinch(_ scales: [CGFloat], at point: CGPoint) -> [(level: Int, shown: Int, progress: CGFloat)] {
            isPinching = true
            pinchScale = 1
            baselineScale = 1
            finger = CGPoint(x: point.x, y: point.y - scrollY)
            setFocus(at: point)
            var trace: [(Int, Int, CGFloat)] = []
            for scale in scales {
                pinchScale = scale
                pinchChanged(scale: pinchScale / baselineScale, rawScale: pinchScale)
                trace.append((level, shownLevel, session?.progress ?? -1))
            }
            isPinching = false
            if let session { endSession(session) }
            return trace
        }

        var debugDatesReady: Bool { !monthGroups.isEmpty }

        func debugDates() -> [(text: String, y: CGFloat)] {
            updateDates()
            return dates?.debugShown ?? []
        }

        func debugScroll(toY y: CGFloat) { scroll(toY: y); pinnedToBottom = false }
        var debugScrollY: CGFloat { scrollY }
        var debugLevel: Int { level }
        #endif

        // MARK: Dates

        private func computeMonthStarts(_ photos: [Photo]) {
            monthStartsTask?.cancel()
            monthStartsTask = Task { [weak self] in
                let starts = await Task.detached(priority: .utility) { () -> [(item: Int, date: Date)] in
                    PhotoGridDateWording.monthStarts(isoDates: photos.reversed().map(\.takenAt), parse: PhotoDates.parse)
                }.value
                guard let self, !Task.isCancelled else { return }
                self.monthGroups = starts.map { .init(item: $0.item, title: MacPhotoGridDates.monthTitle($0.date)) }
                var lastYear: Int?
                self.yearGroups = starts.compactMap { start in
                    let year = Calendar.current.component(.year, from: start.date)
                    defer { lastYear = year }
                    return year == lastYear ? nil : .init(item: start.item, title: MacPhotoGridDates.yearTitle(start.date))
                }
                self.dateHeaderKey = nil
                self.updateDates()
            }
        }

        private var dateHeaderKey: (range: Range<Int>, columns: Int)?
        private var datesScheduled = false

        /// After the current layout pass: moving views while AppKit lays out the window (a
        /// resize lays the tiles out from inside it) is what starts a constraint-update loop.
        private func updateDatesSoon() {
            guard !datesScheduled else { return }
            datesScheduled = true
            DispatchQueue.main.async { [weak self] in
                self?.datesScheduled = false
                self?.updateDates()
            }
        }

        /// The header up to 9 columns, month labels at 15, year labels at 31; while zooming,
        /// neither the labels (they'd sit over the wrong rows) nor a header change.
        private func updateDates() {
            guard let dates, let clip = scrollView?.contentView, !photos.isEmpty else {
                self.dates?.setHeader(nil, top: 0)
                self.dates?.hideChips()
                return
            }
            let top = clip.contentInsets.top
            let bounds = clip.bounds
            if isZooming {
                dates.hideChips()
                return
            }
            if drawsRows {
                dates.setHeader(nil, top: top)
                let groups = columns >= 31 ? yearGroups : monthGroups
                let geometry = self.geometry
                let visible = geometry.items(fromY: bounds.minY + top - 30, toY: bounds.maxY)
                dates.layout(groups: groups, visible: visible, rowTop: { geometry.frame(forItem: $0).minY - bounds.minY }, pinnedY: top + 10)
                return
            }
            dates.hideChips()
            let visible = geometry.items(fromY: bounds.minY + top, toY: bounds.maxY)
            if let key = dateHeaderKey, key.range == visible, key.columns == columns { return }
            dateHeaderKey = (visible, columns)
            let atNewest = visible.upperBound >= photos.count && (distanceFromBottom() ?? 0) < geometry.pitch
            if atNewest || visible.isEmpty {
                dates.setHeader(PhotoGridDateWording.countText(photos.count), top: top)
                return
            }
            // Items run oldest to newest; some photos have no date, so look inwards for one.
            let oldest = visible.lazy.compactMap { self.photo(at: $0).takenAt.flatMap(PhotoDates.parse) }.first
            let newest = visible.reversed().lazy.compactMap { self.photo(at: $0).takenAt.flatMap(PhotoDates.parse) }.first
            guard let oldest, let newest else {
                dates.setHeader(PhotoGridDateWording.countText(photos.count), top: top)
                return
            }
            dates.setHeader(PhotoGridDateWording.rangeText(from: oldest, to: newest), top: top)
        }

        // MARK: Layout

        private var visibleHeight: CGFloat { scrollView?.contentView.bounds.height ?? 0 }
        private var documentHeight: CGFloat { document?.frame.height ?? 0 }

        private func distanceFromBottom() -> CGFloat? {
            guard let clip = scrollView?.contentView, documentHeight > 0 else { return nil }
            return documentHeight - clip.bounds.maxY
        }

        private func relayout() {
            guard let scrollView, let document else { return }
            // The scroll view tiles only its own subviews: the labels' layer is sized here, or it
            // stays 0×0 and every label below the top is thrown away as off screen.
            if let dates, dates.frame != scrollView.bounds { dates.frame = scrollView.bounds }
            let width = scrollView.contentView.bounds.width
            let scale = scrollView.window?.backingScaleFactor ?? 2
            geometry = PhotoGridGeometry(columns: columns, width: width, itemCount: photos.count, scale: scale)
            let height = max(geometry.contentHeight, scrollView.contentView.bounds.height)
            if document.frame.size != CGSize(width: width, height: height) {
                document.frame = CGRect(x: 0, y: 0, width: width, height: height)
            }
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            container.frame = CGRect(x: 0, y: 0, width: width, height: height)
            CATransaction.commit()
            for (_, tile) in tiles { recycle(tile) }
            tiles.removeAll()
            for (_, tile) in rowTiles { recycle(tile) }
            rowTiles.removeAll()
            placeTiles()
        }

        @objc private func resized() {
            let fromBottom = distanceFromBottom()
            relayout()
            if pinnedToBottom { scrollToNewest() } else if let fromBottom { scroll(toY: documentHeight - visibleHeight - fromBottom) }
        }

        @objc private func scrolled() {
            guard !isZooming else { return placeTiles() }
            if let fromBottom = distanceFromBottom() { pinnedToBottom = fromBottom < 4 }
            placeTiles()
            scheduleRestWork()
        }

        private var restTask: Task<Void, Never>?

        /// Once scrolling stops: the neighbouring levels' rows drawn ahead.
        private func scheduleRestWork() {
            restTask?.cancel()
            restTask = Task { [weak self] in
                try? await Task.sleep(for: .milliseconds(200))
                guard !Task.isCancelled, let self, !self.isZooming else { return }
                self.prepareNeighbourRows()
            }
        }

        private func newTile() -> MacPhotoTile {
            let tile = spare.popLast() ?? MacPhotoTile()
            if tile.layer.superlayer == nil { container.addSublayer(tile.layer) }
            tile.layer.isHidden = false
            return tile
        }

        /// Tiles for what's on screen plus half a screen either way; the rest go back to the pool.
        private func placeTiles() {
            guard let clip = scrollView?.contentView else { return }
            let bounds = clip.bounds
            let margin = bounds.height / 2
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            defer {
                updateSelectionLayer()
                CATransaction.commit()
                updateDatesSoon()
            }
            if drawsRows {
                let rowRange = rowRange(fromY: bounds.minY - margin, toY: bounds.maxY + margin)
                for (row, tile) in rowTiles where !rowRange.contains(row) {
                    recycle(tile)
                    rowTiles[row] = nil
                }
                for row in rowRange {
                    let tile = rowTiles[row] ?? {
                        let tile = newTile()
                        rowTiles[row] = tile
                        return tile
                    }()
                    tile.layer.frame = CGRect(x: 0, y: CGFloat(row) * geometry.pitch, width: geometry.width, height: max(geometry.pitch - geometry.spacing, 0))
                    tile.layer.cornerRadius = 0
                    tile.badge.isHidden = true
                    tile.layer.contentsGravity = .topLeft
                    tile.layer.backgroundColor = nil
                    configureRow(tile, row: row)
                }
                return
            }
            let range = geometry.items(fromY: bounds.minY - margin, toY: bounds.maxY + margin)
            for (item, tile) in tiles where !range.contains(item) {
                recycle(tile)
                tiles[item] = nil
            }
            let pixels = PhotoThumbnailCache.pixelSize(forTile: PhotoGridGeometry.itemSide(width: geometry.width, columns: columns), scale: geometry.scale)
            for item in range {
                let tile = tiles[item] ?? {
                    let tile = newTile()
                    tiles[item] = tile
                    return tile
                }()
                tile.layer.frame = geometry.frame(forItem: item)
                tile.layer.cornerRadius = columns >= 9 ? 0 : 4
                tile.layer.contentsGravity = .resizeAspectFill
                tile.layer.backgroundColor = MacPhotoTile.placeholder
                configure(tile, item: item, pixels: pixels)
                PhotoVideoBadge.update(tile.badge, for: photo(at: item), in: tile.layer.bounds, scale: geometry.scale, show: true)
                #if DEBUG
                if let tint = Self.debugTint { tile.layer.backgroundColor = tint(item); tile.layer.contents = nil }
                #endif
            }
        }

        private func rowRange(fromY minY: CGFloat, toY maxY: CGFloat) -> Range<Int> {
            rowRange(in: geometry, fromY: minY, toY: maxY)
        }

        private func rowRange(in geometry: PhotoGridGeometry, fromY minY: CGFloat, toY maxY: CGFloat) -> Range<Int> {
            guard geometry.pitch > 0, geometry.rowCount > 0 else { return 0..<0 }
            let first = max(Int((minY / geometry.pitch).rounded(.down)), 0)
            let last = min(Int((maxY / geometry.pitch).rounded(.down)), geometry.rowCount - 1)
            return first <= last ? first..<(last + 1) : 0..<0
        }

        private func configureRow(_ tile: MacPhotoTile, row: Int) {
            guard let request = rowRequest(row: row) else { return }
            guard tile.rowKey != request.key || tile.layer.contents == nil else { return }
            tile.rowKey = request.key
            tile.layer.contentsScale = request.scale
            if let image = rows.cachedImage(forKey: request.key) {
                tile.layer.contents = image
                return
            }
            if drawsRowsNow {
                let result = rows.renderNow(request)
                tile.layer.contents = result.image
                fetchMissing(result.missing, forRow: request.key)
                return
            }
            tile.layer.contents = nil
            rows.render(request) { [weak self, weak tile] result in
                guard let self else { return }
                if let tile, tile.rowKey == request.key {
                    CATransaction.begin()
                    CATransaction.setDisableActions(true)
                    tile.layer.contents = result.image
                    CATransaction.commit()
                }
                self.fetchMissing(result.missing, forRow: request.key)
            }
        }

        private func rowRequest(row: Int) -> PhotoGridRowRenderer.Request? {
            rowRequest(row: row, in: geometry)
        }

        private func rowRequest(row: Int, in geometry: PhotoGridGeometry) -> PhotoGridRowRenderer.Request? {
            let columns = geometry.columns
            let lower = max(row * columns - geometry.leadingBlanks, 0)
            let upper = min((row + 1) * columns - geometry.leadingBlanks, photos.count)
            guard lower < upper else { return nil }
            let first = geometry.frame(forItem: lower)
            let top = CGFloat(row) * geometry.pitch
            let cells = (lower..<upper).map { item -> (checksum: String, frame: CGRect) in
                (photo(at: item).checksum, geometry.frame(forItem: item).offsetBy(dx: 0, dy: -top))
            }
            // No sharper than the 128 px thumbnails the rows are drawn from.
            let scale = min(geometry.scale, max(CGFloat(PhotoThumbnailTable.side) / max(first.width, 1), 1))
            return PhotoGridRowRenderer.Request(
                key: "\(columns)-\(Int(geometry.width))-\(photos.count)-\(row)",
                cells: cells,
                size: CGSize(width: geometry.width, height: max(geometry.pitch - geometry.spacing, 1)),
                scale: scale,
                placeholder: MacPhotoTile.placeholder
            )
        }

        /// Thumbnails a row had to leave grey: fetched (in bulk, by the loader), then the rows drawn again.
        private func fetchMissing(_ checksums: [String], forRow key: String) {
            guard !checksums.isEmpty else { return }
            rowsAwaitingThumbnails.insert(key)
            let wanted = Set(checksums)
            for photo in photos where wanted.contains(photo.checksum) {
                _ = loader.request(photo, pixels: PhotoThumbnailTable.side, priority: .visible) { [weak self] _ in
                    self?.scheduleRowRedraw()
                }
            }
        }

        private func scheduleRowRedraw() {
            guard !rowRedrawScheduled else { return }
            rowRedrawScheduled = true
            Task { [weak self] in
                try? await Task.sleep(for: .milliseconds(150))
                guard let self else { return }
                self.rowRedrawScheduled = false
                for key in self.rowsAwaitingThumbnails { self.rows.forget(key) }
                self.rowsAwaitingThumbnails.removeAll()
                guard self.drawsRows else { return }
                for (row, tile) in self.rowTiles {
                    tile.rowKey = nil
                    self.configureRow(tile, row: row)
                }
            }
        }

        private func configure(_ tile: MacPhotoTile, item: Int, pixels: Int) {
            let photo = photo(at: item)
            guard photo.checksum != tile.photo?.checksum || pixels != tile.pixels else { return }
            if let ticket = tile.ticket { loader.release(ticket) }
            tile.ticket = nil
            let samePhoto = photo.checksum == tile.photo?.checksum
            tile.photo = photo
            tile.pixels = pixels
            tile.layer.contentsScale = 1
            if let image = PhotoThumbnailCache.cachedCGImage(forChecksum: photo.checksum, pixels: pixels)
                ?? PhotoThumbnailCache.tableCGImage(forChecksum: photo.checksum, pixels: pixels) {
                tile.layer.contents = image
                return
            }
            if !samePhoto { tile.layer.contents = PhotoThumbnailCache.anyCachedCGImage(forChecksum: photo.checksum) }
            tile.ticket = loader.request(photo, pixels: pixels, priority: .visible) { [weak tile] image in
                guard let tile, tile.photo?.checksum == photo.checksum, tile.pixels == pixels else { return }
                tile.ticket = nil
                if let image {
                    CATransaction.begin()
                    CATransaction.setDisableActions(true)
                    tile.layer.contents = image
                    CATransaction.commit()
                }
            }
        }

        private func recycle(_ tile: MacPhotoTile) {
            if let ticket = tile.ticket { loader.release(ticket) }
            tile.ticket = nil
            tile.photo = nil
            tile.rowKey = nil
            tile.pixels = 0
            tile.layer.contents = nil
            tile.layer.isHidden = true
            tile.badge.isHidden = true
            spare.append(tile)
        }

        private func updateSelectionLayer() {
            guard let item = selectedItem else {
                selectionLayer.isHidden = true
                return
            }
            selectionLayer.frame = geometry.frame(forItem: item).insetBy(dx: -1, dy: -1)
            selectionLayer.cornerRadius = columns >= 9 ? 0 : 5
            selectionLayer.isHidden = false
        }

        // MARK: Scrolling

        func scrollToNewest() {
            pinnedToBottom = true
            scroll(toY: documentHeight - visibleHeight)
        }

        private func scroll(toY y: CGFloat) {
            guard let scrollView else { return }
            let clamped = min(max(y, 0), max(documentHeight - visibleHeight, 0))
            scrollView.contentView.scroll(to: CGPoint(x: 0, y: clamped))
            scrollView.reflectScrolledClipView(scrollView.contentView)
            placeTiles()
        }

        private func reveal(_ item: Int) {
            guard let clip = scrollView?.contentView else { return }
            let frame = geometry.frame(forItem: item).insetBy(dx: 0, dy: -8)
            if frame.minY < clip.bounds.minY { scroll(toY: frame.minY) } else if frame.maxY > clip.bounds.maxY { scroll(toY: frame.maxY - visibleHeight) }
        }

        // MARK: Input

        func item(at point: CGPoint) -> Int? {
            let range = geometry.items(fromY: point.y, toY: point.y)
            return range.first { geometry.frame(forItem: $0).contains(point) }
        }

        func click(at point: CGPoint, count: Int) {
            guard let item = item(at: point) else {
                select(nil)
                return
            }
            select(item)
            // In the dense grids a double click zooms in on the spot, like Photos; photos open
            // from 9 columns or fewer.
            // Like the iPhone's tap: the dense grids zoom in on the spot, photos open from 5 columns.
            if count >= 2 {
                if let target = PhotoGridDensity.tapZoomLevel(fromLevel: level) {
                    zoom(toLevel: target, around: point)
                } else {
                    onOpen(photo(at: item))
                }
            }
        }

        private func select(_ item: Int?) {
            selectedID = item.map { photo(at: $0).id }
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            updateSelectionLayer()
            CATransaction.commit()
        }

        private var selectedItem: Int? { selectedID.flatMap(item(of:)) }

        /// Arrow keys, Space/Return, Delete, ⌘+/⌘−. Returns whether the key was used.
        func key(_ event: NSEvent) -> Bool {
            let command = event.modifierFlags.contains(.command)
            if command, let chars = event.charactersIgnoringModifiers {
                if chars == "+" || chars == "=" { zoom(by: -1); return true }
                if chars == "-" { zoom(by: 1); return true }
            }
            guard !photos.isEmpty else { return false }
            let current = selectedItem
            switch Int(event.keyCode) {
            case 123: move(to: (current ?? photos.count) - 1)       // ←
            case 124: move(to: (current ?? photos.count - 2) + 1)   // →
            case 126: move(to: (current ?? photos.count) - columns) // ↑
            case 125: move(to: (current ?? photos.count - 1 - columns) + columns) // ↓
            case 36, 49, 76: // Return, Space, Enter
                if let current { onOpen(photo(at: current)) }
            case 51, 117: // Delete, Forward Delete
                if let current { confirmDelete(photo(at: current)) }
            default:
                return false
            }
            return true
        }

        private func move(to item: Int) {
            let item = min(max(item, 0), photos.count - 1)
            select(item)
            reveal(item)
        }

        /// Negative zooms in (fewer, bigger tiles).
        private func zoom(by delta: Int) {
            zoom(toLevel: min(max(level + delta, 0), PhotoGridDensity.columnLevels.count - 1))
        }

        func menu(at point: CGPoint) -> NSMenu? {
            guard let item = item(at: point) else { return nil }
            select(item)
            let photo = photo(at: item)
            let menu = NSMenu()
            menu.addItem(ClosureMenuItem(title: "Open", image: "arrow.up.left.and.arrow.down.right") { [weak self] in self?.onOpen(photo) })
            menu.addItem(.separator())
            menu.addItem(ClosureMenuItem(title: "Delete…", image: "trash") { [weak self] in self?.confirmDelete(photo) })
            return menu
        }

        private func confirmDelete(_ photo: Photo) {
            let alert = NSAlert()
            alert.messageText = "Delete this photo?"
            alert.informativeText = "It's deleted from your TPCloud for good."
            alert.addButton(withTitle: "Delete").hasDestructiveAction = true
            alert.addButton(withTitle: "Cancel")
            guard let window = scrollView?.window else { return }
            alert.beginSheetModal(for: window) { [weak self] response in
                guard response == .alertFirstButtonReturn else { return }
                self?.onDelete(photo)
            }
        }
    }
}

/// The grid's canvas: tiles are its layer's sublayers. Flipped, so y grows down like the geometry.
final class MacPhotoGridDocument: NSView {
    weak var coordinator: MacPhotoGrid.Coordinator?

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        layerContentsRedrawPolicy = .never
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }
    override var acceptsFirstResponder: Bool { true }

    override func mouseDown(with event: NSEvent) {
        window?.makeFirstResponder(self)
        coordinator?.click(at: convert(event.locationInWindow, from: nil), count: event.clickCount)
    }

    override func keyDown(with event: NSEvent) {
        if coordinator?.key(event) != true { super.keyDown(with: event) }
    }

    override func magnify(with event: NSEvent) {
        coordinator?.magnify(event, at: convert(event.locationInWindow, from: nil))
    }

    override func menu(for event: NSEvent) -> NSMenu? {
        coordinator?.menu(at: convert(event.locationInWindow, from: nil))
    }
}

/// One tile: a bare layer with a thumbnail, or with a whole row's picture in the dense grids.
@MainActor
final class MacPhotoTile {
    let layer = CALayer()
    var photo: Photo?
    var pixels = 0
    var ticket: PhotoThumbnailLoader.Ticket?
    /// The row picture it shows, in the dense grids.
    var rowKey: String?
    /// A video's length, bottom right.
    let badge = PhotoVideoBadge.make()

    static let placeholder = NSColor.quaternaryLabelColor.cgColor

    init() {
        layer.addSublayer(badge)
        layer.contentsGravity = .resizeAspectFill
        layer.masksToBounds = true
        layer.backgroundColor = Self.placeholder
        let none = NSNull()
        layer.actions = ["contents": none, "position": none, "bounds": none, "hidden": none, "cornerRadius": none,
                         "contentsGravity": none, "backgroundColor": none, "contentsScale": none]
    }
}

/// A menu item that runs a closure.
private final class ClosureMenuItem: NSMenuItem {
    private let handler: () -> Void

    init(title: String, image: String, handler: @escaping () -> Void) {
        self.handler = handler
        super.init(title: title, action: #selector(run), keyEquivalent: "")
        target = self
        self.image = NSImage(systemSymbolName: image, accessibilityDescription: nil)
    }

    @available(*, unavailable)
    required init(coder: NSCoder) { fatalError() }

    @objc private func run() { handler() }
}
#endif
