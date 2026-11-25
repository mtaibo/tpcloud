import FileProvider
import Foundation
import TPCore

/// What a file or folder on the host looks like, as far as this extension compares it.
struct Entry: Codable, Equatable, Sendable {
    var isDirectory: Bool
    var size: Int
    var modified: Double
}

/// The extension's memory of one network, kept in the App Group's folder:
/// - identifiers: the host has only paths, but the system wants an item to keep its identifier
///   when it's renamed or moved, so each path the system has seen gets an id of its own here,
///   carried along by the renames and moves made through the Finder;
/// - the last `/tree` seen, which the next one is compared with to tell the system what changed
///   on the host.
final class Catalog: @unchecked Sendable {
    /// The account's own folder on the host is the domain's root; the shared folder appears in
    /// it as «Shared».
    static let sharedPath = "shared"
    static let sharedIdentifier = NSFileProviderItemIdentifier("shared")

    private struct Saved: Codable {
        var ids: [String: String] = [:]
        var tree: [String: Entry]?
        var generation = 0
    }

    private let lock = NSLock()
    private let file: URL
    private var saved: Saved
    private var paths: [String: String] = [:]
    private var root: String?
    private var saveScheduled = false

    init(domain: NSFileProviderDomainIdentifier) {
        let safe = domain.rawValue.replacingOccurrences(of: "/", with: "_").replacingOccurrences(of: ":", with: "_")
        let folder = TPShared.container.appending(path: "FileProvider/\(safe)", directoryHint: .isDirectory)
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        file = folder.appending(path: "catalog.json")
        saved = (try? JSONDecoder().decode(Saved.self, from: Data(contentsOf: file))) ?? Saved()
        for (path, id) in saved.ids { paths[id] = path }
    }

    /// The shared folder's date, from the last `/tree` (a listing never includes it: it's at the top).
    var sharedModified: Double {
        get { lock.withLock { _sharedModified } }
        set { lock.withLock { _sharedModified = newValue } }
    }
    private var _sharedModified = Date().timeIntervalSince1970

    /// The shared folder's date as last told to the system.
    var reportedShared: Double? {
        get { lock.withLock { _reportedShared } }
        set { lock.withLock { _reportedShared = newValue } }
    }
    private var _reportedShared: Double?

    /// Folders' dates from the last `/tree`, to show on folders reported as changes.
    var folderDates: [String: Double] {
        get { lock.withLock { _folderDates } }
        set { lock.withLock { _folderDates = newValue } }
    }
    private var _folderDates: [String: Double] = [:]

    func setRoot(_ path: String) {
        lock.withLock { root = path }
    }

    // MARK: Identifiers

    func identifier(for path: String) -> NSFileProviderItemIdentifier {
        lock.withLock {
            if path == root { return .rootContainer }
            if path == Self.sharedPath { return Self.sharedIdentifier }
            if let id = saved.ids[path] { return NSFileProviderItemIdentifier(id) }
            let id = UUID().uuidString
            saved.ids[path] = id
            paths[id] = path
            scheduleSave()
            return NSFileProviderItemIdentifier(id)
        }
    }

    func path(for identifier: NSFileProviderItemIdentifier) -> String? {
        lock.withLock {
            switch identifier {
            case .rootContainer: root
            case Self.sharedIdentifier: Self.sharedPath
            case .trashContainer, .workingSet: nil
            default: paths[identifier.rawValue]
            }
        }
    }

    func parentIdentifier(for path: String) -> NSFileProviderItemIdentifier {
        identifier(for: Self.parent(of: path))
    }

    /// A path and everything under it moved (a rename or a move): the ids go with them.
    func moved(from old: String, to new: String) {
        lock.withLock {
            for (path, id) in saved.ids where path == old || path.hasPrefix(old + "/") {
                let renamed = new + path.dropFirst(old.count)
                saved.ids[path] = nil
                saved.ids[renamed] = id
                paths[id] = renamed
            }
            if var tree = saved.tree {
                for (path, entry) in tree where path == old || path.hasPrefix(old + "/") {
                    tree[path] = nil
                    tree[new + path.dropFirst(old.count)] = entry
                }
                saved.tree = tree
            }
            scheduleSave()
        }
    }

    /// Gone from the host: the path and everything under it.
    func removed(_ old: String) {
        lock.withLock {
            for (path, id) in saved.ids where path == old || path.hasPrefix(old + "/") {
                saved.ids[path] = nil
                paths[id] = nil
            }
            saved.tree = saved.tree?.filter { $0.key != old && !$0.key.hasPrefix(old + "/") }
            scheduleSave()
        }
    }

    /// Something this extension just did on the host, so the next comparison doesn't report it back.
    func noted(_ path: String, _ entry: Entry) {
        lock.withLock {
            saved.tree?[path] = entry
            scheduleSave()
        }
    }

    // MARK: Changes

    var generation: Int { lock.withLock { saved.generation } }

    /// The host's whole tree now, compared with the last one: what's new or different, and the
    /// identifiers of what's gone. The first time there's nothing to compare with, so nothing is
    /// reported (the Finder lists folders as they're opened).
    func compare(with tree: [String: Entry]) -> (changed: [(String, Entry)], deleted: [NSFileProviderItemIdentifier]) {
        lock.withLock {
            defer {
                saved.tree = tree
                saved.generation += 1
                scheduleSave()
            }
            guard let old = saved.tree else { return ([], []) }
            let changed = tree.filter { old[$0.key] != $0.value }.map { ($0.key, $0.value) }
            var deleted: [NSFileProviderItemIdentifier] = []
            for path in old.keys where tree[path] == nil {
                if let id = saved.ids.removeValue(forKey: path) {
                    paths[id] = nil
                    deleted.append(NSFileProviderItemIdentifier(id))
                }
            }
            return (changed, deleted)
        }
    }

    static func parent(of path: String) -> String {
        guard let slash = path.lastIndex(of: "/") else { return "" }
        return String(path[..<slash])
    }

    static func name(of path: String) -> String {
        guard let slash = path.lastIndex(of: "/") else { return path }
        return String(path[path.index(after: slash)...])
    }

    // Must be called with the lock held.
    private func scheduleSave() {
        guard !saveScheduled else { return }
        saveScheduled = true
        DispatchQueue.global(qos: .utility).asyncAfter(deadline: .now() + 1) { [weak self] in
            self?.save()
        }
    }

    private func save() {
        let data = lock.withLock {
            saveScheduled = false
            return try? JSONEncoder().encode(saved)
        }
        try? data?.write(to: file, options: .atomic)
    }
}
