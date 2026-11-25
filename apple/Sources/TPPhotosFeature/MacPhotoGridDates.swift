#if os(macOS)
import AppKit
import TPCore

/// Over the Mac's grid, like the iPhone's (`PhotoGridDateHeader`, `PhotoGridDateChips`) and the web's:
/// - up to 9 columns, a dark capsule at the top left saying where you are: the dates on screen
///   («4–7 Nov 2026»), or how many photos there are at the newest end;
/// - at 15 columns a capsule with the month («Aug 2026») at the row where each month begins, at
///   31 one with the year; the one you're in stays pinned at the top until the next pushes it out.
/// AppKit only, so moving it with the scroll never touches SwiftUI. Clicks go through to the grid.
final class MacPhotoGridDates: NSView {
    struct Group {
        /// The first (oldest) item of the month or year, in display order.
        let item: Int
        let title: NSAttributedString
    }

    private let header = Capsule()
    private var chips: [Capsule] = []
    private static let height: CGFloat = 30
    private static let gap: CGFloat = 6

    override init(frame: NSRect) {
        super.init(frame: frame)
        addSubview(header)
        header.isHidden = true
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }
    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    /// The header's text (nil hides it). `top`: where the visible area starts below the toolbar.
    func setHeader(_ text: String?, top: CGFloat) {
        guard let text else {
            header.isHidden = true
            return
        }
        header.set(NSAttributedString(string: text, attributes: [
            .font: NSFont.systemFont(ofSize: 15, weight: .semibold),
            .foregroundColor: NSColor.white,
        ]))
        header.setFrameOrigin(NSPoint(x: 14, y: top + 10))
        header.isHidden = false
    }

    /// Places the labels for the groups on screen. `rowTop` gives an item's row top in this view's
    /// coordinates; `pinnedY` is where the current group's label stays.
    func layout(groups: [Group], visible: Range<Int>, rowTop: (Int) -> CGFloat, pinnedY: CGFloat) {
        guard !groups.isEmpty, !visible.isEmpty else {
            chips.forEach { $0.isHidden = true }
            return
        }
        let firstOnScreen = groups.firstIndex { $0.item >= visible.lowerBound } ?? groups.count
        let current = max(firstOnScreen - 1, 0)
        let lastOnScreen = groups.lastIndex { $0.item < visible.upperBound } ?? current
        var placed: [(group: Group, y: CGFloat)] = []
        for index in current...max(current, lastOnScreen) {
            let group = groups[index]
            var y = rowTop(group.item) + 6
            if index == current, y < pinnedY { y = pinnedY }
            placed.append((group, y))
        }
        // The pinned one gives way to the next as it arrives.
        if placed.count > 1, placed[0].y + Self.height + Self.gap > placed[1].y, placed[0].y == pinnedY {
            placed[0].y = placed[1].y - Self.height - Self.gap
        }
        var used = 0
        var lastBottom = -CGFloat.infinity
        for (group, y) in placed where y > -Self.height && y < bounds.height {
            // A group too short to fit its label under the previous one goes without.
            guard y >= lastBottom + Self.gap || used == 0 else { continue }
            let chip = used < chips.count ? chips[used] : makeChip()
            chip.set(group.title)
            chip.setFrameOrigin(NSPoint(x: 12, y: y))
            // Pushed up out of its place, it fades away instead of covering the toolbar.
            chip.alphaValue = y < pinnedY ? max(0, 1 - (pinnedY - y) / (Self.height * 0.8)) : 1
            chip.isHidden = chip.alphaValue == 0
            lastBottom = y + Self.height
            used += 1
        }
        for chip in chips[used...] { chip.isHidden = true }
    }

    #if DEBUG
    /// Tests: the labels showing, top to bottom, with where they are.
    var debugShown: [(text: String, y: CGFloat)] {
        let header = self.header.isHidden ? [] : [(self.header.debugText, self.header.frame.minY)]
        return header + chips.filter { !$0.isHidden }.map { ($0.debugText, $0.frame.minY) }.sorted { $0.1 < $1.1 }
    }
    #endif

    func hideChips() {
        chips.forEach { $0.isHidden = true }
    }

    private func makeChip() -> Capsule {
        let chip = Capsule()
        addSubview(chip)
        chips.append(chip)
        return chip
    }

    // MARK: Titles

    /// «Aug 2026»: the month in bold, the year in the regular weight.
    static func monthTitle(_ date: Date) -> NSAttributedString {
        let text = NSMutableAttributedString(
            string: PhotoGridDateWording.month.string(from: date),
            attributes: [.font: NSFont.systemFont(ofSize: 14, weight: .bold), .foregroundColor: NSColor.white]
        )
        text.append(NSAttributedString(
            string: " " + PhotoGridDateWording.year.string(from: date),
            attributes: [.font: NSFont.systemFont(ofSize: 14, weight: .regular), .foregroundColor: NSColor.white.withAlphaComponent(0.9)]
        ))
        return text
    }

    static func yearTitle(_ date: Date) -> NSAttributedString {
        NSAttributedString(
            string: PhotoGridDateWording.year.string(from: date),
            attributes: [.font: NSFont.systemFont(ofSize: 14, weight: .bold), .foregroundColor: NSColor.white]
        )
    }

    /// A dark capsule with a line of white text, sized to it. Laid out by hand: with
    /// constraints inside, every move of a label asked the SwiftUI view around the grid to update
    /// its constraints mid-layout, and AppKit gave up on the loop by throwing.
    final class Capsule: NSView {
        private let label = NSTextField(labelWithString: "")
        private let box = FlippedBox()
        private var shown: NSAttributedString?

        init() {
            super.init(frame: .zero)
            // Dark like the iPhone's and the web's labels: they read over any photo.
            wantsLayer = true
            layer?.backgroundColor = NSColor.black.withAlphaComponent(0.62).cgColor
            layer?.cornerRadius = MacPhotoGridDates.height / 2
            layer?.cornerCurve = .continuous
            label.lineBreakMode = .byClipping
            box.addSubview(label)
            addSubview(box)
        }

        @available(*, unavailable)
        required init?(coder: NSCoder) { fatalError() }

        override func hitTest(_ point: NSPoint) -> NSView? { nil }

        var debugText: String { shown?.string ?? "" }

        func set(_ text: NSAttributedString) {
            guard text != shown else { return }
            shown = text
            label.attributedStringValue = text
            let size = label.attributedStringValue.size()
            let labelSize = NSSize(width: ceil(size.width) + 4, height: ceil(size.height))
            let width = labelSize.width + 22
            setFrameSize(NSSize(width: width, height: MacPhotoGridDates.height))
            box.frame = bounds
            label.frame = NSRect(x: (width - labelSize.width) / 2, y: (MacPhotoGridDates.height - labelSize.height) / 2,
                                 width: labelSize.width, height: labelSize.height)
        }
    }

    private final class FlippedBox: NSView {
        override var isFlipped: Bool { true }
    }
}
#endif
