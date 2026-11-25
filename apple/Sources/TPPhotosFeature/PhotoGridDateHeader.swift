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

    /// How far right the title and date start: past the back button on a pushed screen (an
    /// album), at the edge on the tab's own screen.
    var leadingInset: CGFloat = 20

    /// `top`: where the bars end. The title sits in the bar's row, level with its buttons, and
    /// the date just under the bar.
    func layout(in bounds: CGRect, top: CGFloat) {
        frame = CGRect(x: 0, y: 0, width: bounds.width, height: top + 70)
        blur.frame = self.bounds
        blurMask.frame = blur.bounds
        shade.frame = self.bounds
        // The bar's buttons take the right ~40% (••• and Select).
        let titleWidth = max(bounds.width * 0.6 - (leadingInset - 20), 80)
        titleLabel.frame = CGRect(x: leadingInset, y: top - 49, width: titleWidth, height: 41)
        dateLabel.frame = CGRect(x: leadingInset, y: top - 6, width: bounds.width - leadingInset - 20, height: 22)
    }

    var title: String? {
        get { titleLabel.text }
        set { if newValue != titleLabel.text { titleLabel.text = newValue } }
    }

    var text: String? {
        get { dateLabel.text }
        set { if newValue != dateLabel.text { dateLabel.text = newValue } }
    }
}
#endif
