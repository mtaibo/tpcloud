import SwiftUI
import TPCore
import TPUI
#if os(iOS)
import UIKit
#endif

/// Which part of the Photos tab is showing, picked from the segmented control in the nav bar.
enum PhotosSection: String, CaseIterable {
    case library, albums, shared

    var title: String {
        switch self {
        case .library: return "Library"
        case .albums: return "Albums"
        case .shared: return "Shared"
        }
    }

    var systemImage: String {
        switch self {
        case .library: return "photo.on.rectangle"
        case .albums: return "rectangle.stack"
        case .shared: return "person.2"
        }
    }
}

public struct PhotoGalleryView: View {
    @ObservedObject var store: PhotoGalleryStore
    @ObservedObject var syncStore: PhotoSyncStore
    let client: PhotosClient
    let ownerEmail: String
    let isAdmin: Bool
    let scrollToBottomSignal: Int

    @Environment(\.openURL) private var openURL
    @State private var section: PhotosSection = .library
    /// The album last picked under Albums; independent of `section` so a trip to Shared and back
    /// to Library keeps it.
    @State private var albumFilter: PhotosClient.Filter = .all
    @State private var detailPhotoID: Int?
    #if os(iOS)
    @State private var selection = PhotoSelection()
    @State private var isConfirmingDelete = false
    @State private var isPreparingShare = false
    @State private var shareURLs: [URL] = []
    @State private var isSharing = false
    #endif
    @State private var isStatusPresented = false
    /// The success message whose ✓ has already been shown for a few seconds and hidden.
    @State private var dismissedMessage: String?

    /// Shared with `PhotoGrid`, which owns pinch-to-zoom; the menu's Zoom In/Out write it too.
    @AppStorage(PhotoGridDensity.storageKey) private var columnLevel = 0

    public init(
        store: PhotoGalleryStore,
        syncStore: PhotoSyncStore,
        client: PhotosClient,
        ownerEmail: String,
        isAdmin: Bool = false,
        scrollToBottomSignal: Int = 0
    ) {
        PhotoGridDensity.applyTestStartLevel()
        self.scrollToBottomSignal = scrollToBottomSignal
        self.store = store
        self.syncStore = syncStore
        self.client = client
        self.ownerEmail = ownerEmail
        self.isAdmin = isAdmin
    }

    private var effectiveFilter: PhotosClient.Filter {
        section == .shared ? .shared : albumFilter
    }

    public var body: some View {
        NavigationStack {
            Group {
                switch section {
                case .albums:
                    AlbumsListView(selected: albumFilter) { filter in
                        albumFilter = filter
                        withAnimation(.snappy) { section = .library }
                    }
                case .library, .shared:
                    galleryContent
                }
            }
            .navigationTitle(section.title)
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            // Select mode swaps the tab bar for its own actions, like Photos.
            .toolbar(selection.isActive ? .hidden : .visible, for: .tabBar)
            .safeAreaInset(edge: .bottom, spacing: 0) {
                if selection.isActive {
                    PhotoSelectionBar(
                        selection: selection,
                        isWorking: isPreparingShare,
                        onShare: shareSelected,
                        onDelete: { isConfirmingDelete = true }
                    )
                    .transition(.move(edge: .bottom).combined(with: .opacity))
                }
            }
            .animation(.snappy, value: selection.isActive)
            .confirmationDialog(
                selection.ids.count == 1 ? "Delete this photo?" : "Delete \(selection.ids.count) photos?",
                isPresented: $isConfirmingDelete,
                titleVisibility: .visible
            ) {
                Button(selection.ids.count == 1 ? "Delete Photo" : "Delete \(selection.ids.count) Photos", role: .destructive) {
                    deleteSelected()
                }
            } message: {
                Text("They'll be removed from your server. The copies on this device aren't affected.")
            }
            .sheet(isPresented: $isSharing) {
                PhotoShareSheet(items: shareURLs)
                    .presentationDetents([.medium, .large])
                    .ignoresSafeArea()
            }
            .onChange(of: section) { selection.isActive = false }
            #endif
            .toolbar { toolbarContent }
        }
        .task {
            store.filter = effectiveFilter
            if store.photos.isEmpty { await store.reload() }
        }
        .onChange(of: effectiveFilter) { _, newValue in
            store.filter = newValue
        }
        #if os(macOS)
        .sheet(item: detailBinding) { selection in
            detail(startingAt: selection.id)
                .frame(minWidth: 720, minHeight: 540)
        }
        #endif
    }

    #if os(macOS)
    private struct DetailSelection: Identifiable {
        let id: Int
    }

    private var detailBinding: Binding<DetailSelection?> {
        Binding(
            get: { detailPhotoID.map(DetailSelection.init) },
            set: { detailPhotoID = $0?.id }
        )
    }

    private func detail(startingAt id: Int) -> some View {
        PhotoDetailView(photos: displayedPhotos, initialPhotoID: id, client: client) { photo in
            Task { await store.delete(photo) }
        }
    }
    #endif

    // MARK: - Toolbar

    @ToolbarContentBuilder
    private var toolbarContent: some ToolbarContent {
        #if os(iOS)
        // Over the grid the title is drawn by the grid itself, large and with the dates under
        // it, like Photos (`PhotoGridDateHeader`); the bar keeps only its buttons.
        if section != .albums {
            ToolbarItem(placement: .principal) { Color.clear.frame(width: 1, height: 1) }
        }
        #endif
        ToolbarItemGroup(placement: .primaryAction) {
            #if os(iOS)
            if selection.isActive {
                menuButton
                Button("Done Selecting", systemImage: "xmark") { selection.isActive = false }
            } else {
                if hasStatus {
                    statusButton
                }
                if section != .albums && !store.photos.isEmpty {
                    Button("Select") { selection.isActive = true }
                }
                menuButton
            }
            #else
            if hasStatus {
                statusButton
            }
            menuButton
            #endif
        }
    }

    #if os(iOS)
    private var selectedPhotos: [Photo] {
        store.photos.reversed().filter { selection.ids.contains($0.id) }
    }

    private func shareSelected() {
        let photos = selectedPhotos
        guard !photos.isEmpty else { return }
        isPreparingShare = true
        Task {
            let urls = await PhotoShareDownloads.originals(of: photos, client: client)
            isPreparingShare = false
            guard !urls.isEmpty else { return }
            shareURLs = urls
            isSharing = true
        }
    }

    private func deleteSelected() {
        let photos = selectedPhotos
        selection.isActive = false
        Task { await store.delete(photos) }
    }
    #endif

    private var isBusy: Bool {
        syncStore.isRunning || syncStore.isSanitizing || syncStore.isChecking
    }

    private var successMessage: String? {
        syncStore.lastSanitizeMessage ?? syncStore.lastCheckMessage
    }

    private var hasStatus: Bool {
        isBusy || syncStore.lastError != nil || (successMessage != nil && successMessage != dismissedMessage)
    }

    private struct StatusDismissKey: Equatable {
        let message: String?
        let isPresented: Bool
        let isBusy: Bool
    }

    /// Sits next to the ••• menu: a ring that fills up while a backup uploads, a spinner for
    /// Check/Sanitize, or the last run's outcome. Tapping it explains the status in full.
    private var statusButton: some View {
        Button {
            isStatusPresented.toggle()
        } label: {
            Group {
                if syncStore.isRunning {
                    BackupProgressRing(fraction: syncStore.progressTotal == 0 ? nil : syncStore.progressFraction)
                } else if isBusy {
                    ProgressView()
                } else if syncStore.lastError != nil {
                    Image(systemName: "exclamationmark.icloud")
                        .foregroundStyle(.red)
                } else {
                    Image(systemName: "checkmark.icloud")
                }
            }
            .frame(width: 24, height: 24)
        }
        .accessibilityLabel("Backup Status")
        .task(id: StatusDismissKey(message: successMessage, isPresented: isStatusPresented, isBusy: isBusy)) {
            // Errors stay until acted on; a plain success ✓ only lingers briefly — and never
            // vanishes while its popover is open or another run is underway.
            if isBusy { dismissedMessage = nil } // a new run's result may repeat the old text
            guard let message = successMessage, syncStore.lastError == nil, !isStatusPresented, !isBusy else { return }
            try? await Task.sleep(for: .seconds(4))
            guard !Task.isCancelled else { return }
            withAnimation { dismissedMessage = message }
        }
        .popover(isPresented: $isStatusPresented) {
            BackupStatusDetail(syncStore: syncStore) { url in openURL(url) }
                .padding()
                .frame(width: 300, alignment: .leading)
                .presentationCompactAdaptation(.popover)
        }
    }

    private var menuButton: some View {
        Menu {
            #if os(iOS)
            if selection.isActive {
                Section {
                    Button("Select All", systemImage: "checkmark.circle") {
                        selection.ids = Set(store.photos.map(\.id))
                    }
                    Button("Deselect All", systemImage: "circle") { selection.ids.removeAll() }
                        .disabled(selection.ids.isEmpty)
                }
            }
            #endif
            // Library, Albums and Shared, in the menu rather than across the top.
            Section {
                Picker("View", selection: $section.animation(.snappy)) {
                    ForEach(PhotosSection.allCases, id: \.self) { item in
                        Label(item.title, systemImage: item.systemImage).tag(item)
                    }
                }
                .pickerStyle(.inline)
            }
            Section {
                Button("Back Up Now", systemImage: "icloud.and.arrow.up") {
                    Task {
                        await syncStore.backupNow(ownerEmail: ownerEmail)
                        await store.reload()
                    }
                }
                Button("Check Library", systemImage: "checkmark.icloud") {
                    Task {
                        await syncStore.checkNow(ownerEmail: ownerEmail)
                        await store.reload()
                    }
                }
                if isAdmin {
                    Button("Sanitize Server", systemImage: "wand.and.stars") {
                        Task { await syncStore.sanitizeNow() }
                    }
                }
            }
            .disabled(isBusy)

            if section != .albums {
                // The pinch gesture's equivalent for VoiceOver, keyboards and Macs without a trackpad.
                Section {
                    Button("Zoom In", systemImage: "plus.magnifyingglass") {
                        withAnimation(.snappy) { columnLevel = max(columnLevel - 1, 0) }
                    }
                    .disabled(columnLevel <= 0)
                    Button("Zoom Out", systemImage: "minus.magnifyingglass") {
                        withAnimation(.snappy) { columnLevel = min(columnLevel + 1, PhotoGridDensity.columnLevels.count - 1) }
                    }
                    .disabled(columnLevel >= PhotoGridDensity.columnLevels.count - 1)
                }
            }
        } label: {
            Label("More", systemImage: "ellipsis")
        }
        .menuIndicator(.hidden)
    }

    // MARK: - Grid

    /// Oldest first, so the newest photo sits at the bottom of the grid like the Photos app.
    /// (The API pages newest-first; this used to be done by flipping the whole scroll view
    /// upside down, which also reversed VoiceOver order and broke the pinch gesture.)
    private var displayedPhotos: [Photo] {
        store.photos.reversed()
    }

    #if os(iOS)
    private var gridSelection: PhotoSelection? { selection }
    #else
    private var gridSelection: PhotoSelection? { nil }
    #endif

    @ViewBuilder
    private var galleryContent: some View {
        if store.isLoading && store.photos.isEmpty {
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let error = store.errorMessage, store.photos.isEmpty {
            ContentUnavailableView {
                Label("Couldn't Load Photos", systemImage: "exclamationmark.triangle")
            } description: {
                Text(error)
            } actions: {
                Button("Try Again") { Task { await store.reload() } }
            }
        } else if store.photos.isEmpty {
            ContentUnavailableView {
                Label(section == .shared ? "No Shared Photos" : "No Photos Yet", systemImage: "photo.on.rectangle")
            } description: {
                if section == .library {
                    Text("Back up this device's library from the ••• menu.")
                }
            }
        } else {
            PhotoGrid(
                photos: store.photos,
                client: client,
                scrollToBottomSignal: scrollToBottomSignal,
                title: section.title,
                selection: gridSelection,
                onSelect: { detailPhotoID = $0.id },
                onDelete: { photo in Task { await store.delete(photo) } }
            )
            .refreshable { await store.reload() }
        }
    }

}

/// The ring in the status button: fills with the backup's progress, or spins while the library is
/// still being scanned and there's no fraction to show yet.
private struct BackupProgressRing: View {
    let fraction: Double?

    var body: some View {
        ZStack {
            Circle().stroke(.secondary.opacity(0.35), lineWidth: 2.5)
            if let fraction {
                Circle()
                    .trim(from: 0, to: fraction)
                    .stroke(.tint, style: StrokeStyle(lineWidth: 2.5, lineCap: .round))
                    .rotationEffect(.degrees(-90))
                    .animation(.easeOut, value: fraction)
            } else {
                ProgressView().controlSize(.mini)
            }
        }
        .padding(2)
    }
}

/// The status popover's contents: what's running, how far along, and what went wrong.
private struct BackupStatusDetail: View {
    @ObservedObject var syncStore: PhotoSyncStore
    let openURL: (URL) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if syncStore.isRunning {
                HStack {
                    Text("Backing Up Photos").font(.headline)
                    Spacer()
                    if syncStore.progressTotal > 0 {
                        Text(syncStore.progressFraction, format: .percent.precision(.fractionLength(0)))
                            .monospacedDigit()
                            .foregroundStyle(.secondary)
                    }
                }
                if syncStore.progressTotal == 0 {
                    ProgressView()
                        .progressViewStyle(.linear)
                    Text("Checking which photos are already backed up…")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                } else {
                    ProgressView(value: syncStore.progressFraction)
                    Text(syncStore.progressLabel)
                        .font(.footnote)
                        .monospacedDigit()
                        .foregroundStyle(.secondary)
                }
                failureList
            } else if syncStore.isSanitizing {
                Label("Reorganizing photos on the server…", systemImage: "wand.and.stars")
            } else if syncStore.isChecking {
                Label("Checking the library…", systemImage: "checkmark.icloud")
            } else if let error = syncStore.lastError {
                Label(error, systemImage: "exclamationmark.triangle.fill")
                    .foregroundStyle(.red)
                #if os(iOS)
                if syncStore.permissionDenied, let url = URL(string: UIApplication.openSettingsURLString) {
                    Button("Open Settings") { openURL(url) }
                }
                #endif
                failureList
            } else if let message = syncStore.lastSanitizeMessage ?? syncStore.lastCheckMessage {
                Label(message, systemImage: "checkmark.circle.fill")
            }
        }
        .font(.subheadline)
        .fixedSize(horizontal: false, vertical: true)
    }

    /// The backup's failures grouped by reason, most frequent first.
    @ViewBuilder
    private var failureList: some View {
        if !syncStore.failureReasons.isEmpty {
            VStack(alignment: .leading, spacing: 4) {
                ForEach(syncStore.failureReasons.sorted { $0.value > $1.value }, id: \.key) { reason, count in
                    HStack(alignment: .firstTextBaseline, spacing: 6) {
                        Text("\(count)")
                            .fontWeight(.semibold)
                            .monospacedDigit()
                            .foregroundStyle(.red)
                        Text(reason)
                            .foregroundStyle(.secondary)
                    }
                    .font(.footnote)
                }
            }
        }
    }
}

/// The server sends ISO-8601, sometimes with fractional seconds.
enum PhotoDates {
    // Built once: creating formatters per call was showing up on every tile's render.
    // ISO8601DateFormatter is thread-safe.
    nonisolated(unsafe) private static let fractional: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }()
    nonisolated(unsafe) private static let plain = ISO8601DateFormatter()

    static func parse(_ string: String) -> Date? {
        fractional.date(from: string) ?? plain.date(from: string)
    }
}
