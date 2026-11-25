#if os(macOS)
import AppKit

/// `-snapshotWindow 1` at launch: every second and a half the window draws itself into
/// `window.png` in the app's temporary folder, so the app can be looked at without a screenshot
/// (the terminal driving it may not be allowed to record the screen). Glass draws as plain, the
/// rest as it is.
enum WindowSnapshots {
    @MainActor static func startIfAsked() {
        guard UserDefaults.standard.bool(forKey: "snapshotWindow") else { return }
        let url = FileManager.default.temporaryDirectory.appending(path: "window.png")
        Timer.scheduledTimer(withTimeInterval: 1.5, repeats: true) { _ in
            MainActor.assumeIsolated {
                guard let window = NSApp.windows.first(where: { $0.isVisible && $0.contentView != nil }),
                      let view = window.contentView, let layer = view.layer else { return }
                let scale = window.backingScaleFactor
                let size = view.bounds.size
                guard let context = CGContext(data: nil, width: Int(size.width * scale), height: Int(size.height * scale),
                                              bitsPerComponent: 8, bytesPerRow: 0, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                              bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return }
                context.scaleBy(x: scale, y: scale)
                if layer.isGeometryFlipped || view.isFlipped {
                    context.translateBy(x: 0, y: size.height)
                    context.scaleBy(x: 1, y: -1)
                }
                layer.render(in: context)
                guard let image = context.makeImage() else { return }
                let rep = NSBitmapImageRep(cgImage: image)
                try? rep.representation(using: .png, properties: [:])?.write(to: url)
            }
        }
    }
}
#endif
