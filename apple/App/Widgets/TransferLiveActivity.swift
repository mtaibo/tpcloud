import ActivityKit
import SwiftUI
import TPCore
import WidgetKit

@main
struct TPCloudWidgets: WidgetBundle {
    var body: some Widget {
        TransferLiveActivity()
    }
}

/// The backup's (and the Store uploads') Live Activity, styled like the Timer's: black, with
/// TPCloud's blue where the Timer has its orange — the logo in a tinted circle, the title and
/// count, the share done large on the right, and a thin bar under it all.
struct TransferLiveActivity: Widget {
    static let blue = Color(red: 0.04, green: 0.52, blue: 1.0)

    var body: some WidgetConfiguration {
        ActivityConfiguration(for: TransferActivityAttributes.self) { context in
            LockScreenView(attributes: context.attributes, state: context.state)
                .activityBackgroundTint(.black)
                .activitySystemActionForegroundColor(Self.blue)
        } dynamicIsland: { context in
            DynamicIsland {
                DynamicIslandExpandedRegion(.leading) {
                    LogoBadge(size: 44)
                        .padding(.leading, 4)
                }
                DynamicIslandExpandedRegion(.trailing) {
                    PercentText(state: context.state, size: 34)
                        .padding(.trailing, 4)
                }
                DynamicIslandExpandedRegion(.center) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(context.attributes.title)
                            .font(.headline)
                            .foregroundStyle(.white)
                        Text(Self.subtitle(context.attributes, context.state))
                            .font(.subheadline)
                            .foregroundStyle(Self.blue)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                DynamicIslandExpandedRegion(.bottom) {
                    ProgressBar(fraction: context.state.fraction)
                        .padding(.horizontal, 4)
                        .padding(.top, 6)
                }
            } compactLeading: {
                Image("Logo")
                    .resizable()
                    .scaledToFit()
                    .foregroundStyle(Self.blue)
                    .frame(width: 18, height: 18)
            } compactTrailing: {
                ProgressRing(fraction: context.state.fraction, lineWidth: 3)
                    .frame(width: 18, height: 18)
            } minimal: {
                ProgressRing(fraction: context.state.fraction, lineWidth: 3)
                    .overlay {
                        Image("Logo")
                            .resizable()
                            .scaledToFit()
                            .foregroundStyle(Self.blue)
                            .padding(5)
                    }
                    .frame(width: 22, height: 22)
            }
            .keylineTint(Self.blue)
        }
    }

    /// «25 of 974 photos and videos», «Preparing…», or how it ended.
    static func subtitle(_ attributes: TransferActivityAttributes, _ state: TransferActivityAttributes.ContentState) -> String {
        if let finished = state.finishedText { return finished }
        guard state.total > 0 else { return "Preparing…" }
        return "\(state.completed) of \(state.total) \(attributes.itemName)"
    }
}

private struct LockScreenView: View {
    let attributes: TransferActivityAttributes
    let state: TransferActivityAttributes.ContentState

    var body: some View {
        VStack(spacing: 12) {
            HStack(spacing: 14) {
                LogoBadge(size: 54)
                VStack(alignment: .leading, spacing: 2) {
                    Text(attributes.title)
                        .font(.headline)
                        .foregroundStyle(.white)
                    Text(TransferLiveActivity.subtitle(attributes, state))
                        .font(.subheadline)
                        .foregroundStyle(TransferLiveActivity.blue)
                        .lineLimit(1)
                }
                Spacer(minLength: 8)
                PercentText(state: state, size: 44)
            }
            ProgressBar(fraction: state.fraction)
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 16)
    }
}

/// The logo in a circle of dimmed blue, like the Timer's buttons.
private struct LogoBadge: View {
    let size: CGFloat

    var body: some View {
        Circle()
            .fill(TransferLiveActivity.blue.opacity(0.24))
            .frame(width: size, height: size)
            .overlay {
                Image("Logo")
                    .resizable()
                    .scaledToFit()
                    .foregroundStyle(TransferLiveActivity.blue)
                    .padding(size * 0.26)
            }
    }
}

/// «38%», large and thin like the Timer's digits; a check when it's done.
private struct PercentText: View {
    let state: TransferActivityAttributes.ContentState
    let size: CGFloat

    var body: some View {
        Group {
            if state.finishedText != nil {
                Image(systemName: state.completed >= state.total ? "checkmark" : "exclamationmark")
                    .font(.system(size: size * 0.7, weight: .light))
            } else if state.total == 0 {
                Text("…")
                    .font(.system(size: size, weight: .light))
            } else {
                Text("\(Int((state.fraction * 100).rounded(.down)))%")
                    .font(.system(size: size, weight: .light))
                    .monospacedDigit()
                    .contentTransition(.numericText())
            }
        }
        .foregroundStyle(TransferLiveActivity.blue)
        .lineLimit(1)
        .minimumScaleFactor(0.6)
    }
}

private struct ProgressBar: View {
    let fraction: Double

    var body: some View {
        GeometryReader { proxy in
            ZStack(alignment: .leading) {
                Capsule().fill(TransferLiveActivity.blue.opacity(0.22))
                Capsule()
                    .fill(TransferLiveActivity.blue)
                    .frame(width: max(proxy.size.width * fraction, fraction > 0 ? 4 : 0))
            }
        }
        .frame(height: 4)
    }
}

private struct ProgressRing: View {
    let fraction: Double
    let lineWidth: CGFloat

    var body: some View {
        ZStack {
            Circle().stroke(TransferLiveActivity.blue.opacity(0.25), lineWidth: lineWidth)
            Circle()
                .trim(from: 0, to: max(fraction, 0.02))
                .stroke(TransferLiveActivity.blue, style: StrokeStyle(lineWidth: lineWidth, lineCap: .round))
                .rotationEffect(.degrees(-90))
        }
    }
}
