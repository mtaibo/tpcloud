import FileProvider
import UniformTypeIdentifiers

/// A file or folder on the host, as the Finder shows it.
final class Item: NSObject, NSFileProviderItem {
    let itemIdentifier: NSFileProviderItemIdentifier
    let parentItemIdentifier: NSFileProviderItemIdentifier
    let filename: String
    let entry: Entry
    /// The host's path (what the API takes).
    let path: String

    init(path: String, entry: Entry, catalog: Catalog) {
        self.path = path
        var entry = entry
        if entry.isDirectory, entry.modified == 0 {
            entry.modified = path == Catalog.sharedPath ? catalog.sharedModified : catalog.folderDates[path] ?? 0
        }
        self.entry = entry
        itemIdentifier = catalog.identifier(for: path)
        switch itemIdentifier {
        case .rootContainer:
            parentItemIdentifier = .rootContainer
            filename = "TPCloud"
        case Catalog.sharedIdentifier:
            parentItemIdentifier = .rootContainer
            filename = "Shared"
        default:
            parentItemIdentifier = catalog.parentIdentifier(for: path)
            filename = Catalog.name(of: path)
        }
    }

    var contentType: UTType {
        if entry.isDirectory { return .folder }
        return UTType(filenameExtension: (filename as NSString).pathExtension) ?? .data
    }

    var documentSize: NSNumber? { entry.isDirectory ? nil : NSNumber(value: entry.size) }

    /// `/tree` gives folders no date (0): the Finder then shows none rather than 1970.
    var contentModificationDate: Date? { entry.modified > 0 ? Date(timeIntervalSince1970: entry.modified) : nil }

    var creationDate: Date? { contentModificationDate }

    var capabilities: NSFileProviderItemCapabilities {
        switch itemIdentifier {
        // The account's folder and the shared one stay where they are.
        case .rootContainer, Catalog.sharedIdentifier:
            [.allowsReading, .allowsContentEnumerating, .allowsAddingSubItems]
        default:
            entry.isDirectory
                ? [.allowsReading, .allowsContentEnumerating, .allowsAddingSubItems, .allowsRenaming, .allowsReparenting, .allowsDeleting]
                : [.allowsReading, .allowsWriting, .allowsRenaming, .allowsReparenting, .allowsDeleting]
        }
    }

    /// Size and date, like the Linux copy compares: the same on both sides means the same file.
    /// A folder has no contents of its own: its version is where it is and its date.
    var itemVersion: NSFileProviderItemVersion {
        if entry.isDirectory {
            return NSFileProviderItemVersion(contentVersion: Data("folder".utf8), metadataVersion: Data("\(path)-\(Int(entry.modified))".utf8))
        }
        let content = Data("\(entry.size)-\(entry.modified)".utf8)
        let metadata = Data("\(path)-\(entry.size)-\(entry.modified)".utf8)
        return NSFileProviderItemVersion(contentVersion: content, metadataVersion: metadata)
    }
}
