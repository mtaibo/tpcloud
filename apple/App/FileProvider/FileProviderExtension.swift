import FileProvider
import Foundation
import TPCore
import UniformTypeIdentifiers

/// TPCloud in the Finder: one domain per network the app is signed in to, rooted at the
/// account's folder on the host, files fetched when they're opened.
final class FileProviderExtension: NSObject, NSFileProviderReplicatedExtension, @unchecked Sendable {
    private let domain: NSFileProviderDomain
    private let remote: Remote
    private let catalog: Catalog

    required init(domain: NSFileProviderDomain) {
        self.domain = domain
        remote = Remote(networkID: domain.identifier.rawValue)
        catalog = Catalog(domain: domain.identifier)
        super.init()
    }

    func invalidate() {}

    // MARK: Items

    func item(for identifier: NSFileProviderItemIdentifier, request: NSFileProviderRequest,
              completionHandler: @escaping (NSFileProviderItem?, Error?) -> Void) -> Progress {
        let progress = Progress(totalUnitCount: 1)
        task(progress) { [self] in
            let item = try await lookUp(identifier)
            completionHandler(item, nil)
        } failed: { completionHandler(nil, $0) }
        return progress
    }

    /// An item as the host has it now, from its folder's listing.
    private func lookUp(_ identifier: NSFileProviderItemIdentifier) async throws -> Item {
        try await prepare()
        guard let path = catalog.path(for: identifier) else { throw NSFileProviderError(.noSuchItem) }
        if identifier == .rootContainer || identifier == Catalog.sharedIdentifier {
            return Item(path: path, entry: Entry(isDirectory: true, size: 0, modified: 0), catalog: catalog)
        }
        return try await lookUp(path: path)
    }

    private func lookUp(path: String) async throws -> Item {
        let parent = Catalog.parent(of: path), name = Catalog.name(of: path)
        let listing = try await remote.store { try await $0.listDirectory(path: parent) }
        guard let found = listing.entries.first(where: { $0.name == name && ($0.type == .file || $0.type == .directory) }) else {
            throw NSFileProviderError(.noSuchItem)
        }
        return Item(path: path, entry: Entry(isDirectory: found.isDirectory, size: found.size ?? 0, modified: found.modified), catalog: catalog)
    }

    /// The account's folder, known before any identifier is turned into a path.
    private func prepare() async throws {
        catalog.setRoot(try await remote.rootPath())
    }

    // MARK: Contents

    func fetchContents(for itemIdentifier: NSFileProviderItemIdentifier, version requestedVersion: NSFileProviderItemVersion?,
                       request: NSFileProviderRequest,
                       completionHandler: @escaping (URL?, NSFileProviderItem?, Error?) -> Void) -> Progress {
        let progress = Progress(totalUnitCount: 100)
        task(progress) { [self] in
            let item = try await lookUp(itemIdentifier)
            let downloaded = try await remote.store { try await $0.downloadFile(path: item.path, progress: progress) }
            // Somewhere the system can take the file from (the download's own place is temporary).
            let place = try NSFileProviderManager(for: domain)?.temporaryDirectoryURL() ?? FileManager.default.temporaryDirectory
            let file = place.appending(path: UUID().uuidString)
            try FileManager.default.moveItem(at: downloaded, to: file)
            completionHandler(file, item, nil)
        } failed: { completionHandler(nil, nil, $0) }
        return progress
    }

    // MARK: Changes from the Mac

    func createItem(basedOn itemTemplate: NSFileProviderItem, fields: NSFileProviderItemFields, contents url: URL?,
                    options: NSFileProviderCreateItemOptions = [], request: NSFileProviderRequest,
                    completionHandler: @escaping (NSFileProviderItem?, NSFileProviderItemFields, Bool, Error?) -> Void) -> Progress {
        let progress = Progress(totalUnitCount: 100)
        let name = itemTemplate.filename
        // .DS_Store, ._ files, editors' locks: the host hides dot names, so they stay on the Mac.
        if name.hasPrefix(".") {
            completionHandler(nil, [], false, NSFileProviderError(.excludedFromSync))
            return progress
        }
        task(progress) { [self] in
            try await prepare()
            guard let parent = catalog.path(for: itemTemplate.parentItemIdentifier) else { throw NSFileProviderError(.noSuchItem) }
            let path = "\(parent)/\(name)"
            let isFolder = itemTemplate.contentType == .folder || itemTemplate.contentType?.conforms(to: .directory) == true
            if isFolder {
                do {
                    try await remote.store { try await $0.createFolder(path: path) }
                } catch {
                    // The host answers «Already exists»: fine when the system said it might.
                    guard let existing = try? await lookUp(path: path) else { throw error }
                    guard existing.entry.isDirectory, options.contains(.mayAlreadyExist) else { throw NSFileProviderError(.filenameCollision) }
                }
                catalog.noted(path, Entry(isDirectory: true, size: 0, modified: 0))
            } else if let url {
                let mtime = itemTemplate.contentModificationDate ?? nil
                try await upload(url, to: parent, name: name, mtime: mtime, progress: progress)
            } else {
                // A symlink or something else with no contents.
                throw NSFileProviderError(.excludedFromSync)
            }
            let item = try await lookUp(path: path)
            if !isFolder { catalog.noted(path, item.entry) }
            completionHandler(item, [], false, nil)
        } failed: { completionHandler(nil, [], false, $0) }
        return progress
    }

    func modifyItem(_ item: NSFileProviderItem, baseVersion version: NSFileProviderItemVersion, changedFields: NSFileProviderItemFields,
                    contents newContents: URL?, options: NSFileProviderModifyItemOptions = [], request: NSFileProviderRequest,
                    completionHandler: @escaping (NSFileProviderItem?, NSFileProviderItemFields, Bool, Error?) -> Void) -> Progress {
        let progress = Progress(totalUnitCount: 100)
        task(progress) { [self] in
            try await prepare()
            guard var path = catalog.path(for: item.itemIdentifier) else { throw NSFileProviderError(.noSuchItem) }
            if changedFields.contains(.parentItemIdentifier) {
                guard let parent = catalog.path(for: item.parentItemIdentifier) else { throw NSFileProviderError(.noSuchItem) }
                let moved = "\(parent)/\(Catalog.name(of: path))"
                try await remote.store { [path] in try await $0.move(path: path, destDir: parent) }
                catalog.moved(from: path, to: moved)
                path = moved
            }
            if changedFields.contains(.filename), item.filename != Catalog.name(of: path) {
                let renamed = "\(Catalog.parent(of: path))/\(item.filename)"
                try await remote.store { [path] in try await $0.rename(path: path, newName: item.filename) }
                catalog.moved(from: path, to: renamed)
                path = renamed
            }
            if changedFields.contains(.contents), let newContents {
                let mtime = changedFields.contains(.contentModificationDate) ? item.contentModificationDate ?? nil : nil
                let current = try? await lookUp(path: path)
                if let current, current.itemVersion.contentVersion != version.contentVersion {
                    // Changed on the host too since the Mac last had it: both are kept, the Mac's
                    // under a name that says so, and the Mac gets the host's back.
                    let copy = Self.conflictName(for: Catalog.name(of: path))
                    try await upload(newContents, to: Catalog.parent(of: path), name: copy, mtime: mtime ?? Date(), progress: progress)
                    completionHandler(current, [], false, nil)
                    NSFileProviderManager(for: domain)?.signalEnumerator(for: .workingSet) { _ in }
                    return
                }
                try await upload(newContents, to: Catalog.parent(of: path), name: Catalog.name(of: path), mtime: mtime ?? Date(), progress: progress)
            }
            let updated = try await lookUp(path: path)
            if !updated.entry.isDirectory { catalog.noted(path, updated.entry) }
            // Tags, last-used dates and the like aren't kept by the host; none is left pending
            // (a pending field would be sent again and again).
            completionHandler(updated, [], false, nil)
        } failed: { completionHandler(nil, [], false, $0) }
        return progress
    }

    func deleteItem(identifier: NSFileProviderItemIdentifier, baseVersion version: NSFileProviderItemVersion,
                    options: NSFileProviderDeleteItemOptions = [], request: NSFileProviderRequest,
                    completionHandler: @escaping (Error?) -> Void) -> Progress {
        let progress = Progress(totalUnitCount: 1)
        task(progress) { [self] in
            try await prepare()
            guard let path = catalog.path(for: identifier) else { throw NSFileProviderError(.noSuchItem) }
            do {
                try await remote.store { try await $0.delete(path: path) }
            } catch NSFileProviderError.noSuchItem {}
            catalog.removed(path)
            completionHandler(nil)
        } failed: { completionHandler($0) }
        return progress
    }

    /// «Notes (conflict 2026-11-25 10.42 MacBook).txt», like the Linux copy names them.
    static func conflictName(for name: String) -> String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd HH.mm"
        let mac = Host.current().localizedName ?? "Mac"
        let tag = "(conflict \(formatter.string(from: Date())) \(mac))"
        let ext = (name as NSString).pathExtension
        let stem = (name as NSString).deletingPathExtension
        return ext.isEmpty ? "\(stem) \(tag)" : "\(stem) \(tag).\(ext)"
    }

    /// Small files in one request, big ones in chunks; either way the host keeps the Mac's date.
    private func upload(_ url: URL, to folder: String, name: String, mtime: Date?, progress: Progress) async throws {
        let size = (try? url.resourceValues(forKeys: [.fileSizeKey]).fileSize) ?? 0
        if size <= 8 * 1024 * 1024 {
            let data = try Data(contentsOf: url)
            try await remote.store { try await $0.upload(path: folder, filename: name, data: data, mtime: mtime) }
        } else {
            let child = Progress(totalUnitCount: Int64(size))
            progress.addChild(child, withPendingUnitCount: 100)
            try await remote.store { try await $0.uploadFile(at: url, to: folder, filename: name, mtime: mtime, progress: child) }
        }
    }

    // MARK: Enumeration

    func enumerator(for containerItemIdentifier: NSFileProviderItemIdentifier, request: NSFileProviderRequest) throws -> NSFileProviderEnumerator {
        switch containerItemIdentifier {
        case .workingSet: WorkingSetEnumerator(remote: remote, catalog: catalog)
        case .trashContainer: EmptyEnumerator()
        default: FolderEnumerator(identifier: containerItemIdentifier, remote: remote, catalog: catalog)
        }
    }

    /// Runs `work` as the system's request: cancelling `progress` cancels it, an error goes to `failed`.
    private func task(_ progress: Progress, _ work: @escaping @Sendable () async throws -> Void, failed: @escaping @Sendable (Error) -> Void) {
        let running = Task {
            do {
                try await work()
            } catch is CancellationError {
                failed(NSError(domain: NSCocoaErrorDomain, code: NSUserCancelledError))
            } catch {
                failed(error)
            }
        }
        progress.cancellationHandler = { running.cancel() }
    }
}

// MARK: - Thumbnails

/// Photos and videos show their picture in the Finder without being downloaded: the host's own
/// thumbnails (the ones the web and the app use).
extension FileProviderExtension: NSFileProviderThumbnailing {
    func fetchThumbnails(for itemIdentifiers: [NSFileProviderItemIdentifier], requestedSize size: CGSize,
                         perThumbnailCompletionHandler: @escaping (NSFileProviderItemIdentifier, Data?, Error?) -> Void,
                         completionHandler: @escaping (Error?) -> Void) -> Progress {
        let progress = Progress(totalUnitCount: Int64(itemIdentifiers.count))
        let pixels = Int(min(max(size.width, size.height), 400))
        task(progress) { [self] in
            try await prepare()
            try await withThrowingTaskGroup(of: Void.self) { group in
                var running = 0
                for identifier in itemIdentifiers {
                    if running == 4 { try await group.next(); running -= 1 }
                    running += 1
                    group.addTask { [self] in
                        defer { progress.completedUnitCount += 1 }
                        guard let path = catalog.path(for: identifier), Self.hasThumbnail(path) else {
                            perThumbnailCompletionHandler(identifier, nil, nil)
                            return
                        }
                        do {
                            let data = try await remote.store { try await $0.thumbnail(path: path, size: pixels) }
                            perThumbnailCompletionHandler(identifier, data, nil)
                        } catch {
                            perThumbnailCompletionHandler(identifier, nil, error)
                        }
                    }
                }
                try await group.waitForAll()
            }
            completionHandler(nil)
        } failed: { completionHandler($0) }
        return progress
    }

    private static func hasThumbnail(_ path: String) -> Bool {
        guard let type = UTType(filenameExtension: (path as NSString).pathExtension) else { return false }
        return type.conforms(to: .image) || type.conforms(to: .movie)
    }
}
