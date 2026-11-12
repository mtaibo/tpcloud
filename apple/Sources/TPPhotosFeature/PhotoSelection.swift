import Observation
import SwiftUI
import TPCore
#if os(iOS)
import UIKit
#endif

/// Select mode, shared by the gallery's bars (SwiftUI) and the grid (UIKit), which does the
/// tapping and dragging. Observable property by property, so dragging across photos only
/// redraws what shows the count.
@MainActor
@Observable
final class PhotoSelection {
    var isActive = false {
        didSet { if !isActive { ids.removeAll() } }
    }
    var ids: Set<Int> = []
    /// Bumped by "Show Selected": the grid opens the viewer on the selected photos.
    var showSelectedSignal = 0
}

#if os(iOS)

/// Select mode's bottom bar, in place of the tab bar, like Photos': share, how many are selected
/// (tap to see them), delete.
struct PhotoSelectionBar: View {
    let selection: PhotoSelection
    let isWorking: Bool
    let onShare: () -> Void
    let onDelete: () -> Void

    var body: some View {
        let count = selection.ids.count
        HStack(spacing: 12) {
            Button(action: onShare) {
                Group {
                    if isWorking { ProgressView() } else { Image(systemName: "square.and.arrow.up") }
                }
                .frame(width: 24, height: 24)
            }
            .buttonStyle(.glass)
            .buttonBorderShape(.circle)
            .controlSize(.large)
            .disabled(count == 0 || isWorking)
            .accessibilityLabel("Share")

            Spacer(minLength: 0)
            Button {
                if count > 0 { selection.showSelectedSignal += 1 }
            } label: {
                Text(count == 0 ? "Select Items" : "Show Selected (\(count))")
                    .font(.body.weight(.semibold))
                    .monospacedDigit()
                    .lineLimit(1)
                    .fixedSize()
                    .padding(.horizontal, 6)
            }
            .buttonStyle(.glass)
            .controlSize(.large)
            .allowsHitTesting(count > 0)
            Spacer(minLength: 0)

            Button(role: .destructive, action: onDelete) {
                Image(systemName: "trash")
                    .frame(width: 24, height: 24)
            }
            .buttonStyle(.glass)
            .buttonBorderShape(.circle)
            .controlSize(.large)
            .disabled(count == 0 || isWorking)
            .accessibilityLabel("Delete")
        }
        .padding(.horizontal, 20)
        .padding(.bottom, 4)
    }
}

/// The system share sheet, for the selected photos' originals.
struct PhotoShareSheet: UIViewControllerRepresentable {
    let items: [URL]

    func makeUIViewController(context: Context) -> UIActivityViewController {
        UIActivityViewController(activityItems: items, applicationActivities: nil)
    }

    func updateUIViewController(_ controller: UIActivityViewController, context: Context) {}
}

enum PhotoShareDownloads {
    /// Downloads the originals (a few at a time) into a fresh temporary folder, under their own
    /// names, for the share sheet.
    static func originals(of photos: [Photo], client: PhotosClient) async -> [URL] {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("Share-\(UUID().uuidString)", isDirectory: true)
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        return await withTaskGroup(of: (Int, URL?).self) { group in
            var results: [(Int, URL?)] = []
            var iterator = photos.enumerated().makeIterator()
            func addNext() {
                guard let (index, photo) = iterator.next() else { return }
                let url = client.originalURL(for: photo)
                let name = photo.originalFilename.isEmpty ? "\(photo.id).jpg" : photo.originalFilename
                let destination = folder.appendingPathComponent("\(index)-\(name)")
                group.addTask {
                    guard let (temp, response) = try? await URLSession.shared.download(from: url),
                          let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else { return (index, nil) }
                    try? FileManager.default.moveItem(at: temp, to: destination)
                    return (index, destination)
                }
            }
            for _ in 0..<4 { addNext() }
            for await result in group {
                results.append(result)
                addNext()
            }
            return results.sorted { $0.0 < $1.0 }.compactMap(\.1)
        }
    }
}
#endif
