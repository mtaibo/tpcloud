#if os(iOS)
import TPCore
import UIKit

/// The top of the grid, like Photos': the section's title, large, and under it the date — how many
/// photos there are at the newest end ("12,809 Items"), anywhere else the dates on screen
/// ("4–7 Nov 2026"). The dense grids leave it out: their dates are labels in the grid
/// (`PhotoGridDateChips`). Behind it the photos darken and blur, less and less further down, so the white
/// text reads over anything. UIKit, so updating it while scrolling never touches SwiftUI.
final class PhotoGridDateHeader: UIView {
    private let titleLabel = UILabel()
    private let dateLabel = UILabel()
    private let blur = UIVisualEffectView(effect: UIBlurEffect(style: .systemUltraThinMaterialDark))
    private let blurMask = CAGradientLayer()
    private let shade = CAGradientLayer()

    override init(frame: CGRect) {
        super.init(frame: frame)
        isUserInteractionEnabled = false
        let none = NSNull()

        // The blur fades out downwards (a gradient mask), the shade too, a little further.
        blurMask.colors = [UIColor.black.cgColor, UIColor.black.withAlphaComponent(0.85).cgColor, UIColor.clear.cgColor]
        blurMask.locations = [0, 0.55, 1]
        blurMask.actions = ["bounds": none, "position": none]
        blur.layer.mask = blurMask
        addSubview(blur)

        shade.colors = [
            UIColor.black.withAlphaComponent(0.6).cgColor,
            UIColor.black.withAlphaComponent(0.35).cgColor,
            UIColor.black.withAlphaComponent(0.12).cgColor,
            UIColor.clear.cgColor,
        ]
        shade.locations = [0, 0.45, 0.75, 1]
        shade.actions = ["bounds": none, "position": none]
        layer.addSublayer(shade)

        titleLabel.font = .systemFont(ofSize: 34, weight: .bold)
        titleLabel.textColor = .white
        dateLabel.font = .systemFont(ofSize: 17, weight: .semibold)
        dateLabel.textColor = .white
        for label in [titleLabel, dateLabel] {
            label.layer.shadowColor = UIColor.black.cgColor
            label.layer.shadowOpacity = 0.3
            label.layer.shadowRadius = 4
            label.layer.shadowOffset = .zero
            addSubview(label)
        }
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    /// `top`: where the bars end. The title sits in the bar's row, level with its buttons, and
    /// the date just under the bar.
    func layout(in bounds: CGRect, top: CGFloat) {
        frame = CGRect(x: 0, y: 0, width: bounds.width, height: top + 70)
        blur.frame = self.bounds
        blurMask.frame = blur.bounds
        shade.frame = self.bounds
        titleLabel.frame = CGRect(x: 20, y: top - 49, width: bounds.width * 0.6, height: 41)
        dateLabel.frame = CGRect(x: 20, y: top - 6, width: bounds.width - 40, height: 22)
    }

    var title: String? {
        get { titleLabel.text }
        set { if newValue != titleLabel.text { titleLabel.text = newValue } }
    }

    var text: String? {
        get { dateLabel.text }
        set { if newValue != dateLabel.text { dateLabel.text = newValue } }
    }

    // MARK: Wording

    private static func formatter(_ template: String) -> DateFormatter {
        let formatter = DateFormatter()
        formatter.setLocalizedDateFormatFromTemplate(template)
        return formatter
    }

    private static func intervalFormatter(_ template: String) -> DateIntervalFormatter {
        let formatter = DateIntervalFormatter()
        formatter.dateTemplate = template
        return formatter
    }

    private static let day = formatter("dMMMy")
    private static let days = intervalFormatter("dMMMy")

    private static let count: NumberFormatter = {
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        return formatter
    }()

    static func countText(_ count: Int) -> String {
        let number = Self.count.string(from: NSNumber(value: count)) ?? String(count)
        return count == 1 ? "1 Item" : "\(number) Items"
    }

    /// "18 Oct 2026" for one day, "4–7 Nov 2026" or "27 Oct–7 Nov 2026" for several.
    static func rangeText(from oldest: Date, to newest: Date) -> String {
        let start = min(oldest, newest), end = max(oldest, newest)
        return Calendar.current.isDate(start, inSameDayAs: end) ? day.string(from: start) : days.string(from: start, to: end)
    }
}
#endif
