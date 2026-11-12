#if os(iOS)
import QuartzCore
import UIKit

/// Counts hitches while the grid moves (scrolling, flinging or zooming), the way Apple's hitch
/// time ratio does: every frame that arrives later than its deadline adds how late it was. Only
/// runs when the app is launched with `-measureGridHitches` (the performance UI tests), and puts
/// its running totals in `statsView`'s accessibility value for the test to read.
@MainActor
final class PhotoGridHitchMonitor: NSObject {
    static var isEnabled: Bool {
        ProcessInfo.processInfo.arguments.contains("-measureGridHitches")
    }

    /// Carries the totals for the test to read: a 1-point view that's always there, whatever
    /// the grid is doing.
    let statsView = UIView(frame: CGRect(x: 0, y: 0, width: 1, height: 1))
    private let isMoving: () -> Bool
    private var link: CADisplayLink?
    private var lastTimestamp: CFTimeInterval?
    private var lastTarget: CFTimeInterval?
    private var lastPublish: CFTimeInterval = 0
    private var hitchSeconds: CFTimeInterval = 0
    private var movingSeconds: CFTimeInterval = 0
    private var frames = 0
    private var lateFrames = 0
    /// Opening a photo: from the tap to the viewer on screen, and to its sharp picture.
    private var tapTime: CFTimeInterval?
    private var openMs: [Int] = []
    private var presentMs: [Int] = []
    /// Main-thread time of the zoom's own steps, in ms.
    private var zoomSteps: [String] = []

    var zoomStart: CFTimeInterval = 0
    private var lateLog: [String] = []

    func zoomStep(_ name: String, _ seconds: CFTimeInterval) {
        zoomSteps.append("\(name)\(Int(seconds * 1000))")
        if zoomSteps.count > 40 { zoomSteps.removeFirst() }
    }
    private var sharpMs: [Int] = []

    static weak var current: PhotoGridHitchMonitor?

    func photoTapped() { tapTime = CACurrentMediaTime() }

    func presentStarted() {
        if let tapTime { presentMs.append(Int((CACurrentMediaTime() - tapTime) * 1000)) }
    }

    func viewerAppeared() {
        if let tapTime { openMs.append(Int((CACurrentMediaTime() - tapTime) * 1000)) }
        publish()
    }

    func sharpPhotoShown() {
        guard let tapTime else { return }
        sharpMs.append(Int((CACurrentMediaTime() - tapTime) * 1000))
        self.tapTime = nil
        publish()
    }

    /// The grid's state, for the test to tell "didn't hitch" from "didn't move".
    var describeGrid: () -> String = { "" }
    /// The scroll offset over time while moving (ms since launch:y), for the tests to read.
    var offset: () -> CGFloat = { 0 }
    var state: () -> String = { "" }
    private var track: [String] = []
    private var jumps: [String] = []
    private var lastTrack: CFTimeInterval = 0

    init(isMoving: @escaping () -> Bool) {
        self.isMoving = isMoving
        super.init()
        statsView.isAccessibilityElement = true
        statsView.accessibilityIdentifier = "PhotoGridStats"
        statsView.isUserInteractionEnabled = false
        let link = CADisplayLink(target: self, selector: #selector(tick(_:)))
        link.preferredFrameRateRange = CAFrameRateRange(minimum: 80, maximum: 120, preferred: 120)
        link.add(to: .main, forMode: .common)
        self.link = link
        Self.current = self
        PhotoGridView.jumpLog = { [weak self] entry in
            self?.jumps.append(entry.replacingOccurrences(of: " ", with: "").replacingOccurrences(of: ",", with: ";"))
            self?.publish()
        }
    }

    func stop() {
        link?.invalidate()
        link = nil
    }

    @objc private func tick(_ link: CADisplayLink) {
        defer {
            lastTimestamp = link.timestamp
            lastTarget = link.targetTimestamp
        }
        if link.timestamp - lastTrack > 0.05 {
            lastTrack = link.timestamp
            track.append("\(Int(link.timestamp * 1000) % 1_000_000):\(Int(offset()))\(state())")
            if track.count > 400 { track.removeFirst(track.count - 400) }
        }
        if link.timestamp - lastPublish > 0.5 {
            lastPublish = link.timestamp
            publish()
        }
        guard let lastTimestamp, let lastTarget, isMoving() else { return }
        let elapsed = link.timestamp - lastTimestamp
        movingSeconds += elapsed
        frames += 1
        // This frame should have been shown at the previous frame's target time.
        let late = link.timestamp - lastTarget
        if late > 0.002 {
            hitchSeconds += late
            lateFrames += 1
            lateLog.append("\(Int(late * 1000))@\(Int((link.timestamp - zoomStart) * 1000))")
            if lateLog.count > 30 { lateLog.removeFirst() }
        }
        publish()
    }

    private func publish() {
        statsView.accessibilityValue = String(
            format: "hitchMs=%.1f movingS=%.2f frames=%d late=%d grid=%@ zoom=%@ late=%@ track=%@ jumps=%@ present=%@ open=%@ sharp=%@",
            hitchSeconds * 1000, movingSeconds, frames, lateFrames, describeGrid(), zoomSteps.joined(separator: ","), lateLog.joined(separator: ","), track.joined(separator: ","), jumps.joined(separator: "|"),
            presentMs.map(String.init).joined(separator: ","),
            openMs.map(String.init).joined(separator: ","), sharpMs.map(String.init).joined(separator: ",")
        )
    }
}
#endif
