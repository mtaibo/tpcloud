#if os(iOS)
import UIKit

/// The blue check on selected photos, like Photos': one small layer per selected photo on
/// screen, laid over the tiles (or the dense grids' row pictures) in the grid's own coordinates,
/// so it scrolls and zooms with them.
final class PhotoGridSelectionBadges {
    let layer = CALayer()
    private var badges: [Int: CALayer] = [:]
    private var spare: [CALayer] = []

    init() {
        layer.actions = ["sublayers": NSNull(), "position": NSNull(), "bounds": NSNull(), "hidden": NSNull()]
        layer.zPosition = 10
    }

    /// Shows a check on every item in `visible` that `isSelected`, at the bottom right of its frame.
    func update(visible: Range<Int>, isSelected: (Int) -> Bool, frame: (Int) -> CGRect) {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        var keep: Set<Int> = []
        for item in visible where isSelected(item) {
            keep.insert(item)
            let tile = frame(item)
            let side = min(24, max(tile.width * 0.36, 10))
            let inset = tile.width > 60 ? 5 : 2
            let badge = badges[item] ?? spare.popLast() ?? makeBadge()
            badge.frame = CGRect(x: tile.maxX - side - CGFloat(inset), y: tile.maxY - side - CGFloat(inset), width: side, height: side)
            badge.isHidden = false
            badges[item] = badge
        }
        for (item, badge) in badges where !keep.contains(item) {
            badge.isHidden = true
            spare.append(badge)
            badges[item] = nil
        }
        CATransaction.commit()
    }

    func clear() {
        update(visible: 0..<0, isSelected: { _ in false }, frame: { _ in .zero })
    }

    private func makeBadge() -> CALayer {
        let badge = CALayer()
        badge.contents = Self.image
        badge.contentsGravity = .resizeAspect
        badge.actions = ["position": NSNull(), "bounds": NSNull(), "hidden": NSNull(), "contents": NSNull()]
        badge.shadowColor = UIColor.black.cgColor
        badge.shadowOpacity = 0.25
        badge.shadowRadius = 2
        badge.shadowOffset = .zero
        layer.addSublayer(badge)
        return badge
    }

    /// A white ring around a blue disc with a white check.
    private static let image: CGImage? = {
        let size = CGSize(width: 48, height: 48)
        let format = UIGraphicsImageRendererFormat()
        format.scale = 3
        return UIGraphicsImageRenderer(size: size, format: format).image { context in
            UIColor.white.setFill()
            context.cgContext.fillEllipse(in: CGRect(origin: .zero, size: size))
            UIColor.systemBlue.setFill()
            context.cgContext.fillEllipse(in: CGRect(x: 4, y: 4, width: 40, height: 40))
            let check = UIImage(systemName: "checkmark", withConfiguration: UIImage.SymbolConfiguration(pointSize: 20, weight: .bold))?
                .withTintColor(.white, renderingMode: .alwaysOriginal)
            if let check {
                let origin = CGPoint(x: (size.width - check.size.width) / 2, y: (size.height - check.size.height) / 2)
                check.draw(at: origin)
            }
        }.cgImage
    }()
}
#endif
