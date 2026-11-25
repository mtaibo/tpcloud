import FileProvider
import TPCore

/// A folder's contents, from `/api/files/list`. The account's folder also holds «Shared».
final class FolderEnumerator: NSObject, NSFileProviderEnumerator {
    private let identifier: NSFileProviderItemIdentifier
    private let remote: Remote
    private let catalog: Catalog

    init(identifier: NSFileProviderItemIdentifier, remote: Remote, catalog: Catalog) {
        self.identifier = identifier
        self.remote = remote
        self.catalog = catalog
    }

    func invalidate() {}

    func enumerateItems(for observer: NSFileProviderEnumerationObserver, startingAt page: NSFileProviderPage) {
        let identifier = identifier
        Task { [remote, catalog] in
            do {
                let root = try await remote.rootPath()
                catalog.setRoot(root)
                guard let path = catalog.path(for: identifier) else { throw NSFileProviderError(.noSuchItem) }
                let listing = try await remote.store { try await $0.listDirectory(path: path) }
                var items: [NSFileProviderItem] = listing.entries.compactMap { entry in
                    guard entry.type == .file || entry.type == .directory else { return nil }
                    let child = Entry(isDirectory: entry.isDirectory, size: entry.size ?? 0, modified: entry.modified)
                    return Item(path: "\(path)/\(entry.name)", entry: child, catalog: catalog)
                }
                if identifier == .rootContainer {
                    items.append(Item(path: Catalog.sharedPath, entry: Entry(isDirectory: true, size: 0, modified: catalog.sharedModified), catalog: catalog))
                }
                observer.didEnumerate(items)
                observer.finishEnumerating(upTo: nil)
            } catch {
                observer.finishEnumeratingWithError(error)
            }
        }
    }

    // On the Mac the system asks only the working set for changes.
    func enumerateChanges(for observer: NSFileProviderChangeObserver, from anchor: NSFileProviderSyncAnchor) {
        observer.finishEnumeratingChanges(upTo: currentAnchor, moreComing: false)
    }

    func currentSyncAnchor(completionHandler: @escaping (NSFileProviderSyncAnchor?) -> Void) {
        completionHandler(currentAnchor)
    }

    private var currentAnchor: NSFileProviderSyncAnchor { WorkingSetEnumerator.anchor(catalog.generation) }
}

/// What changed on the host since the last look, by comparing its whole tree (the account's
/// folder and the shared one) with the one seen before. Asked whenever the app or the extension
/// signals the working set.
final class WorkingSetEnumerator: NSObject, NSFileProviderEnumerator {
    private let remote: Remote
    private let catalog: Catalog

    init(remote: Remote, catalog: Catalog) {
        self.remote = remote
        self.catalog = catalog
    }

    static func anchor(_ generation: Int) -> NSFileProviderSyncAnchor {
        NSFileProviderSyncAnchor(Data(String(generation).utf8))
    }

    func invalidate() {}

    /// Folders are listed as they're opened; the working set starts empty and only reports changes.
    func enumerateItems(for observer: NSFileProviderEnumerationObserver, startingAt page: NSFileProviderPage) {
        Task { [remote, catalog] in
            _ = try? await Self.look(remote: remote, catalog: catalog)
            observer.finishEnumerating(upTo: nil)
        }
    }

    func enumerateChanges(for observer: NSFileProviderChangeObserver, from anchor: NSFileProviderSyncAnchor) {
        Task { [remote, catalog] in
            do {
                let (changed, deleted) = try await Self.look(remote: remote, catalog: catalog)
                if !deleted.isEmpty { observer.didDeleteItems(withIdentifiers: deleted) }
                // In pages, so a big change (a folder of thousands added) doesn't arrive as one message.
                var items = changed.map { Item(path: $0.0, entry: $0.1, catalog: catalog) }
                // «Shared» isn't in either tree: its date goes out when it changes.
                if catalog.reportedShared != catalog.sharedModified {
                    catalog.reportedShared = catalog.sharedModified
                    items.append(Item(path: Catalog.sharedPath, entry: Entry(isDirectory: true, size: 0, modified: catalog.sharedModified), catalog: catalog))
                }
                for start in stride(from: 0, to: items.count, by: 500) {
                    observer.didUpdate(Array(items[start..<min(start + 500, items.count)]))
                }
                observer.finishEnumeratingChanges(upTo: Self.anchor(catalog.generation), moreComing: false)
            } catch {
                observer.finishEnumeratingWithError(error)
            }
        }
    }

    func currentSyncAnchor(completionHandler: @escaping (NSFileProviderSyncAnchor?) -> Void) {
        completionHandler(Self.anchor(catalog.generation))
    }

    private static func look(remote: Remote, catalog: Catalog) async throws -> (changed: [(String, Entry)], deleted: [NSFileProviderItemIdentifier]) {
        let root = try await remote.rootPath()
        catalog.setRoot(root)
        async let mine = remote.store { try await $0.tree(path: root) }
        async let shared = remote.store { try await $0.tree(path: Catalog.sharedPath) }
        var tree: [String: Entry] = [:]
        let sharedTree = try await shared
        let mineTree = try await mine
        if let modified = sharedTree.modified { catalog.sharedModified = modified }
        if let modified = mineTree.modified { catalog.folderDates[root] = modified }
        for (prefix, part) in [(root, mineTree), (Catalog.sharedPath, sharedTree)] {
            // Folders' dates aren't compared (they move whenever something inside does), only shown.
            for dir in part.dirs {
                tree["\(prefix)/\(dir)"] = Entry(isDirectory: true, size: 0, modified: 0)
                if let modified = part.dirModified?[dir] { catalog.folderDates["\(prefix)/\(dir)"] = modified }
            }
            for file in part.files { tree["\(prefix)/\(file.path)"] = Entry(isDirectory: false, size: file.size, modified: file.modified) }
        }
        return catalog.compare(with: tree)
    }
}

/// Nothing is ever in the trash: the host deletes for good.
final class EmptyEnumerator: NSObject, NSFileProviderEnumerator {
    func invalidate() {}

    func enumerateItems(for observer: NSFileProviderEnumerationObserver, startingAt page: NSFileProviderPage) {
        observer.finishEnumerating(upTo: nil)
    }

    func enumerateChanges(for observer: NSFileProviderChangeObserver, from anchor: NSFileProviderSyncAnchor) {
        observer.finishEnumeratingChanges(upTo: anchor, moreComing: false)
    }

    func currentSyncAnchor(completionHandler: @escaping (NSFileProviderSyncAnchor?) -> Void) {
        completionHandler(NSFileProviderSyncAnchor(Data("0".utf8)))
    }
}
