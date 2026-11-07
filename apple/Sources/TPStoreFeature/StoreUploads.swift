import CoreTransferable
import Foundation
import Observation
import Photos
import PhotosUI
import SwiftUI
import TPCore
import TPUI
import UniformTypeIdentifiers

/// Every upload started from the Store tab, for the whole tab rather than one folder screen, so a
/// batch keeps going (and its progress stays on screen) while you browse elsewhere.
///
/// Files are read from disk a chunk at a time and photos are exported to a temporary file only
/// right before their turn. Uploading used to read each file whole into memory and send it in a
/// single request, which ran the app out of memory on a big batch and broke on any video over
/// Cloudflare's 100 MB request limit.
@MainActor
@Observable
final class StoreUploadQueue {
    @MainActor
    @Observable
    final class Item: Identifiable {
        enum Source {
            case file(URL)
            case photo(PhotosPickerItem)
        }

        enum State: Equatable {
            case waiting
            /// Exporting a photo or video from the library before it can be sent.
            case preparing
            case uploading
            case done
            case failed(String)
            case cancelled
        }

        let id = UUID()
        let folder: StoreFolder
        let source: Source
        /// A photo's real file name is only known once it's been exported.
        var name: String
        var state: State = .waiting
        /// 0...1, copied from `progress` a few times a second while uploading.
        var fraction: Double = 0
        @ObservationIgnored nonisolated let progress = Progress(totalUnitCount: 1)

        init(folder: StoreFolder, source: Source, name: String) {
            self.folder = folder
            self.source = source
            self.name = name
        }

        var isFinished: Bool {
            switch state {
            case .done, .failed, .cancelled: true
            case .waiting, .preparing, .uploading: false
            }
        }
    }

    private(set) var items: [Item] = []

    private let storeClient: StoreClient
    private let cache: StoreDirectoryCache
    @ObservationIgnored private var runner: Task<Void, Never>?
    @ObservationIgnored private var foldersPendingRefresh: Set<StoreFolder> = []

    /// Files sent at once: enough to keep the connection busy through a batch of small photos,
    /// few enough not to swamp the Pi.
    private static let concurrency = 3

    init(storeClient: StoreClient, cache: StoreDirectoryCache) {
        self.storeClient = storeClient
        self.cache = cache
    }

    // MARK: - Summary

    /// Cancelled uploads drop out of the counts, as if they'd never been asked for.
    private var counted: [Item] { items.filter { $0.state != .cancelled } }

    var totalCount: Int { counted.count }
    var doneCount: Int { counted.count(where: { $0.state == .done }) }
    var failedItems: [Item] { items.filter { if case .failed = $0.state { true } else { false } } }
    var isRunning: Bool { items.contains { !$0.isFinished } }

    var fractionCompleted: Double {
        let counted = counted
        guard !counted.isEmpty else { return 0 }
        return counted.reduce(0) { $0 + ($1.state == .done ? 1 : $1.fraction) } / Double(counted.count)
    }

    /// The files on their way right now, for the bar's subtitle.
    var activeNames: [String] {
        items.filter { $0.state == .uploading || $0.state == .preparing }.map(\.name)
    }

    // MARK: - Adding

    func enqueue(files urls: [URL], into folder: StoreFolder) {
        items += urls.map { Item(folder: folder, source: .file($0), name: $0.lastPathComponent) }
        start()
    }

    func enqueue(photos picks: [PhotosPickerItem], into folder: StoreFolder) {
        items += picks.map { Item(folder: folder, source: .photo($0), name: "Photo") }
        start()
    }

    // MARK: - Control

    /// Stops what's in flight and drops what hasn't started.
    func cancelAll() {
        runner?.cancel()
        for item in items where !item.isFinished && item.state != .uploading && item.state != .preparing {
            item.state = .cancelled
        }
    }

    func retryFailed() {
        for item in failedItems {
            item.state = .waiting
            item.fraction = 0
            item.progress.completedUnitCount = 0
        }
        start()
    }

    /// Clears a finished batch off the screen.
    func dismiss() {
        guard !isRunning else { return }
        items.removeAll()
    }

    // MARK: - Running

    private func start() {
        guard runner == nil else { return }
        runner = Task { await run() }
    }

    private func run() async {
        let activity = BackgroundActivity.begin(named: "Store uploads")
        #if os(iOS)
        // Keeps the batch going once the app is in the background or the screen locks, with its
        // progress in the system's own UI; without it iOS suspends the app within seconds.
        let continuedTask = ContinuedProcessingTask(
            identifierPrefix: "com.migueltaibo.tpapp.storeupload.continued",
            title: "Uploading to Store",
            itemName: "files"
        )
        continuedTask.start(onExpire: { [weak self] in self?.cancelAll() })
        #endif

        let poller = Task {
            while !Task.isCancelled {
                for item in items where item.state == .uploading {
                    item.fraction = item.progress.fractionCompleted
                }
                try? await Task.sleep(for: .milliseconds(250))
            }
        }

        await withTaskGroup(of: Void.self) { group in
            var running = 0
            while true {
                while running < Self.concurrency, !Task.isCancelled,
                      let item = items.first(where: { $0.state == .waiting }) {
                    item.state = .preparing
                    group.addTask { await self.upload(item) }
                    running += 1
                }
                guard running > 0 else { break }
                await group.next()
                running -= 1
                #if os(iOS)
                continuedTask.update(completed: doneCount + failedItems.count, total: totalCount)
                #endif
            }
        }

        poller.cancel()
        for item in items where item.state == .waiting { item.state = .cancelled }
        #if os(iOS)
        continuedTask.finish(success: failedItems.isEmpty)
        #endif
        activity.end()
        runner = nil
        // Picked up anything added in the moment between the last upload ending and here.
        if items.contains(where: { $0.state == .waiting }) { start() }
    }

    private func upload(_ item: Item) async {
        var temporaryDirectory: URL?
        defer { if let temporaryDirectory { try? FileManager.default.removeItem(at: temporaryDirectory) } }
        do {
            let fileURL: URL
            switch item.source {
            case .file(let url):
                fileURL = url
            case .photo(let pick):
                let exported = try await PhotoExport.export(pick)
                temporaryDirectory = exported.deletingLastPathComponent()
                item.name = exported.lastPathComponent
                fileURL = exported
            }
            try Task.checkCancellation()
            item.state = .uploading

            // Files picked in the Files app are only readable inside this scope.
            let accessed = fileURL.startAccessingSecurityScopedResource()
            defer { if accessed { fileURL.stopAccessingSecurityScopedResource() } }
            try await storeClient.uploadFile(
                at: fileURL,
                to: item.folder.path,
                location: item.folder.location,
                filename: item.name,
                progress: item.progress
            )
            item.fraction = 1
            item.state = .done
            refreshSoon(item.folder)
        } catch is CancellationError {
            item.state = .cancelled
        } catch {
            item.state = .failed(Self.message(for: error))
        }
    }

    /// One listing refresh per folder for a burst of finished uploads, not one per file.
    private func refreshSoon(_ folder: StoreFolder) {
        guard foldersPendingRefresh.insert(folder).inserted else { return }
        Task {
            try? await Task.sleep(for: .seconds(1))
            foldersPendingRefresh.remove(folder)
            _ = try? await cache.load(folder)
        }
    }

    private static func message(for error: Error) -> String {
        if let error = error as? APIError { return error.displayMessage }
        return error.localizedDescription
    }
}

// MARK: - Photo library export

/// Turns a photo or video picked from the library into a file with its real name (IMG_1234.HEIC,
/// IMG_1235.MOV…), the original quality, and any edits applied.
private enum PhotoExport {
    @concurrent
    static func export(_ pick: PhotosPickerItem) async throws -> URL {
        if let identifier = pick.itemIdentifier, let url = try await exportAsset(identifier) {
            return url
        }
        // Without library access the asset can't be looked up; the picker still hands over the file.
        guard let file = try await pick.loadTransferable(type: PickedMediaFile.self) else {
            throw APIError.transport("The photo couldn't be read from the library")
        }
        return file.url
    }

    private static func exportAsset(_ identifier: String) async throws -> URL? {
        let status = PHPhotoLibrary.authorizationStatus(for: .readWrite)
        guard status == .authorized || status == .limited,
              let asset = PHAsset.fetchAssets(withLocalIdentifiers: [identifier], options: nil).firstObject else {
            return nil
        }
        let resources = PHAssetResource.assetResources(for: asset)
        let isVideo = asset.mediaType == .video
        let original = resources.first { $0.type == (isVideo ? .video : .photo) }
        // An edited photo's full-size render is what the Photos app shows, so that's what goes up.
        let edited = resources.first { $0.type == (isVideo ? .fullSizeVideo : .fullSizePhoto) }
        guard let resource = edited ?? original else { return nil }

        // The render is called FullSizeRender.heic; keep the original's name with the render's extension.
        let stem = ((original?.filename ?? resource.filename ?? "Photo") as NSString).deletingPathExtension
        let fileExtension = ((resource.filename ?? "") as NSString).pathExtension
        let name = fileExtension.isEmpty ? stem : "\(stem).\(fileExtension)"

        let url = try temporaryFileURL(named: name)
        let options = PHAssetResourceRequestOptions()
        // Lets photos that only live in iCloud download first.
        options.isNetworkAccessAllowed = true
        try await PHAssetResourceManager.default().writeData(for: resource, toFile: url, options: options)
        return url
    }

    /// In a folder of its own, so two photos with the same name never clash.
    static func temporaryFileURL(named name: String) throws -> URL {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        return directory.appendingPathComponent(name)
    }
}

/// The picker's own file for a photo or video, copied out before the picker deletes it.
private struct PickedMediaFile: Transferable {
    let url: URL

    static var transferRepresentation: some TransferRepresentation {
        FileRepresentation(importedContentType: .movie) { try copy($0.file) }
        FileRepresentation(importedContentType: .image) { try copy($0.file) }
    }

    private static func copy(_ file: URL) throws -> PickedMediaFile {
        let url = try PhotoExport.temporaryFileURL(named: file.lastPathComponent)
        try FileManager.default.copyItem(at: file, to: url)
        return PickedMediaFile(url: url)
    }
}

// MARK: - Views

/// The batch's progress, floating over the bottom of the Store tab while it runs and until it's
/// dismissed. Tapping it lists every file and how it went.
struct StoreUploadBar: View {
    let uploads: StoreUploadQueue

    @State private var isShowingList = false

    var body: some View {
        HStack(spacing: 14) {
            statusIcon
                .font(.title2)
                .frame(width: 28)
            VStack(alignment: .leading, spacing: 4) {
                Text(title)
                    .font(.subheadline.weight(.semibold))
                    .monospacedDigit()
                if uploads.isRunning {
                    ProgressView(value: uploads.fractionCompleted)
                }
                if let subtitle {
                    Text(subtitle)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
            }
            Spacer(minLength: 0)
            if !uploads.isRunning, !uploads.failedItems.isEmpty {
                Button("Retry") { uploads.retryFailed() }
                    .buttonStyle(.bordered)
            }
            Button {
                if uploads.isRunning { uploads.cancelAll() } else { uploads.dismiss() }
            } label: {
                Image(systemName: "xmark")
                    .font(.footnote.weight(.semibold))
                    .frame(width: 32, height: 32)
            }
            .buttonStyle(.borderless)
            .foregroundStyle(.secondary)
            .accessibilityLabel(uploads.isRunning ? "Stop Uploading" : "Dismiss")
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
        .contentShape(.rect)
        .onTapGesture { isShowingList = true }
        .glassEffect(.regular, in: .rect(cornerRadius: 22))
        .padding(.horizontal, 16)
        .padding(.bottom, 8)
        .sheet(isPresented: $isShowingList) {
            StoreUploadList(uploads: uploads)
        }
        // A batch that all went up clears itself after a moment; one with failures waits for you.
        .task(id: uploads.isRunning) {
            guard !uploads.isRunning, uploads.failedItems.isEmpty else { return }
            try? await Task.sleep(for: .seconds(4))
            guard !Task.isCancelled else { return }
            withAnimation { uploads.dismiss() }
        }
    }

    @ViewBuilder
    private var statusIcon: some View {
        if uploads.isRunning {
            Image(systemName: "arrow.up.circle")
                .foregroundStyle(.tint)
                .symbolEffect(.pulse)
        } else if uploads.failedItems.isEmpty {
            Image(systemName: "checkmark.circle.fill")
                .foregroundStyle(.green)
        } else {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(.orange)
        }
    }

    private var title: String {
        let total = uploads.totalCount
        if uploads.isRunning {
            return "Uploading… \(uploads.doneCount) of \(total)"
        }
        let failed = uploads.failedItems.count
        if failed > 0 {
            return "\(failed) of \(total) couldn't upload"
        }
        return uploads.doneCount == 1 ? "Uploaded 1 item" : "Uploaded \(uploads.doneCount) items"
    }

    private var subtitle: String? {
        if uploads.isRunning {
            let failed = uploads.failedItems.count
            let names = uploads.activeNames.joined(separator: ", ")
            return failed > 0 ? "\(failed) failed · \(names)" : names
        }
        return uploads.failedItems.first.flatMap { item in
            if case .failed(let message) = item.state { message } else { nil }
        }
    }
}

/// Every file in the batch with how far it got, so a failure says which file and why.
private struct StoreUploadList: View {
    let uploads: StoreUploadQueue

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List(uploads.items) { item in
                StoreUploadRow(item: item)
            }
            .navigationTitle("Uploads")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
                if !uploads.isRunning, !uploads.failedItems.isEmpty {
                    ToolbarItem(placement: .primaryAction) {
                        Button("Retry Failed") { uploads.retryFailed() }
                    }
                }
            }
        }
        .presentationDetents([.medium, .large])
    }
}

private struct StoreUploadRow: View {
    let item: StoreUploadQueue.Item

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: FileKind(name: item.name).systemImage)
                .font(.title3)
                .foregroundStyle(.secondary)
                .frame(width: 28)
            VStack(alignment: .leading, spacing: 4) {
                Text(item.name)
                    .lineLimit(1)
                    .truncationMode(.middle)
                switch item.state {
                case .uploading:
                    ProgressView(value: item.fraction)
                case .failed(let message):
                    Text(message)
                        .font(.caption)
                        .foregroundStyle(.red)
                default:
                    Text(status)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            Spacer(minLength: 0)
            stateIcon
        }
    }

    private var status: String {
        switch item.state {
        case .waiting: "Waiting"
        case .preparing: "Preparing…"
        case .uploading: "Uploading"
        case .done: "Uploaded to \(item.folder.title)"
        case .failed: "Failed"
        case .cancelled: "Cancelled"
        }
    }

    @ViewBuilder
    private var stateIcon: some View {
        switch item.state {
        case .done:
            Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
        case .failed:
            Image(systemName: "exclamationmark.circle.fill").foregroundStyle(.orange)
        case .cancelled:
            Image(systemName: "xmark.circle").foregroundStyle(.secondary)
        case .preparing:
            ProgressView().controlSize(.small)
        case .waiting, .uploading:
            EmptyView()
        }
    }
}
