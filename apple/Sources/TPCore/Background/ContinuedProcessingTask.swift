#if os(iOS)
import BackgroundTasks
import Foundation

/// Keeps a user-started run (a "Backup now", a batch of Store uploads) alive after the app leaves
/// the foreground, using iOS 26's `BGContinuedProcessingTask`: the system shows the run's progress
/// in its own UI and lets the process keep executing while it keeps reporting progress, instead of
/// suspending it a few seconds after the user switches apps or locks the screen.
///
/// This is what actually makes a large backup finish unattended. `PhotoBackgroundUploader`'s
/// background-session chain can't: chunk tasks it enqueues while the app is already in the
/// background are treated as discretionary by iOS, so they mostly wait for Wi-Fi + charging.
///
/// Usage: `start(onExpire:)` when the run begins, `update(completed:total:)` as items land, and
/// `finish(success:)` exactly once at the end. If the system refuses the request (or the device
/// doesn't support it) every call is a no-op and the run just behaves as a foreground run.
@MainActor
public final class ContinuedProcessingTask {
    /// Must match a wildcard entry in Info.plist's `BGTaskSchedulerPermittedIdentifiers`
    /// (`<identifierPrefix>.*`).
    private let identifierPrefix: String
    private let title: String
    /// What's being counted, for the "3 of 10 photos" subtitle.
    private let itemName: String

    private var task: BGContinuedProcessingTask?
    private var isFinished = false
    private var lastCompleted = 0
    private var lastTotal = 0

    public init(identifierPrefix: String, title: String, itemName: String) {
        self.identifierPrefix = identifierPrefix
        self.title = title
        self.itemName = itemName
    }

    /// `onExpire` runs when the system (or the user, from the system progress UI) stops the task —
    /// the caller should stop starting new work.
    public func start(onExpire: @escaping @MainActor () -> Void) {
        // A fresh identifier per run: registering the same identifier twice throws, and continued
        // tasks (unlike BGProcessingTask) may be registered right before they're submitted.
        let identifier = "\(identifierPrefix).\(UUID().uuidString)"

        // `using: .main` so the launch handler runs on the main thread and can touch this
        // main-actor state directly, without sending the (non-Sendable) task across actors.
        let registered = BGTaskScheduler.shared.register(forTaskWithIdentifier: identifier, using: .main) { [weak self] task in
            MainActor.assumeIsolated {
                guard let task = task as? BGContinuedProcessingTask else {
                    task.setTaskCompleted(success: false)
                    return
                }
                guard let self, !self.isFinished else {
                    task.setTaskCompleted(success: true) // the run already ended before iOS launched this
                    return
                }
                task.expirationHandler = { [weak self] in
                    Task { @MainActor in
                        onExpire()
                        self?.finish(success: false)
                    }
                }
                self.task = task
                self.report()
            }
        }
        guard registered else { return }

        let request = BGContinuedProcessingTaskRequest(
            identifier: identifier,
            title: title,
            subtitle: "Preparing…"
        )
        // Fail instead of queueing: a run that only starts minutes later, after the user has
        // moved on, would be more confusing than one that just runs in the foreground.
        request.strategy = .fail
        BGTaskScheduler.shared.submitTaskRequest(request) { _ in }
    }

    public func update(completed: Int, total: Int) {
        lastCompleted = completed
        lastTotal = total
        report()
    }

    public func finish(success: Bool) {
        guard !isFinished else { return }
        isFinished = true
        guard let task else { return }
        report()
        task.setTaskCompleted(success: success)
        self.task = nil
    }

    private func report() {
        guard let task else { return }
        // Total stays 0 (indeterminate) while the run is still working out what to do.
        task.progress.totalUnitCount = Int64(lastTotal)
        task.progress.completedUnitCount = Int64(lastCompleted)
        let subtitle = lastTotal == 0 ? "Preparing…" : "\(lastCompleted) of \(lastTotal) \(itemName)"
        task.updateTitle(title, subtitle: subtitle)
    }
}
#endif
