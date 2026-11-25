import Foundation

/// What the grid says about where you are in the library, on the iPhone and the Mac alike: the
/// count at the newest end, the dates on screen anywhere else, and where each month begins (for
/// the dense grids' month and year labels).
enum PhotoGridDateWording {
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

    nonisolated(unsafe) private static let day = formatter("dMMMy")
    nonisolated(unsafe) private static let days = intervalFormatter("dMMMy")
    nonisolated(unsafe) static let month = formatter("MMM")
    nonisolated(unsafe) static let year = formatter("y")

    nonisolated(unsafe) private static let count: NumberFormatter = {
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

    /// Where each month begins, oldest first, straight from the photos' ISO dates in display
    /// order: the month is read off the text («2025-08-…», the photo's own local time) and only
    /// each month's first date is parsed. Parsing all 12,000 dates of a library took seconds,
    /// and the month and year labels waited for it.
    nonisolated static func monthStarts(isoDates: [String?], parse: (String) -> Date?) -> [(item: Int, date: Date)] {
        var starts: [(item: Int, date: Date)] = []
        var last: Substring?
        for (item, text) in isoDates.enumerated() {
            guard let text, text.count >= 7 else { continue }
            let key = text.prefix(7)
            guard key != last, let date = parse(text) else { continue }
            starts.append((item, date))
            last = key
        }
        return starts
    }

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
