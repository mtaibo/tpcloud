#if os(iOS)
import SwiftUI
import TPCore
import UIKit

/// The iPhone/iPad photo grid (`PhotoGridView`: layer tiles in a scroll view, like Photos').
///
/// Three rounds of tuning a SwiftUI `LazyVStack` still left it hitching, and a `UICollectionView`
/// after it choked on the thousands of cells the dense grids show. Here tiles are bare layers,
/// frames are arithmetic, thumbnails are loaded ahead of the scroll (see `updateLookahead`), and
/// SwiftUI is not involved at all while scrolling.
struct PhotoCollectionGrid: UIViewRepresentable {
    /// Newest first, as the server pages them; shown oldest at the top.
    let photos: [Photo]
    let client: PhotosClient
    @Binding var columnLevel: Int
    let scrollToBottomSignal: Int
    let title: String
    let selection: PhotoSelection?
    let onDelete: (Photo) -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(client: client, level: columnLevel)
    }

    func makeUIView(context: Context) -> PhotoGridContainerView {
        let coordinator = context.coordinator
        let columns = PhotoGridDensity.columns(forLevel: columnLevel)
        let gridView = PhotoGridView(columns: columns)
        gridView.backgroundColor = .clear
        gridView.alwaysBounceVertical = true
        gridView.contentInsetAdjustmentBehavior = .always
        // Breathing room between the newest photos and the tab bar, like Photos, and room at
        // the top for the dates under the title.
        gridView.contentInset.bottom = 20
        gridView.contentInset.top = 30
        gridView.delegate = coordinator
        gridView.tileSource = coordinator
        gridView.accessibilityLabel = "Photos"

        let pinch = UIPinchGestureRecognizer(target: coordinator.zoom, action: #selector(PhotoGridZoomController.handlePinch(_:)))
        pinch.delegate = coordinator
        gridView.addGestureRecognizer(pinch)
        let tap = UITapGestureRecognizer(target: coordinator, action: #selector(Coordinator.handleTap(_:)))
        tap.delegate = coordinator
        gridView.addGestureRecognizer(tap)
        // Select mode: dragging sideways selects a run of photos (the scroll waits to see).
        let selectPan = UIPanGestureRecognizer(target: coordinator, action: #selector(Coordinator.handleSelectPan(_:)))
        selectPan.delegate = coordinator
        selectPan.maximumNumberOfTouches = 1
        gridView.addGestureRecognizer(selectPan)
        gridView.panGestureRecognizer.require(toFail: selectPan)
        coordinator.selectPan = selectPan
        gridView.layer.addSublayer(coordinator.badges.layer)

        coordinator.gridView = gridView
        coordinator.zoom.collectionView = gridView
        gridView.isGridZooming = { [weak zoom = coordinator.zoom] in zoom?.isZooming ?? false }
        coordinator.columns = columns
        coordinator.photos = photos
        coordinator.scrollToBottomSignal = scrollToBottomSignal
        gridView.reloadData(itemCount: photos.count)
        coordinator.computeMonthStartsSoon()
        gridView.accessibilityIdentifier = "PhotoGrid"
        if PhotoGridHitchMonitor.isEnabled {
            coordinator.hitchMonitor = PhotoGridHitchMonitor { [weak gridView, weak zoom = coordinator.zoom] in
                guard let gridView else { return false }
                return gridView.isDragging || gridView.isDecelerating || (zoom?.isZooming ?? false)
            }
            coordinator.hitchMonitor?.offset = { [weak gridView] in gridView?.contentOffset.y ?? 0 }
            coordinator.hitchMonitor?.state = { [weak gridView] in
                guard let gridView else { return "" }
                return (gridView.isTracking ? "T" : "") + (gridView.isDecelerating ? "D" : "")
            }
            coordinator.hitchMonitor?.describeGrid = { [weak gridView, weak zoom = coordinator.zoom] in
                guard let gridView else { return "-" }
                return "y\(Int(gridView.contentOffset.y)),cols\(gridView.columns),zooming\(zoom?.isZooming == true ? 1 : 0),pan\(gridView.panGestureRecognizer.isEnabled ? 1 : 0)"
            }
        }

        let container = PhotoGridContainerView(collectionView: gridView)
        coordinator.dateHeader = container.dateHeader
        coordinator.dateChips = container.dateChips
        container.dateHeader.title = title
        container.statsView = coordinator.hitchMonitor?.statsView
        // Performance tests: zoom through every level and back on its own, so nothing else (a
        // menu closing, the test querying the app) runs alongside the measurement.
        if ProcessInfo.processInfo.arguments.contains("-runZoomCycle") {
            let zoom = coordinator.zoom
            let levels = Array(1..<PhotoGridDensity.columnLevels.count) + Array((0..<PhotoGridDensity.columnLevels.count - 1).reversed())
            for (index, level) in levels.enumerated() {
                DispatchQueue.main.asyncAfter(deadline: .now() + 10 + Double(index) * 1.2) { zoom.zoom(toLevel: level) }
            }
        }
        // Once the grid is up, draw ahead the levels a first pinch would go to.
        DispatchQueue.main.asyncAfter(deadline: .now() + 1) { [weak coordinator] in coordinator?.cameToRest() }
        return container
    }

    func updateUIView(_ container: PhotoGridContainerView, context: Context) {
        let coordinator = context.coordinator
        coordinator.onDelete = onDelete
        coordinator.sync(selection: selection)
        coordinator.dateHeader?.title = title
        coordinator.setColumnLevel = { columnLevel = $0 }
        coordinator.updateRefreshControl(context.environment.refresh)
        coordinator.update(photos: photos)

        // Zoom In/Out from the menu: only a change of `columnLevel` the grid didn't write itself
        // (it saves its level a moment after a pinch, see `levelCommitted`).
        if columnLevel != coordinator.lastColumnLevel {
            coordinator.lastColumnLevel = columnLevel
            if columnLevel != coordinator.zoom.level { coordinator.zoom.zoom(toLevel: columnLevel) }
        }
        if scrollToBottomSignal != coordinator.scrollToBottomSignal {
            coordinator.scrollToBottomSignal = scrollToBottomSignal
            coordinator.scrollToNewest()
        }
    }

    @MainActor
    final class Coordinator: NSObject, UIScrollViewDelegate, UIGestureRecognizerDelegate, PhotoGridTileSource {
        /// Newest first; item `i` is `photos[photos.count - 1 - i]`.
        var photos: [Photo] = []
        var columns: Int
        var scrollToBottomSignal = 0
        var onDelete: (Photo) -> Void = { _ in }
        var setColumnLevel: (Int) -> Void = { _ in }
        /// The `columnLevel` last seen from (or written to) SwiftUI.
        var lastColumnLevel: Int
        private var saveLevelTask: Task<Void, Never>?

        /// Saves the level once the zoom has fully settled. Writing it to `@AppStorage` re-renders
        /// the whole gallery in SwiftUI (bars, menu, glass), and doing that on the frame a level
        /// was reached — every level of a long pinch — showed up as a dropped frame each time.
        private func levelCommitted(_ level: Int) {
            saveLevelTask?.cancel()
            saveLevelTask = Task { [weak self] in
                try? await Task.sleep(for: .milliseconds(400))
                guard let self, !Task.isCancelled else { return }
                if self.zoom.isZooming {
                    self.levelCommitted(self.zoom.level)
                    return
                }
                self.lastColumnLevel = self.zoom.level
                self.setColumnLevel(self.zoom.level)
            }
        }
        weak var gridView: PhotoGridView?

        // MARK: Select mode

        private(set) var selection: PhotoSelection?
        let badges = PhotoGridSelectionBadges()
        weak var selectPan: UIPanGestureRecognizer?
        private var selectionActive = false
        private var selectedIDs: Set<Int> = []
        private var showSelectedSignal = 0
        /// The drag: where it began, whether it selects or deselects, and the selection before it.
        private var dragAnchor: Int?
        private var dragSelects = true
        private var dragBase: Set<Int> = []
        private var dragLastItem: Int?
        private var dragPoint: CGPoint = .zero
        private var autoScrollLink: CADisplayLink?

        /// Brings the grid in line with SwiftUI's selection (called from `updateUIView`, which
        /// SwiftUI reruns whenever what it read here changes).
        func sync(selection: PhotoSelection?) {
            self.selection = selection
            let active = selection?.isActive ?? false
            let ids = selection?.ids ?? []
            let signal = selection?.showSelectedSignal ?? 0
            if active != selectionActive || ids != selectedIDs {
                selectionActive = active
                selectedIDs = ids
                updateBadges()
            }
            if signal != showSelectedSignal {
                showSelectedSignal = signal
                showSelected()
            }
        }

        private func updateBadges() {
            guard let gridView, selectionActive, !selectedIDs.isEmpty, !photos.isEmpty else {
                badges.clear()
                return
            }
            let geometry = gridView.geometry
            let visible = geometry.items(fromY: gridView.bounds.minY, toY: gridView.bounds.maxY)
            badges.update(
                visible: visible,
                isSelected: { [selectedIDs, photos] item in selectedIDs.contains(photos[photos.count - 1 - item].id) },
                frame: { geometry.frame(forItem: $0) }
            )
        }

        private func toggleSelection(_ item: Int) {
            guard let selection, item < photos.count else { return }
            let id = photo(at: item).id
            if selection.ids.contains(id) { selection.ids.remove(id) } else { selection.ids.insert(id) }
        }

        private func showSelected() {
            let chosen = photos.reversed().filter { selectedIDs.contains($0.id) }
            guard !chosen.isEmpty else { return }
            presentViewer(photos: chosen, index: 0)
        }

        @objc func handleSelectPan(_ pan: UIPanGestureRecognizer) {
            guard let gridView, let selection else { return }
            dragPoint = pan.location(in: gridView.superview)
            switch pan.state {
            case .began:
                guard let item = nearestItem(toViewPoint: dragPoint) else { return }
                dragAnchor = item
                dragBase = selection.ids
                dragSelects = !selection.ids.contains(photo(at: item).id)
                dragLastItem = nil
                extendDrag()
                startAutoScroll()
            case .changed:
                extendDrag()
            default:
                dragAnchor = nil
                stopAutoScroll()
            }
        }

        /// Selects (or deselects) every photo between where the drag began and the finger, in
        /// reading order, on top of what was selected before the drag.
        private func extendDrag() {
            guard let selection, let anchor = dragAnchor, let item = nearestItem(toViewPoint: dragPoint),
                  item != dragLastItem else { return }
            dragLastItem = item
            let run = Set((min(anchor, item)...max(anchor, item)).map { photo(at: $0).id })
            selection.ids = dragSelects ? dragBase.union(run) : dragBase.subtracting(run)
        }

        /// The item under a point of the container (the nearest one in a gap or past the ends).
        private func nearestItem(toViewPoint point: CGPoint) -> Int? {
            guard let gridView, !photos.isEmpty else { return nil }
            let geometry = gridView.geometry
            guard geometry.pitch > 0 else { return nil }
            let content = CGPoint(x: point.x + gridView.bounds.minX, y: point.y + gridView.bounds.minY)
            let column = min(max(Int(content.x / geometry.pitch), 0), geometry.columns - 1)
            let row = min(max(Int(content.y / geometry.pitch), 0), max(geometry.rowCount - 1, 0))
            let item = row * geometry.columns + column - geometry.leadingBlanks
            return min(max(item, 0), photos.count - 1)
        }

        /// While the finger is near the top or bottom edge, the grid scrolls on its own, faster the
        /// closer it gets, and the run follows.
        private func startAutoScroll() {
            guard autoScrollLink == nil else { return }
            let link = CADisplayLink(target: self, selector: #selector(autoScrollStep(_:)))
            link.add(to: .main, forMode: .common)
            autoScrollLink = link
        }

        private func stopAutoScroll() {
            autoScrollLink?.invalidate()
            autoScrollLink = nil
        }

        @objc private func autoScrollStep(_ link: CADisplayLink) {
            guard let gridView, dragAnchor != nil else { return }
            let insets = gridView.adjustedContentInset
            let top = insets.top + 40
            let bottom = gridView.bounds.height - insets.bottom - 70
            let zone: CGFloat = 70
            var speed: CGFloat = 0
            if dragPoint.y < top + zone { speed = -min((top + zone - dragPoint.y) / zone, 1.5) }
            else if dragPoint.y > bottom - zone { speed = min((dragPoint.y - bottom + zone) / zone, 1.5) }
            guard speed != 0 else { return }
            let step = speed * 1100 * CGFloat(link.targetTimestamp - link.timestamp)
            let minY = -insets.top
            let maxY = max(gridView.contentSize.height + insets.bottom - gridView.bounds.height, minY)
            gridView.contentOffset.y = min(max(gridView.contentOffset.y + step, minY), maxY)
            extendDrag()
        }
        weak var dateHeader: PhotoGridDateHeader? {
            didSet { updateDateHeader() }
        }
        private var dateHeaderRange: Range<Int>?
        weak var dateChips: PhotoGridDateChips?
        /// Where each month begins (display order), worked out off the main thread per photo list.
        private var monthStarts: [(item: Int, date: Date)] = []
        private var monthGroups: [PhotoGridDateChips.Group] = []
        private var yearGroups: [PhotoGridDateChips.Group] = []
        private var monthStartsTask: Task<Void, Never>?

        func computeMonthStartsSoon() {
            DispatchQueue.main.async { [weak self] in self?.computeMonthStarts() }
        }

        private func computeMonthStarts() {
            monthStartsTask?.cancel()
            let photos = self.photos
            monthStartsTask = Task { [weak self] in
                let starts = await Task.detached(priority: .utility) { () -> [(item: Int, date: Date)] in
                    let dates = photos.reversed().map { $0.takenAt.flatMap(PhotoDates.parse) }
                    return PhotoGridDateChips.monthStarts(dates)
                }.value
                guard let self, !Task.isCancelled else { return }
                self.monthStarts = starts
                self.monthGroups = starts.map { .init(item: $0.item, title: PhotoGridDateChips.monthTitle($0.date)) }
                var lastYear: Int?
                self.yearGroups = starts.compactMap { start in
                    let year = Calendar.current.component(.year, from: start.date)
                    defer { lastYear = year }
                    return year == lastYear ? nil : .init(item: start.item, title: PhotoGridDateChips.yearTitle(start.date))
                }
                self.updateDateChips()
            }
        }

        /// Month labels at 15 columns, year labels at 31, none otherwise (or while zooming).
        func updateDateChips() {
            guard let gridView, let dateChips else { return }
            let groups = zoom.isZooming ? [] : columns >= 31 ? yearGroups : columns >= 15 ? monthGroups : []
            let bounds = gridView.bounds
            let insets = gridView.adjustedContentInset
            let visible = gridView.geometry.items(fromY: bounds.minY + insets.top - 30, toY: bounds.maxY)
            let geometry = gridView.geometry
            dateChips.layout(
                groups: groups,
                visible: visible,
                rowTop: { geometry.frame(forItem: $0).minY - bounds.minY },
                pinnedY: gridView.safeAreaInsets.top + 22
            )
        }
        private var dateHeaderColumns = 0
        var hitchMonitor: PhotoGridHitchMonitor?

        private let client: PhotosClient
        private let loader: PhotoThumbnailLoader
        /// Loads for tiles just ahead of the scroll, by item.
        private var lookahead: [Int: PhotoThumbnailLoader.Ticket] = [:]
        private var lookaheadKey: (bucket: Int, direction: Int)?
        private var lastLookaheadY: CGFloat = 0
        let zoom: PhotoGridZoomController
        /// A sync that landed mid-zoom; reloading would pull the items out from under it.
        private var pendingPhotos: [Photo]?
        private var refreshAction: RefreshAction?
        private var refreshControl: UIRefreshControl?

        init(client: PhotosClient, level: Int) {
            self.client = client
            loader = PhotoThumbnailLoader(client: client)
            columns = PhotoGridDensity.columns(forLevel: level)
            lastColumnLevel = level
            zoom = PhotoGridZoomController(level: level)
            super.init()
            zoom.onLevelCommitted = { [weak self] level in self?.levelCommitted(level) }
            zoom.levelWillInstall = { [weak self] level in self?.prepare(forLevel: level) }
            zoom.levelDidInstall = { [weak self] in
                self?.refreshVisibleTiles()
                self?.updateBadges()
            }
            zoom.onZoomEnded = { [weak self] in
                self?.cameToRest()
                guard let self, let pending = self.pendingPhotos else { return }
                self.pendingPhotos = nil
                self.update(photos: pending)
            }
        }

        private func photo(at item: Int) -> Photo {
            photos[photos.count - 1 - item]
        }

        private var cornerRadius: CGFloat {
            columns >= 9 ? 0 : 4
        }

        /// While flinging, tiles show only the table's ready-made 128 px copy, like Photos: no
        /// decoding for tiles that are gone a frame later. Sharp ones load once the scroll slows.
        private var isFlinging = false
        private var lastScroll: (y: CGFloat, time: CFTimeInterval)?
        private var scrollSpeed: CGFloat = 0

        /// While flinging or zooming, the table's ready-made copy only; sharp thumbnails load once
        /// the grid settles (`endFling`, `cameToRest`).
        private var pixels: Int {
            let pixels = restingPixels
            return isFlinging || zoom.isZooming ? min(pixels, PhotoThumbnailTable.side) : pixels
        }

        private var restingPixels: Int {
            guard let gridView, gridView.bounds.width > 0 else { return PhotoThumbnailCache.pixelSize }
            let insets = gridView.adjustedContentInset
            let side = PhotoGridGeometry.itemSide(width: gridView.bounds.width - insets.left - insets.right, columns: columns)
            return PhotoThumbnailCache.pixelSize(forTile: side, scale: gridView.traitCollection.displayScale)
        }

        // MARK: Data

        /// Reloads for a new photo list (once per sync), keeping the photos on screen where they
        /// are: rows are cut from the newest end, so holding the distance from the bottom fixed
        /// means older photos landing above don't move anything.
        func update(photos newPhotos: [Photo]) {
            guard !Self.sameArray(newPhotos, photos), newPhotos != photos, let gridView else { return }
            guard !zoom.isZooming else {
                pendingPhotos = newPhotos
                return
            }
            let distanceFromBottom = gridView.contentSize.height - gridView.contentOffset.y
            photos = newPhotos
            releaseLookahead()
            gridView.reloadData(itemCount: newPhotos.count)
            dateHeaderRange = nil
            computeMonthStarts()
            defer { updateDateHeader() }
            guard !gridView.isPinnedToBottom else { return }
            let insets = gridView.adjustedContentInset
            let maxY = max(gridView.contentSize.height + insets.bottom - gridView.bounds.height, -insets.top)
            gridView.contentOffset.y = min(max(gridView.contentSize.height - distanceFromBottom, -insets.top), maxY)
        }

        /// `updateUIView` runs on every re-render of the gallery (e.g. each backup progress tick);
        /// when the store hasn't published a new list it's the same buffer, so skip comparing
        /// thousands of photos field by field.
        private static func sameArray(_ lhs: [Photo], _ rhs: [Photo]) -> Bool {
            lhs.count == rhs.count && lhs.withUnsafeBufferPointer { l in
                rhs.withUnsafeBufferPointer { r in l.baseAddress == r.baseAddress }
            }
        }

        // MARK: Tiles

        func gridView(_ gridView: PhotoGridView, configure tile: PhotoTile, forItem item: Int) {
            configure(tile, item: item, pixels: pixels)
            // The tile holds its own claim now.
            if let ticket = lookahead.removeValue(forKey: item) { loader.release(ticket) }
        }

        // MARK: Row pictures (15 and 31 columns)

        private let rows = PhotoGridRowRenderer()
        /// Row pictures drawn with grey squares, waiting for those thumbnails.
        private var rowsAwaitingThumbnails: Set<String> = []
        private var rowRedrawScheduled = false

        func gridView(_ gridView: PhotoGridView, configureRow tile: PhotoTile, row: Int) {
            guard let request = rowRequest(row: row, geometry: gridView.geometry) else { return }
            tile.photo = nil
            tile.rowKey = request.key
            if let image = rows.cachedImage(forKey: request.key) {
                tile.layer.contents = image
                return
            }
            tile.layer.contents = nil
            rows.render(request) { [weak self, weak tile] result in
                guard let self else { return }
                if let tile, tile.rowKey == request.key { tile.layer.contents = result.image }
                self.fetchMissing(result.missing, forRow: request.key)
            }
        }

        /// What to draw for a row of `geometry` (the grid's own, or another level's ahead of a zoom).
        private func rowRequest(row: Int, geometry: PhotoGridGeometry) -> PhotoGridRowRenderer.Request? {
            let columns = geometry.columns
            let lower = max(row * columns - geometry.leadingBlanks, 0)
            let upper = min((row + 1) * columns - geometry.leadingBlanks, photos.count)
            guard lower < upper, geometry.itemCount == photos.count else { return nil }
            let first = geometry.frame(forItem: lower)
            let top = first.minY
            let cells = (lower..<upper).map { item -> (checksum: String, frame: CGRect) in
                (photo(at: item).checksum, geometry.frame(forItem: item).offsetBy(dx: 0, dy: -top))
            }
            let placeholder = UIColor.tertiarySystemFill.resolvedColor(with: gridView?.traitCollection ?? .current).cgColor
            // No sharper than the 128 px thumbnails it's drawn from: at 15 columns a 3× row
            // would be three and a half times the bytes to draw and upload for nothing.
            let scale = min(geometry.scale, max(CGFloat(PhotoThumbnailTable.side) / max(first.width, 1), 1))
            return PhotoGridRowRenderer.Request(
                key: "\(columns)-\(Int(geometry.width))-\(photos.count)-\(row)",
                cells: cells,
                size: CGSize(width: geometry.width, height: first.height),
                scale: scale,
                placeholder: placeholder
            )
        }

        /// Thumbnails a row picture had to leave grey: load them, then draw the row again.
        private func fetchMissing(_ checksums: [String], forRow key: String) {
            guard !checksums.isEmpty else { return }
            rowsAwaitingThumbnails.insert(key)
            let wanted = Set(checksums)
            for photo in photos where wanted.contains(photo.checksum) {
                let ticket = loader.request(photo, pixels: PhotoThumbnailTable.side, priority: .visible) { [weak self] _ in
                    self?.scheduleRowRedraw()
                }
                // Kept alive by the loader until delivered; nothing to cancel per row.
                _ = ticket
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
                guard let gridView = self.gridView, gridView.drawsRows else { return }
                for tile in gridView.placedRowTiles {
                    self.gridView(gridView, configureRow: tile, row: tile.item)
                }
            }
        }

        /// Draws ahead the rows of `columns` around what's on screen now, so a zoom to that level
        /// (or a scroll at it) finds them ready.
        private func prepareRows(columns: Int, screens: CGFloat) {
            guard let gridView, columns >= PhotoGridView.rowPictureColumns, !photos.isEmpty else { return }
            let target = gridView.geometry(columns: columns)
            let current = gridView.geometry
            guard target.pitch > 0, current.pitch > 0 else { return }
            let visible = current.items(fromY: gridView.bounds.minY, toY: gridView.bounds.maxY)
            guard !visible.isEmpty else { return }
            let middle = target.frame(forItem: visible.lowerBound + visible.count / 2)
            let span = gridView.bounds.height * screens
            let range = target.items(fromY: middle.midY - span, toY: middle.midY + span)
            guard !range.isEmpty else { return }
            let firstRow = (range.lowerBound + target.leadingBlanks) / columns
            let lastRow = (range.upperBound - 1 + target.leadingBlanks) / columns
            for row in firstRow...lastRow {
                if let request = rowRequest(row: row, geometry: target) { rows.render(request) }
            }
        }

        /// At rest: the levels a pinch would go to next.
        private func prepareNeighbourLevels() {
            let level = zoom.level
            for neighbour in [level - 1, level + 1] where PhotoGridDensity.columnLevels.indices.contains(neighbour) {
                prepareRows(columns: PhotoGridDensity.columns(forLevel: neighbour), screens: 1.5)
            }
            prepareRows(columns: columns, screens: 1.5)
        }

        func gridView(_ gridView: PhotoGridView, didRecycle tile: PhotoTile) {
            tile.rowKey = nil
            if let ticket = tile.ticket { loader.release(ticket) }
            tile.ticket = nil
            tile.photo = nil
            tile.pixels = 0
        }

        /// Draws from the memory cache (or the table) on the same frame when it can; otherwise
        /// shows whatever size is at hand — so neither a big tile nor a zoom step is ever blank —
        /// until the right one lands.
        private func configure(_ tile: PhotoTile, item: Int, pixels: Int) {
            guard item < photos.count else { return }
            let photo = photo(at: item)
            if tile.layer.cornerRadius != cornerRadius { tile.layer.cornerRadius = cornerRadius }
            let samePhoto = photo.checksum == tile.photo?.checksum
            guard !samePhoto || pixels != tile.pixels else { return }
            if let ticket = tile.ticket { loader.release(ticket) }
            tile.ticket = nil
            tile.photo = photo
            tile.pixels = pixels

            if let cached = PhotoThumbnailCache.cachedCGImage(forChecksum: photo.checksum, pixels: pixels)
                ?? PhotoThumbnailCache.tableCGImage(forChecksum: photo.checksum, pixels: pixels) {
                tile.layer.contents = cached
                return
            }
            if !samePhoto {
                tile.layer.contents = PhotoThumbnailCache.anyCachedCGImage(forChecksum: photo.checksum)
            }
            tile.ticket = loader.request(photo, pixels: pixels, priority: .visible) { [weak tile] image in
                guard let tile, tile.photo?.checksum == photo.checksum, tile.pixels == pixels else { return }
                tile.ticket = nil
                if let image { tile.layer.contents = image }
            }
        }

        func gridView(_ gridView: PhotoGridView, accessibilityLabelForItem item: Int) -> String {
            item < photos.count ? PhotoGrid.accessibilityLabel(for: photo(at: item)) : ""
        }

        func gridView(_ gridView: PhotoGridView, activateItem item: Int) {
            guard item < photos.count else { return }
            select(item)
        }

        @objc func handleTap(_ tap: UITapGestureRecognizer) {
            guard let gridView, !tapStoppedScroll, let item = gridView.item(at: tap.location(in: gridView)) else { return }
            select(item)
        }

        private func select(_ item: Int) {
            guard let gridView, !zoom.isZooming else { return }
            if selectionActive {
                toggleSelection(item)
                return
            }
            // In the dense grids a tap closes in on the photo (31 → 15 → 5 columns) instead of
            // opening it; photos open from 5 columns or fewer.
            if let target = PhotoGridDensity.tapZoomLevel(fromLevel: zoom.level) {
                let frame = gridView.geometry.frame(forItem: item)
                zoom.zoom(toLevel: target, around: CGPoint(x: frame.midX, y: frame.midY))
            } else {
                openViewer(at: item)
            }
        }

        /// A tap that lands while the grid is moving just stops it, as in Photos.
        private var tapStoppedScroll = false

        nonisolated func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldReceive touch: UITouch) -> Bool {
            MainActor.assumeIsolated {
                if gestureRecognizer is UITapGestureRecognizer, let gridView {
                    tapStoppedScroll = gridView.isDecelerating || gridView.isDragging
                }
            }
            return true
        }

        // MARK: Viewer

        /// Opens the photo with the system zoom transition, which grows it out of its tile and,
        /// when closed (or swiped down), shrinks it back into the tile of whichever photo the
        /// viewer is on by then.
        private func openViewer(at item: Int) {
            guard !isOpeningViewer else { return }
            // Like Photos, open on the sharp photo: start decoding its preview (on the phone
            // already if the grid warmed it) and give it a moment before presenting.
            hitchMonitor?.photoTapped()
            let photo = photo(at: item)
            let loader = PhotoViewerImageLoader.shared
            loader.prepare(photo, client: client)
            isOpeningViewer = true
            Task { [weak self] in
                await loader.waitForPreview(photo, timeout: .milliseconds(180))
                guard let self else { return }
                self.isOpeningViewer = false
                self.presentViewer(at: item)
            }
        }

        private var isOpeningViewer = false

        private func presentViewer(at item: Int) {
            presentViewer(photos: photos.reversed(), index: item)
        }

        /// The viewer on `photos` (oldest first), starting at `index`: the whole library, or just
        /// the selected photos.
        private func presentViewer(photos viewerPhotos: [Photo], index: Int) {
            guard let gridView, let presenter = gridView.owningViewController,
                  presenter.presentedViewController == nil else { return }
            let viewer = PhotoViewerController(photos: viewerPhotos, index: index, client: client) { [weak self] photo in
                self?.onDelete(photo)
            }
            let navigation = PhotoViewerNavigationController(rootViewController: viewer)
            let options = UIViewController.Transition.ZoomOptions()
            options.interactiveDismissShouldBegin = { [weak viewer] _ in viewer?.allowsInteractiveDismiss ?? true }
            options.alignmentRectProvider = { [weak viewer] context in
                viewer?.currentPhotoFrame(in: context.zoomedViewController.view) ?? context.zoomedViewController.view.bounds
            }
            navigation.preferredTransition = .zoom(options: options) { [weak self, weak viewer] _ in
                guard let self, let photo = viewer?.currentPhoto else { return nil }
                return self.transitionView(for: photo)
            }
            viewer.onDismissed = { [weak self] in self?.removeTransitionView() }
            hitchMonitor?.presentStarted()
            presenter.present(navigation, animated: true) { [weak self] in self?.removeTransitionView() }
        }

        /// The zoom transition grows from (and shrinks back into) a view; tiles are layers, so
        /// a stand-in view with the tile's picture is put over the tile for it.
        private var transitionView: UIView?

        private func transitionView(for photo: Photo) -> UIView? {
            removeTransitionView()
            guard let gridView, let position = photos.firstIndex(where: { $0.id == photo.id }) else { return nil }
            let item = photos.count - 1 - position
            if gridView.tile(forItem: item) == nil { gridView.scrollToItem(item) }
            guard let tile = gridView.tile(forItem: item) else { return nil }
            let view = UIView(frame: tile.layer.frame)
            view.isUserInteractionEnabled = false
            view.layer.contents = tile.layer.contents
            view.layer.contentsGravity = .resizeAspectFill
            view.layer.cornerRadius = tile.layer.cornerRadius
            view.clipsToBounds = true
            gridView.addSubview(view)
            transitionView = view
            return view
        }

        private func removeTransitionView() {
            transitionView?.removeFromSuperview()
            transitionView = nil
        }

        // MARK: Look-ahead

        /// Loads the thumbnails about a screen and a half ahead in the direction of the scroll, at
        /// lower priority than the tiles on screen, so a fling lands on tiles that are already
        /// decoded. UIKit's own prefetching looks only a row or two ahead, which a fast fling
        /// outruns. Recomputed every quarter screen, not every frame.
        /// Follows the scroll's speed (points per second, smoothed) and switches flinging on above
        /// about three screens a second; `endFling` turns it off when the grid stops.
        private func updateSpeed(in grid: PhotoGridView) {
            let now = CACurrentMediaTime()
            let y = grid.contentOffset.y
            defer { lastScroll = (y, now) }
            guard let last = lastScroll, now > last.time, !zoom.isZooming else { return }
            let instant = abs(y - last.y) / CGFloat(now - last.time)
            scrollSpeed = scrollSpeed * 0.6 + instant * 0.4
            let height = max(grid.bounds.height, 1)
            // Only ever switched off when the scroll comes to rest: flinging one way and straight
            // back passes through zero speed, and sharpening every tile then had their pictures
            // land in the middle of the next fling, which hitched.
            if !isFlinging, scrollSpeed > height * 3 {
                isFlinging = true
            }
        }

        private func endFling() {
            guard isFlinging else { return }
            isFlinging = false
            scrollSpeed = 0
            lookaheadKey = nil
            refreshVisibleTiles()
        }

        private func updateLookahead(in grid: PhotoGridView) {
            let y = grid.contentOffset.y
            let delta = y - lastLookaheadY
            lastLookaheadY = y
            guard delta != 0, !zoom.isZooming, grid.bounds.height > 0 else { return }
            let height = grid.bounds.height
            let direction = delta > 0 ? 1 : -1
            let key = (bucket: Int((y / (height / 4)).rounded(.down)), direction: direction)
            guard lookaheadKey?.bucket != key.bucket || lookaheadKey?.direction != key.direction else { return }
            lookaheadKey = key

            let bounds = grid.bounds
            let window = direction > 0
                ? CGRect(x: bounds.minX, y: bounds.maxY, width: bounds.width, height: height * 1.5)
                : CGRect(x: bounds.minX, y: bounds.minY - height * 1.5, width: bounds.width, height: height * 1.5)
            // Dense grids: draw the row pictures coming up.
            if grid.drawsRows {
                for row in grid.rows(fromY: window.minY, toY: window.maxY) {
                    if let request = rowRequest(row: row, geometry: grid.geometry) { rows.render(request) }
                }
                return
            }

            let items = Set(grid.geometry.items(fromY: window.minY, toY: window.maxY))

            // Flinging: no decoding ahead, just make sure the table copies are in memory.
            if isFlinging {
                releaseLookahead()
                lookaheadKey = key
                PhotoThumbnailTable.shared.prefault(items.filter { $0 < photos.count }.map { photo(at: $0).checksum })
                return
            }

            for (item, ticket) in lookahead where !items.contains(item) {
                loader.release(ticket)
                lookahead[item] = nil
            }
            let pixels = self.pixels
            for item in items where lookahead[item] == nil && item < photos.count {
                let photo = photo(at: item)
                guard PhotoThumbnailCache.cachedCGImage(forChecksum: photo.checksum, pixels: pixels) == nil,
                      pixels > PhotoThumbnailTable.side || !PhotoThumbnailTable.shared.contains(photo.checksum) else { continue }
                lookahead[item] = loader.request(photo, pixels: pixels, priority: .lookahead) { _ in }
            }
        }

        private func releaseLookahead() {
            for ticket in lookahead.values { loader.release(ticket) }
            lookahead.removeAll()
            lookaheadKey = nil
        }

        // MARK: Date header

        /// The count at the newest photos, the dates on screen anywhere else. Recomputed only
        /// when the rows on screen change.
        func updateDateHeader() {
            guard let gridView, let dateHeader, !photos.isEmpty else {
                dateHeader?.text = nil
                return
            }
            let insets = gridView.adjustedContentInset
            let bounds = gridView.bounds
            let visible = gridView.geometry.items(fromY: bounds.minY + insets.top, toY: bounds.maxY - insets.bottom)
            guard visible != dateHeaderRange || columns != dateHeaderColumns else { return }
            dateHeaderColumns = columns
            dateHeaderRange = visible
            // Dense grids show the dates as labels in the grid instead (`updateDateChips`).
            if columns >= PhotoGridView.rowPictureColumns {
                dateHeader.text = nil
                return
            }
            let atNewest = visible.upperBound >= photos.count && gridView.contentOffset.y >= gridView.contentSize.height + insets.bottom - bounds.height - gridView.geometry.pitch
            if atNewest || visible.isEmpty {
                dateHeader.text = PhotoGridDateHeader.countText(photos.count)
                return
            }
            // Items run oldest to newest; some photos have no date, so look inwards for one.
            let dates = visible.lazy.compactMap { self.photo(at: $0).takenAt.flatMap(PhotoDates.parse) }
            let newestDates = visible.reversed().lazy.compactMap { self.photo(at: $0).takenAt.flatMap(PhotoDates.parse) }
            guard let oldest = dates.first, let newest = newestDates.first else {
                dateHeader.text = PhotoGridDateHeader.countText(photos.count)
                return
            }
            dateHeader.text = PhotoGridDateHeader.rangeText(from: oldest, to: newest)
        }

        func scrollViewDidScroll(_ scrollView: UIScrollView) {
            guard let grid = scrollView as? PhotoGridView else { return }
            updateDateHeader()
            updateDateChips()
            if selectionActive { updateBadges() }
            updateSpeed(in: grid)
            updateLookahead(in: grid)
        }

        func scrollViewWillBeginDragging(_ scrollView: UIScrollView) {
            gridView?.isPinnedToBottom = false
            PhotoViewerImageLoader.shared.cancelWarming()
        }

        func scrollViewDidEndDragging(_ scrollView: UIScrollView, willDecelerate decelerate: Bool) {
            if !decelerate {
                endFling()
                gridView?.scrollCameToRest()
                cameToRest()
            }
        }

        func scrollViewDidEndDecelerating(_ scrollView: UIScrollView) {
            endFling()
            gridView?.scrollCameToRest()
            cameToRest()
        }

        /// The grid stopped (after a scroll or a zoom): sharpen what's on screen, then — a moment
        /// later, so it never lands on the last frames of an animation — get ready for what's
        /// likely next.
        func cameToRest() {
            dateHeaderRange = nil
            updateDateHeader()
            updateDateChips()
            updateBadges()
            refreshVisibleTiles()
            restTask?.cancel()
            restTask = Task { [weak self] in
                try? await Task.sleep(for: .milliseconds(250))
                guard let self, !Task.isCancelled, !self.zoom.isZooming,
                      let gridView = self.gridView, !gridView.isDragging, !gridView.isDecelerating else { return }
                self.warmVisiblePreviews()
                self.prepareNeighbourLevels()
            }
        }

        private var restTask: Task<Void, Never>?

        /// Once the grid stops on large tiles (where photos open from), fetch the previews of the
        /// photos on screen to disk, nearest the middle first, so a tap opens one from the phone.
        private func warmVisiblePreviews() {
            guard let gridView, columns <= 5, !zoom.isZooming else { return }
            let middle = CGPoint(x: gridView.bounds.midX, y: gridView.bounds.midY)
            let items = gridView.placedTiles
                .map { tile -> (item: Int, distance: CGFloat) in
                    let frame = tile.layer.frame
                    return (tile.item, hypot(frame.midX - middle.x, frame.midY - middle.y))
                }
                .sorted { $0.distance < $1.distance }
                .prefix(30)
            PhotoViewerImageLoader.shared.warm(items.map { photo(at: $0.item) }, client: client)
        }

        // MARK: Zoom

        // Two fingers that start moving before the pinch is recognised already belong to the
        // scroll view's pan; without this the pinch would never begin.
        nonisolated func gestureRecognizer(
            _ gestureRecognizer: UIGestureRecognizer,
            shouldRecognizeSimultaneouslyWith otherGestureRecognizer: UIGestureRecognizer
        ) -> Bool {
            MainActor.assumeIsolated {
                // The selection drag owns its touch: no scrolling under it.
                gestureRecognizer !== selectPan && otherGestureRecognizer !== selectPan
            }
        }

        /// The selection drag only in select mode, and only when the finger sets off sideways;
        /// up and down is a scroll.
        nonisolated func gestureRecognizerShouldBegin(_ gestureRecognizer: UIGestureRecognizer) -> Bool {
            MainActor.assumeIsolated {
                guard gestureRecognizer === selectPan, let pan = gestureRecognizer as? UIPanGestureRecognizer else { return true }
                guard selectionActive, !zoom.isZooming else { return false }
                let velocity = pan.velocity(in: pan.view)
                return abs(velocity.x) > abs(velocity.y)
            }
        }

        /// A level's grid is about to go in: cells made from now on get its thumbnail size and corners.
        private func prepare(forLevel level: Int) {
            columns = PhotoGridDensity.columns(forLevel: level)
            releaseLookahead()
            updateDateChips()
        }

        /// Tiles on screen switch to the current corner radius and thumbnail size (keeping the
        /// image they have until the new size is ready).
        private func refreshVisibleTiles() {
            guard let gridView else { return }
            if gridView.drawsRows {
                for tile in gridView.placedRowTiles { self.gridView(gridView, configureRow: tile, row: tile.item) }
                return
            }
            let pixels = self.pixels
            for tile in gridView.placedTiles {
                configure(tile, item: tile.item, pixels: pixels)
            }
        }

        // MARK: Scrolling

        func scrollToNewest() {
            guard let gridView, !zoom.isZooming else { return }
            gridView.scrollToBottom(animated: true)
        }

        /// Re-tapping the Photos tab (and tapping the status bar) asks the scroll view to go to
        /// the top — the oldest photos here. Racing that with a delayed scroll back down used to
        /// lose now and then, sending the grid all the way up and then back down. The system
        /// scroll is refused instead, and the grid goes straight to where it starts: the newest
        /// photos at the bottom.
        func scrollViewShouldScrollToTop(_ scrollView: UIScrollView) -> Bool {
            scrollToNewest()
            return false
        }

        func updateRefreshControl(_ action: RefreshAction?) {
            refreshAction = action
            guard let gridView else { return }
            if action != nil, refreshControl == nil {
                let control = UIRefreshControl()
                control.addTarget(self, action: #selector(handleRefresh(_:)), for: .valueChanged)
                gridView.refreshControl = control
                refreshControl = control
            } else if action == nil, refreshControl != nil {
                gridView.refreshControl = nil
                refreshControl = nil
            }
        }

        @objc private func handleRefresh(_ control: UIRefreshControl) {
            guard let refreshAction else {
                control.endRefreshing()
                return
            }
            Task {
                await refreshAction()
                control.endRefreshing()
            }
        }
    }
}

extension UIView {
    /// The view controller this view belongs to, to present from.
    var owningViewController: UIViewController? {
        var responder: UIResponder? = next
        while let current = responder {
            if let controller = current as? UIViewController { return controller }
            responder = current.next
        }
        return nil
    }
}

#endif
