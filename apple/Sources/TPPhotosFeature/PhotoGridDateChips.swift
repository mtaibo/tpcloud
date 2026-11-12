#if os(iOS)
import TPCore
import UIKit

/// The month (15 columns) or year (31 columns) labels of the dense grids, like Photos': a dark
/// capsule at the left edge of the row where each month or year begins, scrolling with it, and the
/// one you're in pinned under the title until the next one pushes it out.
final class PhotoGridDateChips: UIView {
    struct Group {
        /// The first (oldest) item of the month or year, in display order.
        let item: Int
        let title: NSAttributedString
    }

    private var chips: [UIView] = []
    private static let height: CGFloat = 32
    private static let gap: CGFloat = 6

    override init(frame: CGRect) {
        super.init(frame: frame)
        isUserInteractionEnabled = false
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    /// Places the labels for the groups on screen. `rowTop` gives an item's row top in this
    /// view's coordinates; `pinnedY` is where the current group's label stays.
    func layout(groups: [Group], visible: Range<Int>, rowTop: (Int) -> CGFloat, pinnedY: CGFloat) {
        guard !groups.isEmpty, !visible.isEmpty else {
            chips.forEach { $0.isHidden = true }
            return
        }
        // The group the top of the screen is in, then every group starting on screen.
        let firstOnScreen = groups.firstIndex { $0.item >= visible.lowerBound } ?? groups.count
        let current = max(firstOnScreen - 1, 0)
        let lastOnScreen = groups.lastIndex { $0.item < visible.upperBound } ?? current
        var placed: [(group: Group, y: CGFloat)] = []
        for index in current...max(current, lastOnScreen) {
            let group = groups[index]
            var y = rowTop(group.item) + 4
            if index == current, y < pinnedY { y = pinnedY }
            placed.append((group, y))
        }
        // The pinned one gives way to the next as it arrives.
        if placed.count > 1, placed[0].y + Self.height + Self.gap > placed[1].y, placed[0].y == pinnedY {
            placed[0].y = placed[1].y - Self.height - Self.gap
        }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        var used = 0
        var lastBottom = -CGFloat.infinity
        for (group, y) in placed where y > -Self.height && y < bounds.height {
            // A group too short to fit its label under the previous one goes without.
            guard y >= lastBottom + Self.gap || used == 0 else { continue }
            let chip = used < chips.count ? chips[used] : makeChip()
            let label = chip.subviews.first as! UILabel
            if label.attributedText != group.title { label.attributedText = group.title }
            let width = ceil(label.intrinsicContentSize.width) + 24
            chip.frame = CGRect(x: 12, y: y, width: width, height: Self.height)
            label.frame = chip.bounds
            // Pushed up out of its place under the title, it fades away instead of covering it.
            chip.alpha = y < pinnedY ? max(0, 1 - (pinnedY - y) / (Self.height * 0.8)) : 1
            chip.isHidden = chip.alpha == 0
            lastBottom = y + Self.height
            used += 1
        }
        for chip in chips[used...] { chip.isHidden = true }
        CATransaction.commit()
    }

    private func makeChip() -> UIView {
        let chip = UIView()
        chip.backgroundColor = UIColor.black.withAlphaComponent(0.62)
        chip.layer.cornerRadius = Self.height / 2
        chip.layer.cornerCurve = .continuous
        let label = UILabel()
        label.textAlignment = .center
        chip.addSubview(label)
        addSubview(chip)
        chips.append(chip)
        return chip
    }

    // MARK: Groups

    /// "Aug 2026": the month in bold, the year in the regular weight.
    static func monthTitle(_ date: Date) -> NSAttributedString {
        let text = NSMutableAttributedString(
            string: monthFormatter.string(from: date),
            attributes: [.font: UIFont.systemFont(ofSize: 17, weight: .bold), .foregroundColor: UIColor.white]
        )
        text.append(NSAttributedString(
            string: " " + yearFormatter.string(from: date),
            attributes: [.font: UIFont.systemFont(ofSize: 17, weight: .regular), .foregroundColor: UIColor.white.withAlphaComponent(0.9)]
        ))
        return text
    }

    static func yearTitle(_ date: Date) -> NSAttributedString {
        NSAttributedString(
            string: yearFormatter.string(from: date),
            attributes: [.font: UIFont.systemFont(ofSize: 17, weight: .bold), .foregroundColor: UIColor.white]
        )
    }

    private static let monthFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.setLocalizedDateFormatFromTemplate("MMM")
        return formatter
    }()

    private static let yearFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.setLocalizedDateFormatFromTemplate("y")
        return formatter
    }()

    /// Where each month begins, oldest first: `dates` in display order (oldest first, nil for a
    /// photo with no date, which joins the month before it).
    nonisolated static func monthStarts(_ dates: [Date?]) -> [(item: Int, date: Date)] {
        let calendar = Calendar.current
        var starts: [(item: Int, date: Date)] = []
        var last: (Int, Int)?
        for (item, date) in dates.enumerated() {
            guard let date else { continue }
            let parts = calendar.dateComponents([.year, .month], from: date)
            let key = (parts.year ?? 0, parts.month ?? 0)
            if last.map({ $0 != key }) ?? true {
                starts.append((item, date))
                last = key
            }
        }
        return starts
    }
}
#endif
