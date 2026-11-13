#if os(iOS)
import TPCore
import UIKit

/// One tile of the grid: a bare `CALayer` showing the thumbnail, nothing else.
@MainActor
final class PhotoTile {
    let layer = CALayer()
    fileprivate(set) var item = -1
    var photo: Photo?
    var pixels = 0
    var ticket: PhotoThumbnailLoader.Ticket?
    /// The row picture it shows, in the dense grids.
    var rowKey: String?

    init() {
        layer.contentsGravity = .resizeAspectFill
        layer.masksToBounds = true
        layer.backgroundColor = UIColor.tertiarySystemFill.cgColor
        // Tiles move, fill in and get reused without any implicit animation.
        let none = NSNull()
        layer.actions = ["contents": none, "position": none, "bounds": none, "hidden": none, "cornerRadius": none, "backgroundColor": none]
    }
}

@MainActor
protocol PhotoGridTileSource: AnyObject {
    /// A tile has just been placed for `item`: give it its picture.
    func gridView(_ gridView: PhotoGridView, configure tile: PhotoTile, forItem item: Int)
    /// In the dense grids, a whole row is one tile: give it the row's picture.
    func gridView(_ gridView: PhotoGridView, configureRow tile: PhotoTile, row: Int)
    /// A tile has left the screen and is about to be reused: drop what it was loading.
    func gridView(_ gridView: PhotoGridView, didRecycle tile: PhotoTile)
    /// What VoiceOver reads for a tile.
    func gridView(_ gridView: PhotoGridView, accessibilityLabelForItem item: Int) -> String
    /// VoiceOver (or a test) activated a tile.
    func gridView(_ gridView: PhotoGridView, activateItem item: Int)
}

/// A tile as VoiceOver sees it: tiles are layers, which accessibility can't reach on its own.
private final class PhotoTileAccessibilityElement: UIAccessibilityElement {
    let item: Int
    weak var gridView: PhotoGridView?

    init(item: Int, gridView: PhotoGridView) {
        self.item = item
        self.gridView = gridView
        super.init(accessibilityContainer: gridView)
        accessibilityTraits = [.image, .button]
    }

    override var accessibilityLabel: String? {
        get { gridView.flatMap { $0.tileSource?.gridView($0, accessibilityLabelForItem: item) } }
        set {}
    }

    override var accessibilityFrameInContainerSpace: CGRect {
        get { gridView?.geometry.frame(forItem: item) ?? .zero }
        set {}
    }

    override func accessibilityActivate() -> Bool {
        guard let gridView else { return false }
        gridView.tileSource?.gridView(gridView, activateItem: item)
        return true
    }
}

/// The iPhone/iPad photo grid, built the way the Photos app's is: a scroll view whose tiles are
/// plain layers, one per photo, placed by arithmetic (`PhotoGridGeometry`) and recycled a row at a
/// time as they scroll in and out.
///
/// It replaced a `UICollectionView`, which is fine for a few dozen cells but not for the 1,000–
/// 2,000 the 15- and 31-column grids put on screen: on device, flinging spent most of every frame
/// inside the collection view itself (walking its subviews to reuse cells, giving each new cell
/// its safe area, two views per cell to lay out and commit), and the grid stuttered however
/// cheap the thumbnails were. Here a frame that scrolls in a new row touches just that row's
/// layers.
///
/// In the dense grids (15 and 31 columns) each row is a single layer showing a picture of the
/// whole row, drawn in the background from the thumbnail table — the way Photos does it. Two
/// thousand separate layers at 31 columns made Core Animation walk them all on every frame of a
/// fling and every step of a zoom; as rows it's twenty-odd.
///
/// Opens on the newest photos at the bottom, and stays there — through the bars' insets settling
/// and the first sync landing — until the user first scrolls.
final class PhotoGridView: UIScrollView {
    weak var tileSource: PhotoGridTileSource?
    var isPinnedToBottom = true
    /// A zoom owns the offset while it runs.
    var isGridZooming: () -> Bool = { false }

    private(set) var columns: Int
    private(set) var itemCount = 0
    private(set) var geometry: PhotoGridGeometry

    private var tiles: [Int: PhotoTile] = [:]
    private var placedRange: Range<Int> = 0..<0
    /// Row tiles, by row, in the dense grids.
    private var rowTiles: [Int: PhotoTile] = [:]
    private var placedRows: Range<Int> = 0..<0

    /// Column counts from which rows are drawn as one picture.
    static let rowPictureColumns = 15
    var drawsRows: Bool { columns >= Self.rowPictureColumns }
    private var spareTiles: [PhotoTile] = []

    private var lastInsets: UIEdgeInsets?
    /// An inset change that landed mid-scroll, to settle once the scroll comes to rest.
    private var pendingBottomFix: CGFloat?

    init(columns: Int) {
        self.columns = columns
        geometry = PhotoGridGeometry(columns: columns, width: 0, itemCount: 0, scale: 2)
        super.init(frame: .zero)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    /// Performance tests only: who moves the grid by thousands of points at once.
    static var jumpLog: ((String) -> Void)?
    /// How big a single move has to be to be logged (smaller in select mode, where any jump is
    /// a bug: nothing but the finger should move the grid there).
    var jumpThreshold: CGFloat = 5000

    override var contentOffset: CGPoint {
        didSet {
            guard let jumpLog = Self.jumpLog, abs(contentOffset.y - oldValue.y) > jumpThreshold else { return }
            let stack = Thread.callStackSymbols.dropFirst(1).prefix(14).map { line in
                line.split(separator: " ", omittingEmptySubsequences: true).dropFirst(3).prefix(1).joined()
            }
            jumpLog("\(Int(oldValue.y))>\(Int(contentOffset.y))[\(stack.joined(separator: "<"))]")
        }
    }

    // MARK: Geometry

    /// The geometry `columns` would have here right now.
    func geometry(columns: Int) -> PhotoGridGeometry {
        let insets = adjustedContentInset
        return PhotoGridGeometry(
            columns: columns,
            width: bounds.width - insets.left - insets.right,
            itemCount: itemCount,
            scale: traitCollection.displayScale
        )
    }

    private func updateGeometry() {
        geometry = geometry(columns: columns)
        let size = CGSize(width: geometry.width, height: geometry.contentHeight)
        if contentSize != size { contentSize = size }
    }

    /// The item under a point in content coordinates, if the point is on a tile.
    func item(at point: CGPoint) -> Int? {
        let pitch = geometry.pitch
        guard pitch > 0, point.x >= 0, point.y >= 0 else { return nil }
        let column = Int(point.x / pitch)
        let row = Int(point.y / pitch)
        guard column < geometry.columns else { return nil }
        let item = row * geometry.columns + column - geometry.leadingBlanks
        guard item >= 0, item < itemCount, geometry.frame(forItem: item).contains(point) else { return nil }
        return item
    }

    func tile(forItem item: Int) -> PhotoTile? {
        tiles[item]
    }

    var placedTiles: Dictionary<Int, PhotoTile>.Values {
        tiles.values
    }

    // MARK: Accessibility

    private var cachedAccessibilityElements: (range: Range<Int>, elements: [Any])?

    /// Dense grids (15 and 31 columns, up to 2,000 tiles) are one element, which zooms in when
    /// activated, like a tap; reading thousands of tiles one by one helps nobody.
    private var isDense: Bool { geometry.columns > 9 }

    override var isAccessibilityElement: Bool {
        get { isDense }
        set {}
    }

    override func accessibilityActivate() -> Bool {
        let visible = geometry.items(fromY: bounds.minY, toY: bounds.maxY)
        guard isDense, !visible.isEmpty else { return false }
        let middle = visible.lowerBound + visible.count / 2
        tileSource?.gridView(self, activateItem: middle)
        return true
    }

    /// One element per tile on screen, built only when accessibility asks.
    override var accessibilityElements: [Any]? {
        get {
            guard !isDense else { return nil }
            if let cached = cachedAccessibilityElements, cached.range == placedRange { return cached.elements }
            let elements: [Any] = placedRange.map { PhotoTileAccessibilityElement(item: $0, gridView: self) }
            cachedAccessibilityElements = (placedRange, elements)
            return elements
        }
        set {}
    }

    // MARK: Changes

    /// A new photo list: every tile is placed again.
    func reloadData(itemCount: Int) {
        self.itemCount = itemCount
        recycleAll()
        updateGeometry()
        setNeedsLayout()
    }

    /// Switches to another column count, scrolled to `offset`, and places its tiles at once.
    func setColumns(_ columns: Int, offset: CGPoint) {
        self.columns = columns
        recycleAll()
        updateGeometry()
        contentOffset = offset
        placeTiles()
    }

    // MARK: Tiles

    override func layoutSubviews() {
        super.layoutSubviews()
        let current = geometry(columns: columns)
        if current.width != geometry.width || current.itemCount != geometry.itemCount || current.scale != geometry.scale {
            if jumpThreshold < 5000 {
                let i = adjustedContentInset
                Self.jumpLog?("geom:w\(geometry.width)>\(current.width),n\(geometry.itemCount)>\(current.itemCount),bw\(bounds.width),il\(i.left),ir\(i.right)")
            }
            recycleAll()
            updateGeometry()
        }
        if isPinnedToBottom, bounds.height > 0, contentSize.height > 0 {
            let insets = adjustedContentInset
            let bottom = max(contentSize.height + insets.bottom - bounds.height, -insets.top)
            if contentOffset.y != bottom { contentOffset.y = bottom }
        }
        placeTiles()
    }

    /// Places the rows that came on screen and recycles the ones that left.
    private func placeTiles() {
        guard geometry.width > 0 else { return }
        if drawsRows {
            placeRowTiles()
            return
        }
        let range = geometry.items(fromY: bounds.minY, toY: bounds.maxY)
        guard range != placedRange else { return }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for item in placedRange where !range.contains(item) {
            if let tile = tiles.removeValue(forKey: item) { recycle(tile) }
        }
        for item in range where tiles[item] == nil {
            let tile = spareTiles.popLast() ?? newTile()
            if tile.layer.superlayer == nil { layer.insertSublayer(tile.layer, at: 0) }
            tile.layer.backgroundColor = UIColor.tertiarySystemFill.cgColor
            tile.layer.contentsGravity = .resizeAspectFill
            tile.item = item
            tile.layer.frame = geometry.frame(forItem: item)
            tile.layer.isHidden = false
            tiles[item] = tile
            tileSource?.gridView(self, configure: tile, forItem: item)
        }
        placedRange = range
        trimSpareTiles()
        CATransaction.commit()
    }

    /// Spare tiles stay in the layer tree (hidden) to be reused without re-inserting them, but
    /// only a screenful's worth: after the 31-column grid, two thousand hidden layers left
    /// behind made Core Animation walk them all on every frame.
    private func trimSpareTiles() {
        let keep = max(placedRange.count / 2, placedRows.count, 64)
        guard spareTiles.count > keep else { return }
        for tile in spareTiles[keep...] { tile.layer.removeFromSuperlayer() }
        spareTiles.removeSubrange(keep...)
    }

    /// The rows overlapping `minY...maxY`.
    func rows(fromY minY: CGFloat, toY maxY: CGFloat) -> Range<Int> {
        let pitch = geometry.pitch
        guard pitch > 0, geometry.rowCount > 0 else { return 0..<0 }
        let first = max(Int((minY / pitch).rounded(.down)), 0)
        let last = min(Int((maxY / pitch).rounded(.down)), geometry.rowCount - 1)
        return first <= last ? first..<(last + 1) : 0..<0
    }

    /// The items in a row (the oldest row can be partial).
    func items(inRow row: Int) -> Range<Int> {
        let columns = geometry.columns
        let lower = max(row * columns - geometry.leadingBlanks, 0)
        let upper = min((row + 1) * columns - geometry.leadingBlanks, itemCount)
        return lower < upper ? lower..<upper : 0..<0
    }

    func frame(forRow row: Int) -> CGRect {
        let first = geometry.frame(forItem: max(row * geometry.columns - geometry.leadingBlanks, 0))
        return CGRect(x: 0, y: first.minY, width: geometry.width, height: first.height)
    }

    var placedRowTiles: Dictionary<Int, PhotoTile>.Values {
        rowTiles.values
    }

    private func placeRowTiles() {
        let range = rows(fromY: bounds.minY, toY: bounds.maxY)
        guard range != placedRows else { return }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for row in placedRows where !range.contains(row) {
            if let tile = rowTiles.removeValue(forKey: row) { recycle(tile) }
        }
        for row in range where rowTiles[row] == nil {
            let tile = spareTiles.popLast() ?? newTile()
            if tile.layer.superlayer == nil { layer.insertSublayer(tile.layer, at: 0) }
            tile.item = row
            tile.layer.frame = frame(forRow: row)
            tile.layer.cornerRadius = 0
            // The gaps between photos are part of the picture.
            tile.layer.backgroundColor = nil
            tile.layer.contentsGravity = .resize
            tile.layer.isHidden = false
            rowTiles[row] = tile
            tileSource?.gridView(self, configureRow: tile, row: row)
        }
        placedRows = range
        trimSpareTiles()
        CATransaction.commit()
    }

    // MARK: Zoom hand-off

    /// Takes the tiles on screen out of the grid into a layer of their own, laid out in the
    /// view's coordinates, for a zoom to scale and fade out while the next level is placed in
    /// the grid. Moving layers is free, where snapshotting the grid cost a render of the whole
    /// screen on the first frame of every zoom.
    func detachPlacedTiles() -> (layer: CALayer, tiles: [PhotoTile]) {
        let holder = CALayer()
        let none = NSNull()
        holder.actions = ["transform": none, "position": none, "bounds": none, "sublayers": none, "mask": none, "opacity": none]
        holder.frame = frame
        let detached = Array(tiles.values) + Array(rowTiles.values)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for tile in detached {
            tile.layer.removeFromSuperlayer()
            tile.layer.position = CGPoint(x: tile.layer.position.x - bounds.minX, y: tile.layer.position.y - bounds.minY)
            holder.addSublayer(tile.layer)
        }
        CATransaction.commit()
        tiles.removeAll(keepingCapacity: true)
        rowTiles.removeAll(keepingCapacity: true)
        placedRange = 0..<0
        placedRows = 0..<0
        return (holder, detached)
    }

    /// Tiles a zoom has finished with go back to be reused.
    func recycleDetached(_ detached: [PhotoTile]) {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for tile in detached {
            tile.layer.removeFromSuperlayer()
            recycle(tile)
        }
        trimSpareTiles()
        CATransaction.commit()
    }

    private func newTile() -> PhotoTile {
        let tile = PhotoTile()
        // Under the scroll indicators.
        layer.insertSublayer(tile.layer, at: 0)
        return tile
    }

    private func recycle(_ tile: PhotoTile) {
        tileSource?.gridView(self, didRecycle: tile)
        tile.item = -1
        tile.layer.isHidden = true
        tile.layer.contents = nil
        spareTiles.append(tile)
    }

    private func recycleAll() {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for tile in tiles.values { recycle(tile) }
        for tile in rowTiles.values { recycle(tile) }
        CATransaction.commit()
        tiles.removeAll(keepingCapacity: true)
        rowTiles.removeAll(keepingCapacity: true)
        placedRange = 0..<0
        placedRows = 0..<0
    }

    // MARK: Scrolling

    private func maxOffsetY(for insets: UIEdgeInsets) -> CGFloat {
        max(contentSize.height + insets.bottom - bounds.height, -insets.top)
    }

    /// The tab bar can change the bottom inset as it minimizes and restores. UIKit keeps the
    /// offset where it was, so a grid sitting on its newest photos either slid behind the bar or
    /// left a gap under the last row — until switching tabs re-laid it out. Keep it on its
    /// bottom edge instead, and never leave blank space below the last row.
    override func adjustedContentInsetDidChange() {
        super.adjustedContentInsetDidChange()
        superview?.setNeedsLayout()
        let insets = adjustedContentInset
        defer { lastInsets = insets }
        if jumpThreshold < 5000, let old = lastInsets, old != insets {
            Self.jumpLog?("insets:t\(Int(old.top))>\(Int(insets.top)),b\(Int(old.bottom))>\(Int(insets.bottom)),l\(Int(old.left))>\(Int(insets.left)),r\(Int(old.right))>\(Int(insets.right))")
        }
        guard let old = lastInsets, old != insets, contentSize.height > 0, bounds.height > 0 else { return }
        let wasAtBottom = contentOffset.y >= maxOffsetY(for: old) - 1
        if isTracking || isDecelerating || isGridZooming() {
            if wasAtBottom { pendingBottomFix = abs(insets.bottom - old.bottom) + 1 }
            return
        }
        let maxY = maxOffsetY(for: insets)
        if wasAtBottom || contentOffset.y > maxY { contentOffset.y = maxY }
    }

    /// After a drag or fling ends: applies an inset change that happened during it.
    func scrollCameToRest() {
        defer { pendingBottomFix = nil }
        // Never while a finger is on the grid: an animated scroll then fights the new drag.
        guard !isGridZooming(), !isTracking, !isDragging, contentSize.height > 0 else { return }
        let maxY = maxOffsetY(for: adjustedContentInset)
        let tolerance = pendingBottomFix ?? 0.5
        if contentOffset.y > maxY || (pendingBottomFix != nil && abs(contentOffset.y - maxY) <= tolerance) {
            setContentOffset(CGPoint(x: contentOffset.x, y: maxY), animated: true)
        }
    }

    func scrollToBottom(animated: Bool) {
        setContentOffset(CGPoint(x: contentOffset.x, y: maxOffsetY(for: adjustedContentInset)), animated: animated)
    }

    /// Scrolls (without animating) so `item` is in the middle of the screen, and places its tile.
    func scrollToItem(_ item: Int) {
        guard item >= 0, item < itemCount else { return }
        isPinnedToBottom = false
        let insets = adjustedContentInset
        let frame = geometry.frame(forItem: item)
        let y = frame.midY - (bounds.height - insets.top - insets.bottom) / 2 - insets.top
        contentOffset.y = min(max(y, -insets.top), maxOffsetY(for: insets))
        placeTiles()
    }
}
#endif
