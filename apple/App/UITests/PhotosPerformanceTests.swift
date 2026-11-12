import XCTest

/// Drives the Photos tab on a real device and measures how smooth it is: the app (launched with
/// `-measureGridHitches`) adds up how late every frame is while the grid moves, and each test
/// prints the hitch time ratio — milliseconds of hitches per second of movement, Apple's own
/// measure (under 5 is good, over 10 is visibly janky). Run with `make perf` on a signed-in iPhone.
final class PhotosPerformanceTests: XCTestCase {
    private var app: XCUIApplication!

    override func setUp() {
        continueAfterFailure = true
    }

    /// Launches on the Photos tab with the grid at `level` (0 = 3 columns … 4 = 31).
    private func launch(level: Int?, extra: [String] = []) {
        app = XCUIApplication()
        app.launchArguments = ["-measureGridHitches"] + (level.map { ["-photosStartLevel", String($0)] } ?? []) + extra
        app.launch()
        let photos = app.tabBars.buttons["Photos"]
        XCTAssertTrue(photos.waitForExistence(timeout: 15))
        // The tab bar can swallow the first tap while the app settles: make sure Photos is up.
        let library = app.staticTexts["Library"]
        for _ in 0..<4 where !library.exists {
            photos.tap()
            _ = library.waitForExistence(timeout: 3)
        }
        XCTAssertTrue(library.exists, "not on the Photos tab")
        XCTAssertTrue(statsElement.waitForExistence(timeout: 15))
        // Let the grid, the photo list and the thumbnail table settle.
        sleep(3)
    }

    private var grid: XCUIElement { app.windows.firstMatch }
    private var statsElement: XCUIElement { app.descendants(matching: .any)["PhotoGridStats"].firstMatch }

    private struct Stats {
        var hitchMs = 0.0, movingS = 0.0, frames = 0, late = 0
        var grid = "", zoom = "", lateLog = "", track = "", jumps = "", present = "", open = "", sharp = ""
    }

    private func stats() -> Stats {
        var result = Stats()
        for part in ((statsElement.value as? String) ?? "").split(separator: " ") {
            let pair = part.split(separator: "=")
            guard pair.count == 2 else { continue }
            switch pair[0] {
            case "hitchMs": result.hitchMs = Double(pair[1]) ?? 0
            case "movingS": result.movingS = Double(pair[1]) ?? 0
            case "frames": result.frames = Int(pair[1]) ?? 0
            case "late" where !pair[1].contains("@"): result.late = Int(pair[1]) ?? 0
            case "grid": result.grid = String(pair[1])
            case "track": result.track = String(pair[1])
            case "jumps": result.jumps = String(pair[1])
            case "zoom": result.zoom = String(pair[1])
            case "late" where pair[1].contains("@"): result.lateLog = String(pair[1])
            case "present": result.present = String(pair[1])
            case "open": result.open = String(pair[1])
            case "sharp": result.sharp = String(pair[1])
            default: break
            }
        }
        return result
    }

    private func report(_ name: String, _ work: () -> Void) {
        let before = stats()
        work()
        sleep(1)
        let after = stats()
        let hitch = after.hitchMs - before.hitchMs
        let moving = max(after.movingS - before.movingS, 0.001)
        let frames = after.frames - before.frames
        let late = after.late - before.late
        print("PERF \(name) zoom steps: \(after.zoom)")
        print("PERF \(name) late frames (ms late @ ms since zoom start): \(after.lateLog)")
        print("PERF \(name) grid before: \(before.grid) after: \(after.grid)")
        print(String(format: "PERF %@: hitch ratio %.1f ms/s (%.0f ms of hitches over %.1f s, %d of %d frames late)",
                     name, hitch / moving, hitch, moving, late, frames))
    }

    /// Zoom In / Zoom Out from the ••• menu: the same animation as a pinch, without relying on
    /// XCUITest's synthetic pinches (which the grid doesn't see as one).
    private func menuZoom(_ title: String) {
        app.navigationBars.buttons["More"].firstMatch.tap()
        let item = app.buttons[title]
        guard item.waitForExistence(timeout: 3), item.isEnabled else {
            app.tap()
            return
        }
        item.tap()
        sleep(1)
    }

    private func fling(times: Int) {
        for _ in 0..<times { grid.swipeDown(velocity: .fast) }
        for _ in 0..<times { grid.swipeUp(velocity: .fast) }
    }

    func testFlingAtThreeColumns() {
        launch(level: 0)
        report("fling 3 columns") { fling(times: 6) }
    }

    func testFlingAtFiveColumns() {
        launch(level: 1)
        report("fling 5 columns") { fling(times: 6) }
    }

    func testFlingAtFifteenColumns() {
        launch(level: 3)
        report("fling 15 columns") { fling(times: 6) }
    }

    func testFlingAtThirtyOneColumns() {
        launch(level: 4)
        report("fling 31 columns") { fling(times: 6) }
    }

    func testZoomThroughEveryLevel() {
        launch(level: 0)
        report("zoom 3→31→3 (menu)") {
            for _ in 0..<4 { menuZoom("Zoom Out") }
            for _ in 0..<4 { menuZoom("Zoom In") }
        }
    }

    /// The app zooms 3→31→3 by itself (`-runZoomCycle`), with the test only watching.
    func testZoomCycleUntouched() {
        // The cycle starts 10 s after launch and takes ~10 s: measure across all of it.
        launch(level: 0, extra: ["-runZoomCycle"])
        report("zoom 3→31→3 (untouched)") { sleep(18) }
    }

    /// Screenshots at each level and in the middle of zooms, to look at.
    func testZoomLooksRight() {
        launch(level: 0, extra: ["-runZoomCycle"])
        let start = Date()
        var shots = 0
        sleep(4)
        while Date().timeIntervalSince(start) < 18 {
            let shot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
            shot.name = String(format: "zoom-%02d-%.1fs", shots, Date().timeIntervalSince(start))
            shot.lifetime = .keepAlways
            add(shot)
            shots += 1
        }
    }

    /// Quick flings one way and straight back the other, from the newest photos at the bottom.
    func testFlingDownAndBackUp() {
        launch(level: 1)
        report("fling back and forth from the bottom") {
            for _ in 0..<8 {
                grid.swipeUp(velocity: .fast)
                grid.swipeDown(velocity: .fast)
            }
            sleep(3)
            print("PERF jumps: \(stats().jumps)")
        }
    }

    func testDateHeader() {
        launch(level: 1)
        func snap(_ name: String) {
            let shot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
            shot.name = "zoom-header-\(name)"
            shot.lifetime = .keepAlways
            add(shot)
        }
        snap("1-open")
        grid.swipeDown(velocity: .fast)
        snap("2-scrolling")
        sleep(2)
        snap("3-stopped")
        for _ in 0..<3 { grid.swipeDown(velocity: .fast) }
        sleep(2)
        snap("4-older")
    }

    func testHeaderAtEveryLevel() {
        for level in [1, 3, 4] {
            launch(level: level)
            if level == 1 {
                snapHeader("lv\(level)-0-open")
            }
            grid.swipeDown(velocity: .fast)
            sleep(2)
            snapHeader("lv\(level)-1-scrolled")
            app.terminate()
        }
        launch(level: 1)
        app.navigationBars.buttons["More"].firstMatch.tap()
        sleep(1)
        snapHeader("lv9-menu")
    }

    private func snapHeader(_ name: String) {
        let shot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        shot.name = "zoom-header-\(name)"
        shot.lifetime = .keepAlways
        add(shot)
    }

    /// Select mode, the way it was tried on Photos (never deletes or shares).
    func testSelect() {
        launch(level: 1)
        app.navigationBars.buttons["Select"].firstMatch.tap()
        sleep(1)
        snapHeader("sel-1-mode")
        print("PERF select mode buttons: " + app.buttons.allElementsBoundByIndex.prefix(30).map(\.label).joined(separator: " | "))
        for point in [(0.3, 0.5), (0.5, 0.5), (0.7, 0.6)] {
            grid.coordinate(withNormalizedOffset: CGVector(dx: point.0, dy: point.1)).tap()
            usleep(400_000)
        }
        snapHeader("sel-2-three")
        print("PERF after taps: " + app.buttons.allElementsBoundByIndex.filter { $0.label.contains("Selected") || $0.label.contains("Select Items") }.map(\.label).joined(separator: ","))
        let start = grid.coordinate(withNormalizedOffset: CGVector(dx: 0.1, dy: 0.35))
        let end = grid.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.42))
        start.press(forDuration: 0.15, thenDragTo: end, withVelocity: .slow, thenHoldForDuration: 0.3)
        sleep(1)
        snapHeader("sel-3-dragged")
        print("PERF after drag: " + app.buttons.allElementsBoundByIndex.filter { $0.label.contains("Selected") }.map(\.label).joined(separator: ","))
        let low = grid.coordinate(withNormalizedOffset: CGVector(dx: 0.2, dy: 0.62))
        let edge = grid.coordinate(withNormalizedOffset: CGVector(dx: 0.8, dy: 0.86))
        low.press(forDuration: 0.15, thenDragTo: edge, withVelocity: .slow, thenHoldForDuration: 1.2)
        sleep(1)
        snapHeader("sel-4-edge")
        print("PERF after edge drag: " + app.buttons.allElementsBoundByIndex.filter { $0.label.contains("Selected") }.map(\.label).joined(separator: ","))
        let show = app.buttons.allElementsBoundByIndex.first { $0.label.hasPrefix("Show Selected") }
        show?.tap()
        sleep(2)
        snapHeader("sel-5-show-selected")
        let close = app.buttons["close"]
        if close.exists { close.tap(); sleep(1) }
        app.navigationBars.buttons["Done Selecting"].firstMatch.tap()
        sleep(1)
        snapHeader("sel-6-done")
    }

    func testOpenAndClosePhotos() {
        launch(level: 0)
        let close = app.buttons["close"]
        var times: [Double] = []
        for row in 0..<6 {
            // A different photo each time, so some open from the network and some from disk.
            let point = grid.coordinate(withNormalizedOffset: CGVector(dx: 0.2 + 0.3 * Double(row % 3), dy: 0.5 + 0.12 * Double(row / 3)))
            let start = Date()
            point.tap()
            XCTAssertTrue(close.waitForExistence(timeout: 10), "the viewer didn't open")
            times.append(Date().timeIntervalSince(start))
            sleep(1)
            close.tap()
            _ = close.waitForNonExistence(timeout: 5)
            sleep(1)
        }
        let measured = stats()
        print("PERF open photo (seen by the test): " + times.map { String(format: "%.2f s", $0) }.joined(separator: ", "))
        print("PERF open photo (in the app, ms from tap): animation starts \(measured.present); viewer settled \(measured.open); sharp picture \(measured.sharp)")
    }

}

/// Looks at Apple's Photos app, to copy how it behaves: screenshots while it scrolls and zooms,
/// and the text it shows (date headers and the like).
final class ApplePhotosObservationTests: XCTestCase {
    private func shot(_ app: XCUIApplication, _ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    private func texts(_ app: XCUIApplication, _ label: String) {
        let labels = app.staticTexts.allElementsBoundByIndex.prefix(40).map { "\($0.label)@\(Int($0.frame.minY))" }
        let buttons = app.buttons.allElementsBoundByIndex.prefix(30).map { $0.label }
        print("OBS \(label) texts: \(labels.joined(separator: " | "))")
        print("OBS \(label) buttons: \(buttons.joined(separator: " | "))")
    }

    /// Miguel pinches by hand; the test takes a screenshot (and reads the header) every second.
    func testWatchManualZoom() {
        let app = XCUIApplication(bundleIdentifier: "com.apple.mobileslideshow")
        app.launch()
        sleep(2)
        for second in 0..<45 {
            let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
            attachment.name = String(format: "obs-z%02d", second)
            attachment.lifetime = .keepAlways
            add(attachment)
            sleep(1)
        }
    }

    /// Select mode: what appears, how photos get marked, tapping and dragging across them.
    func testObserveSelect() {
        let app = XCUIApplication(bundleIdentifier: "com.apple.mobileslideshow")
        app.launch()
        sleep(2)
        let window = app.windows.firstMatch
        texts(app, "before select")
        app.buttons["Select"].firstMatch.tap()
        sleep(1)
        shot(app, "obs-s1-select-mode")
        texts(app, "select mode")
        // Tap three photos.
        for (index, point) in [(0.3, 0.5), (0.5, 0.5), (0.7, 0.6)].enumerated() {
            window.coordinate(withNormalizedOffset: CGVector(dx: point.0, dy: point.1)).tap()
            usleep(400_000)
            if index == 0 { shot(app, "obs-s2-one-selected"); texts(app, "one selected") }
        }
        shot(app, "obs-s3-three-selected")
        texts(app, "three selected")
        // Drag across a row and down.
        let start = window.coordinate(withNormalizedOffset: CGVector(dx: 0.1, dy: 0.35))
        let end = window.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.42))
        start.press(forDuration: 0.15, thenDragTo: end, withVelocity: .slow, thenHoldForDuration: 0.3)
        sleep(1)
        shot(app, "obs-s4-dragged")
        texts(app, "after drag")
        // Drag down past the screen's edge: does it scroll while selecting?
        let low = window.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.7))
        let edge = window.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.86))
        low.press(forDuration: 0.15, thenDragTo: edge, withVelocity: .slow, thenHoldForDuration: 1.0)
        sleep(1)
        shot(app, "obs-s5-edge-drag")
        texts(app, "edge drag")
        // Leave without touching anything.
        for label in ["Cancel", "Done", "Close"] where app.buttons[label].firstMatch.exists {
            app.buttons[label].firstMatch.tap()
            break
        }
        sleep(1)
        shot(app, "obs-s7-after-cancel")
    }

    func testObserveDenseLevels() {
        let app = XCUIApplication(bundleIdentifier: "com.apple.mobileslideshow")
        app.launch()
        sleep(3)
        let window = app.windows.firstMatch
        window.swipeDown(velocity: .fast)
        sleep(2)
        shot(app, "obs-d0-start")
        for (index, velocity) in [-0.3, -0.5, -1.0].enumerated() {
            window.pinch(withScale: 0.4, velocity: velocity)
            sleep(2)
            shot(app, "obs-d\(index + 1)-pinch\(velocity)")
        }
        window.swipeDown(velocity: .slow)
        sleep(2)
        shot(app, "obs-d4-scrolled")
        window.pinch(withScale: 0.4, velocity: -0.5)
        sleep(2)
        shot(app, "obs-d5-more")
        window.swipeDown(velocity: .slow)
        sleep(2)
        shot(app, "obs-d6-more-scrolled")
    }

    func testObservePhotos() {
        let app = XCUIApplication(bundleIdentifier: "com.apple.mobileslideshow")
        app.launch()
        sleep(3)
        shot(app, "obs-01-open")
        texts(app, "open")
        let window = app.windows.firstMatch

        // Scrolling into older photos: what appears while it moves and once it stops.
        window.swipeDown(velocity: .fast)
        shot(app, "obs-02-scrolling")
        texts(app, "scrolling")
        sleep(2)
        shot(app, "obs-03-stopped")
        texts(app, "stopped")
        window.swipeDown(velocity: .slow)
        shot(app, "obs-04-slow")

        // Zooming out and in.
        window.pinch(withScale: 0.5, velocity: -1)
        sleep(1)
        shot(app, "obs-05-zoomed-out")
        texts(app, "zoomed out")
        window.pinch(withScale: 0.5, velocity: -1)
        sleep(1)
        shot(app, "obs-06-zoomed-out-more")
        texts(app, "zoomed out more")
        window.swipeDown(velocity: .fast)
        shot(app, "obs-07-dense-scrolling")
        sleep(2)
        shot(app, "obs-08-dense-stopped")
        texts(app, "dense stopped")
        window.pinch(withScale: 3, velocity: 2)
        sleep(1)
        shot(app, "obs-09-zoomed-in")
        window.pinch(withScale: 3, velocity: 2)
        sleep(1)
        shot(app, "obs-10-zoomed-in-more")
        texts(app, "zoomed in more")

        // Opening a photo.
        window.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.55)).tap()
        sleep(1)
        shot(app, "obs-11-photo")
        texts(app, "photo")
    }
}
