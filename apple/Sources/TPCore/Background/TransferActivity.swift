#if os(iOS)
import ActivityKit
import Foundation

/// What the backup's and the Store uploads' Live Activity shows (the widget extension draws it):
/// black like the Timer's, TPCloud's blue, the logo, and «25 of 974 photos».
public struct TransferActivityAttributes: ActivityAttributes {
    public struct ContentState: Codable, Hashable, Sendable {
        public var completed: Int
        public var total: Int
        /// Set once it's over: «Done», or why it stopped.
        public var finishedText: String?

        public init(completed: Int, total: Int, finishedText: String? = nil) {
            self.completed = completed
            self.total = total
            self.finishedText = finishedText
        }

        public var fraction: Double { total > 0 ? min(Double(completed) / Double(total), 1) : 0 }
    }

    /// «Photo Backup», «Uploading to Store».
    public var title: String
    /// What's counted: «photos and videos», «files».
    public var itemName: String
    /// Which run it is, so the backup's own uploader in the background finds the right one.
    public var kind: String

    public init(title: String, itemName: String, kind: String) {
        self.title = title
        self.itemName = itemName
        self.kind = kind
    }
}

/// A run's Live Activity: `start()` when it begins (the app must be in the foreground), `update`
/// as items land, `finish` at the end. It replaced iOS 26's `BGContinuedProcessingTask`, whose
/// progress UI the system draws and can't be coloured (Miguel chose this look over the extra
/// running time that task gave; the backup's background uploader carries on with the app
/// suspended, updating the activity as it goes).
@MainActor
public final class TransferActivity {
    public nonisolated static let photoBackupKind = "photo-backup"

    private let attributes: TransferActivityAttributes
    private var activity: Activity<TransferActivityAttributes>?
    private var lastState = TransferActivityAttributes.ContentState(completed: 0, total: 0)

    public init(title: String, itemName: String, kind: String) {
        attributes = TransferActivityAttributes(title: title, itemName: itemName, kind: kind)
    }

    public func start() {
        guard ActivityAuthorizationInfo().areActivitiesEnabled else { return }
        // One per kind: a run started while the last one's activity is still up takes it over.
        if let existing = Activity<TransferActivityAttributes>.activities.first(where: { $0.attributes.kind == attributes.kind }) {
            activity = existing
            return
        }
        activity = try? Activity.request(attributes: attributes, content: ActivityContent(state: lastState, staleDate: nil))
    }

    public func update(completed: Int, total: Int) {
        let state = TransferActivityAttributes.ContentState(completed: completed, total: total)
        guard state != lastState, let activity else { return }
        lastState = state
        let id = activity.id
        Task.detached { await Self.find(id)?.update(ActivityContent(state: state, staleDate: nil)) }
    }

    /// `text` nil: it's handed over (the background uploader carries on and updates it).
    public func finish(text: String?) {
        guard let activity else { return }
        self.activity = nil
        guard let text else { return }
        var state = lastState
        state.finishedText = text
        let id = activity.id
        Task.detached { await Self.find(id)?.end(ActivityContent(state: state, staleDate: nil), dismissalPolicy: .after(.now + 4)) }
    }

    /// Activities aren't Sendable: each piece of work looks its own up by id where it runs.
    private nonisolated static func find(_ id: String) -> Activity<TransferActivityAttributes>? {
        Activity<TransferActivityAttributes>.activities.first { $0.id == id }
    }

    /// From the backup's background uploader, with the app maybe just woken for a moment: one
    /// more photo or video is up. Does nothing when no backup activity is showing.
    public nonisolated static func noteBackedUp() async {
        guard let activity = Activity<TransferActivityAttributes>.activities.first(where: { $0.attributes.kind == photoBackupKind }) else { return }
        var state = activity.content.state
        state.completed += 1
        if state.total > 0, state.completed >= state.total {
            state.completed = state.total
            state.finishedText = "Done"
            await activity.end(ActivityContent(state: state, staleDate: nil), dismissalPolicy: .after(.now + 4))
        } else {
            await activity.update(ActivityContent(state: state, staleDate: nil))
        }
    }

    /// The background uploader has nothing left to do: the backup's activity, if any, ends.
    public nonisolated static func endBackupIfShowing(text: String = "Done") async {
        for activity in Activity<TransferActivityAttributes>.activities where activity.attributes.kind == photoBackupKind {
            var state = activity.content.state
            state.finishedText = text
            await activity.end(ActivityContent(state: state, staleDate: nil), dismissalPolicy: .after(.now + 4))
        }
    }
}
#endif
