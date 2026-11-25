import CoreText
import QuartzCore
import TPCore

/// A video's length at the bottom right of its tile («0:42», «1:03:20»), like Photos': white,
/// with a soft shadow so it reads over any picture. A bare text layer, so the grids keep drawing
/// tiles with layers only.
enum PhotoVideoBadge {
    static func make() -> CATextLayer {
        let badge = CATextLayer()
        badge.font = CTFontCreateUIFontForLanguage(.emphasizedSystem, 12, nil)
        badge.fontSize = 12
        badge.alignmentMode = .right
        badge.foregroundColor = CGColor(gray: 1, alpha: 1)
        badge.shadowColor = CGColor(gray: 0, alpha: 1)
        badge.shadowOpacity = 0.55
        badge.shadowRadius = 2
        badge.shadowOffset = .zero
        badge.isHidden = true
        let none = NSNull()
        badge.actions = ["contents": none, "position": none, "bounds": none, "hidden": none, "string": none]
        return badge
    }

    /// Shows `photo`'s length on `badge` (or hides it): `bounds` is the tile's, `scale` the screen's.
    static func update(_ badge: CATextLayer, for photo: Photo?, in bounds: CGRect, scale: CGFloat, show: Bool) {
        guard show, let photo, photo.isVideo, bounds.width >= 44 else {
            if !badge.isHidden { badge.isHidden = true }
            return
        }
        let text = Self.text(photo.duration)
        if badge.string as? String != text { badge.string = text }
        badge.contentsScale = scale
        badge.frame = CGRect(x: 4, y: bounds.height - 19, width: bounds.width - 9, height: 16)
        if badge.isHidden { badge.isHidden = false }
    }

    /// «0:42», «12:05», «1:03:20»; a play mark when the length isn't known yet.
    static func text(_ duration: Double?) -> String {
        guard let duration, duration > 0 else { return "▶︎" }
        let total = Int(duration.rounded())
        let hours = total / 3600, minutes = total / 60 % 60, seconds = total % 60
        return hours > 0 ? String(format: "%d:%02d:%02d", hours, minutes, seconds) : String(format: "%d:%02d", minutes, seconds)
    }
}
