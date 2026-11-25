#if os(macOS)
import AppKit
import Testing
import TPCore
@testable import TPPhotosFeature

/// The Mac grid's zoom: at every moment the photos of the grid being left and of the new one
/// must sit in the same place on screen, or the grid visibly jumps as it zooms.
@MainActor
@Suite struct MacPhotoGridZoomTests {
    private static func photo(_ id: Int) -> Photo {
        Photo(id: id, path: "p/\(id).jpg", location: .external, checksum: "c\(id)", sourceAssetId: nil,
              originalFilename: "\(id).jpg", fileSize: 1, mimeType: "image/jpeg", width: 4, height: 3,
              takenAt: nil, isScreenshot: false, isCameraOriginal: true, sourceType: .userLibrary,
              cameraMake: nil, cameraModel: nil, gpsLat: nil, gpsLon: nil, status: .verified,
              uploadedAt: "2026-01-01T00:00:00Z", verifiedAt: nil)
    }

    private func makeGrid(level: Int) -> (MacPhotoGrid.Coordinator, NSWindow) {
        _ = NSApplication.shared
        let coordinator = MacPhotoGrid.Coordinator(client: PhotosClient(client: HTTPClient(baseURL: URL(string: "https://example.invalid")!)))
        let scrollView = NSScrollView(frame: NSRect(x: 0, y: 0, width: 1000, height: 700))
        scrollView.hasVerticalScroller = true
        let document = MacPhotoGridDocument()
        document.coordinator = coordinator
        scrollView.documentView = document
        scrollView.contentView.postsBoundsChangedNotifications = true
        let window = NSWindow(contentRect: scrollView.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = scrollView
        coordinator.attach(scrollView, document: document)
        coordinator.update(photos: (0..<2000).reversed().map(Self.photo), level: level)
        return (coordinator, window)
    }

    // One level at a time, between levels drawn as separate photos (15 and 31 are row pictures).
    @Test(arguments: [(3, 5), (5, 3), (5, 9), (9, 5)])
    func gridsMeetThroughoutTheZoom(fromColumns: Int, toColumns: Int) {
        let from = PhotoGridDensity.columnLevels.firstIndex(of: fromColumns)!
        let to = PhotoGridDensity.columnLevels.firstIndex(of: toColumns)!
        let (grid, window) = makeGrid(level: from)
        defer { window.close() }
        for scroll: CGFloat in [3000, 0, 1_000_000] {
            grid.debugScroll(toY: scroll)
            let y = grid.debugScrollY
            for point in [CGPoint(x: 500, y: y + 350), CGPoint(x: 120, y: y + 100), CGPoint(x: 900, y: y + 650)] {
                let start = grid.debugMismatch(to: to, around: point, progress: 0)
                #expect(start.compared > 0)
                #expect(start.latticeError < 3.5, "\(fromColumns)→\(toColumns) scroll \(y) point \(point): new cells off the old lattice by \(start.latticeError) pt (\(start.compared) cells)")
                let end = grid.debugMismatch(to: to, around: point, progress: 1)
                // Near the ends of the library the scroll is clamped, so the photo can't always
                // come back under the pointer; in the middle it must.
                if scroll == 3000 {
                    #expect(end.focalError < 1.5, "\(fromColumns)→\(toColumns) point \(point): the photo under the pointer ends \(end.focalError) pt away from it")
                }
            }
        }
    }
}
#endif

#if os(macOS)
import ImageIO
import UniformTypeIdentifiers

extension MacPhotoGridZoomTests {
    /// Writes frames of a zoom to $ZOOM_FRAMES (skipped otherwise), to look at.
    @Test func renderZoomFrames() throws {
        guard let folder = ProcessInfo.processInfo.environment["ZOOM_FRAMES"] else { return }
        MacPhotoGrid.Coordinator.debugTint = { item in
            NSColor(hue: CGFloat((item * 37) % 100) / 100, saturation: 0.7, brightness: 0.9, alpha: 1).cgColor
        }
        defer { MacPhotoGrid.Coordinator.debugTint = nil }
        for (fromColumns, toColumns) in [(5, 3), (3, 5)] {
            let from = PhotoGridDensity.columnLevels.firstIndex(of: fromColumns)!
            let to = PhotoGridDensity.columnLevels.firstIndex(of: toColumns)!
            let (grid, window) = makeGrid(level: from)
            defer { window.close() }
            grid.debugScroll(toY: 3000)
            let point = CGPoint(x: 300, y: grid.debugScrollY + 300)
            for progress in [0.0, 0.25, 0.5, 0.75, 1.0] {
                guard let image = grid.debugRender(to: to, around: point, progress: progress) else { continue }
                let url = URL(fileURLWithPath: folder).appendingPathComponent("zoom-\(fromColumns)-\(toColumns)-\(Int(progress * 100)).png")
                let dest = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil)!
                CGImageDestinationAddImage(dest, image, nil)
                CGImageDestinationFinalize(dest)
            }
        }
    }
}
#endif

#if os(macOS)
extension MacPhotoGridZoomTests {
    /// A steady pinch goes one level at a time and never takes a zoom back. (A zoom that marked
    /// its target as the current level made every event start another zoom a level further on,
    /// sweeping through every level on each frame.)
    @Test(arguments: [0.985, 1.015])
    func steadyPinchMovesOneLevelAtATime(step: Double) {
        let start = PhotoGridDensity.columnLevels.firstIndex(of: 9)!
        let (grid, window) = makeGrid(level: start)
        defer { window.close() }
        grid.debugScroll(toY: 3000)
        let scales = (1...300).map { CGFloat(pow(step, Double($0))) }
        let trace = grid.debugPinch(scales, at: CGPoint(x: 500, y: grid.debugScrollY + 350))
        #expect(grid.debugReverts == 0, "zooms taken back mid-pinch: \(grid.debugReverts)")
        var previous = start
        for entry in trace {
            #expect(abs(entry.level - previous) <= 1, "jumped from level \(previous) to \(entry.level)")
            #expect(abs(entry.shown - entry.level) <= 1, "laid out \(entry.shown) while at \(entry.level)")
            previous = entry.level
        }
        // Spreading ends at the largest level, pinching at the densest.
        #expect(trace.last?.level == (step > 1 ? 0 : PhotoGridDensity.columnLevels.count - 1))
    }
}
#endif

#if os(macOS)
import SwiftUI

extension MacPhotoGridZoomTests {
    /// The grid in a split view, as the app shows it, through a few display cycles and resizes:
    /// its date labels once made SwiftUI and AppKit lay each other out until AppKit threw.
    @Test func gridInASplitViewLaysOutWithoutLooping() throws {
        _ = NSApplication.shared
        let photos = (0..<3000).reversed().map { id -> Photo in
            let day = 1 + id % 28, month = 1 + (id / 28) % 12
            return Photo(id: id, path: "p/\(id).jpg", location: .external, checksum: "c\(id)", sourceAssetId: nil,
                         originalFilename: "\(id).jpg", fileSize: 1, mimeType: "image/jpeg", width: 4, height: 3,
                         takenAt: String(format: "2025-%02d-%02dT12:00:00Z", month, day), isScreenshot: false, isCameraOriginal: true,
                         sourceType: .userLibrary, cameraMake: nil, cameraModel: nil, gpsLat: nil, gpsLon: nil,
                         status: .verified, uploadedAt: "2026-01-01T00:00:00Z", verifiedAt: nil)
        }
        let client = PhotosClient(client: HTTPClient(baseURL: URL(string: "https://example.invalid")!))
        for columns in [5, 15, 31] {
            let level = PhotoGridDensity.columnLevels.firstIndex(of: columns)!
            // As the app has it: a tab of a sidebar-adaptable TabView, in a navigation stack with a toolbar.
            let root = TabView {
                Tab("Store", systemImage: "folder") { Text("Store") }
                Tab("Photos", systemImage: "photo") {
                    NavigationStack {
                        MacPhotoGrid(photos: photos, client: client, columnLevel: .constant(level), scrollToBottomSignal: 0, onOpen: { _ in }, onDelete: { _ in })
                            .navigationTitle("All")
                            .toolbar { ToolbarItem { Button("More", systemImage: "ellipsis") {} } }
                    }
                }
            }
            .tabViewStyle(.sidebarAdaptable)
            let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1100, height: 760), styleMask: [.titled, .resizable, .fullSizeContentView], backing: .buffered, defer: false)
            window.isReleasedWhenClosed = false
            window.contentView = NSHostingView(rootView: root)
            window.toolbar = NSToolbar()
            window.orderFront(nil)
            // Open the Photos tab, as the person does.
            RunLoop.main.run(until: Date().addingTimeInterval(0.2))
            for size in [NSSize(width: 1100, height: 760), NSSize(width: 800, height: 500), NSSize(width: 1300, height: 900)] {
                window.setContentSize(size)
                for _ in 0..<10 {
                    RunLoop.main.run(until: Date().addingTimeInterval(0.03))
                    window.displayIfNeeded()
                }
            }
            window.close()
        }
    }
}
#endif

#if os(macOS)
extension MacPhotoGridZoomTests {
    /// 40 photos a day for two years: every month is a few rows at 15 columns.
    static func datedPhotos() -> [Photo] {
        let start = Date(timeIntervalSince1970: 1_704_067_200) // 1 Jan 2024
        let formatter = ISO8601DateFormatter()
        return (0..<(730 * 12)).reversed().map { id in
            let date = start.addingTimeInterval(Double(id) * 86_400 / 12)
            return Photo(id: id, path: "p/\(id).jpg", location: .external, checksum: "c\(id)", sourceAssetId: nil,
                         originalFilename: "\(id).jpg", fileSize: 1, mimeType: "image/jpeg", width: 4, height: 3,
                         takenAt: formatter.string(from: date), isScreenshot: false, isCameraOriginal: true,
                         sourceType: .userLibrary, cameraMake: nil, cameraModel: nil, gpsLat: nil, gpsLon: nil,
                         status: .verified, uploadedAt: "2026-01-01T00:00:00Z", verifiedAt: nil)
        }
    }

    /// A grid with its date labels, at `columns`, scrolled to `y`: the labels showing.
    func dateLabels(columns: Int, scrolledTo y: CGFloat) async throws -> [(text: String, y: CGFloat)] {
        _ = NSApplication.shared
        let coordinator = MacPhotoGrid.Coordinator(client: PhotosClient(client: HTTPClient(baseURL: URL(string: "https://example.invalid")!)))
        let scrollView = NSScrollView(frame: NSRect(x: 0, y: 0, width: 1000, height: 700))
        let document = MacPhotoGridDocument()
        document.coordinator = coordinator
        scrollView.documentView = document
        scrollView.contentView.postsBoundsChangedNotifications = true
        // Made at 0×0, as the app makes it before the scroll view has a size.
        let dates = MacPhotoGridDates(frame: .zero)
        scrollView.addSubview(dates)
        let window = NSWindow(contentRect: scrollView.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = scrollView
        defer { window.close() }
        coordinator.attach(scrollView, document: document, dates: dates)
        coordinator.update(photos: Self.datedPhotos(), level: PhotoGridDensity.columnLevels.firstIndex(of: columns)!)
        // Month starts are worked out in the background.
        for _ in 0..<200 where !coordinator.debugDatesReady { try await Task.sleep(for: .milliseconds(50)) }
        coordinator.debugScroll(toY: y)
        return coordinator.debugDates()
    }

    /// The dense grids label each year (31 columns) or month (15) where it begins, the current
    /// one pinned at the top; the others say which days are on screen.
    @Test func dateLabelsAtEachLevel() async throws {
        let years = try await dateLabels(columns: 31, scrolledTo: 4300) // 1 Jan 2025 is on screen
        #expect(years.map(\.text) == ["2024", "2025"], "\(years)")
        let months = try await dateLabels(columns: 15, scrolledTo: 2000)
        #expect(months.count >= 1 && months.allSatisfy { $0.text.hasSuffix("2024") }, "\(months)")
        #expect(months.first?.y == 10, "the current month is pinned at the top: \(months)")
        let days = try await dateLabels(columns: 5, scrolledTo: 2000)
        #expect(days.count == 1 && days[0].text.contains("2024") && days[0].text.contains("–"), "\(days)")
    }
}
#endif
