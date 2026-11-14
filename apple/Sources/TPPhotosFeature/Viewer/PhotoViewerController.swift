#if os(iOS)
import SwiftUI
import TPCore
import UIKit

/// The full-screen photo viewer on iPhone and iPad, built like Photos': opened with the system
/// zoom transition from the tapped tile and closed back into it (or by swiping the photo down),
/// pages that swipe with a gap between photos, and a native scroll-view zoom on each.
///
/// It replaced a SwiftUI `fullScreenCover` that slid up from the bottom, paged a `LazyHStack`,
/// zoomed with `scaleEffect`, and downloaded every original whole before showing it sharp.
final class PhotoViewerController: UIViewController, UICollectionViewDataSource, UICollectionViewDelegate,
    UICollectionViewDataSourcePrefetching {
    /// Oldest first, like the grid.
    private(set) var photos: [Photo]
    private(set) var index: Int
    private let client: PhotosClient
    private let onDelete: (Photo) -> Void
    /// Once the viewer has closed (its transition finished).
    var onDismissed: (() -> Void)?

    private let layout = UICollectionViewFlowLayout()
    private lazy var collectionView = UICollectionView(frame: .zero, collectionViewLayout: layout)
    private var laidOutSize: CGSize = .zero
    private var chromeHidden = false
    private lazy var trashItem = UIBarButtonItem(
        title: "Delete",
        image: UIImage(systemName: "trash"),
        primaryAction: UIAction { [weak self] _ in self?.confirmDelete() }
    )

    /// The space between two photos while swiping, as in Photos.
    private static let gap: CGFloat = 20

    init(photos: [Photo], index: Int, client: PhotosClient, onDelete: @escaping (Photo) -> Void) {
        self.photos = photos
        self.index = min(max(index, 0), max(photos.count - 1, 0))
        self.client = client
        self.onDelete = onDelete
        super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    var currentPhoto: Photo? {
        photos.indices.contains(index) ? photos[index] : nil
    }

    private var currentCell: PhotoPageCell? {
        collectionView.cellForItem(at: IndexPath(item: index, section: 0)) as? PhotoPageCell
    }

    /// The photo's frame on screen, for the zoom transition to start from and land on.
    func currentPhotoFrame(in view: UIView) -> CGRect? {
        guard let cell = currentCell else { return nil }
        return cell.zoomView.photoFrame(in: view)
    }

    /// Swiping down to close only makes sense on the whole photo; zoomed in it pans.
    var allowsInteractiveDismiss: Bool {
        !(currentCell?.zoomView.isZoomedIn ?? false)
    }

    // MARK: Lifecycle

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .black

        layout.scrollDirection = .horizontal
        layout.minimumLineSpacing = Self.gap
        layout.minimumInteritemSpacing = 0
        layout.sectionInset = UIEdgeInsets(top: 0, left: Self.gap / 2, bottom: 0, right: Self.gap / 2)
        collectionView.backgroundColor = .clear
        collectionView.isPagingEnabled = true
        collectionView.showsHorizontalScrollIndicator = false
        collectionView.contentInsetAdjustmentBehavior = .never
        collectionView.register(PhotoPageCell.self, forCellWithReuseIdentifier: PhotoPageCell.reuseIdentifier)
        collectionView.dataSource = self
        collectionView.delegate = self
        collectionView.prefetchDataSource = self
        view.addSubview(collectionView)

        let doubleTap = UITapGestureRecognizer(target: self, action: #selector(handleDoubleTap(_:)))
        doubleTap.numberOfTapsRequired = 2
        let singleTap = UITapGestureRecognizer(target: self, action: #selector(handleSingleTap(_:)))
        singleTap.require(toFail: doubleTap)
        collectionView.addGestureRecognizer(doubleTap)
        collectionView.addGestureRecognizer(singleTap)

        navigationItem.leftBarButtonItem = UIBarButtonItem(
            systemItem: .close,
            primaryAction: UIAction { [weak self] _ in self?.dismiss(animated: true) }
        )
        let infoItem = UIBarButtonItem(
            title: "Info",
            image: UIImage(systemName: "info.circle"),
            primaryAction: UIAction { [weak self] _ in self?.showInfo() }
        )
        toolbarItems = [infoItem, .flexibleSpace(), trashItem]
        updateTitle()
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        let size = view.bounds.size
        guard size != laidOutSize, size.width > 0 else { return }
        laidOutSize = size
        // Each page is the screen plus the gap; the collection view overhangs both edges by half
        // of it, so a resting photo fills the screen exactly.
        collectionView.frame = CGRect(x: -Self.gap / 2, y: 0, width: size.width + Self.gap, height: size.height)
        layout.itemSize = size
        layout.invalidateLayout()
        collectionView.layoutIfNeeded()
        collectionView.contentOffset = CGPoint(x: CGFloat(index) * (size.width + Self.gap), y: 0)
        collectionView.layoutIfNeeded()
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        PhotoGridHitchMonitor.current?.viewerAppeared()
        if let photo = currentPhoto, PhotoViewerImageLoader.shared.cachedImage(for: photo, kind: .preview) != nil {
            PhotoGridHitchMonitor.current?.sharpPhotoShown()
        }
        prefetchNeighbours()
    }

    override func viewDidDisappear(_ animated: Bool) {
        super.viewDidDisappear(animated)
        if navigationController?.isBeingDismissed == true || isBeingDismissed { onDismissed?() }
    }

    override var prefersStatusBarHidden: Bool {
        chromeHidden
    }

    override var preferredStatusBarUpdateAnimation: UIStatusBarAnimation {
        .fade
    }

    // MARK: Chrome

    private func updateTitle() {
        guard let date = currentPhoto?.takenAt.flatMap(PhotoDates.parse) else {
            navigationItem.title = nil
            navigationItem.subtitle = nil
            return
        }
        navigationItem.title = date.formatted(.dateTime.day().month(.wide).year())
        navigationItem.subtitle = date.formatted(date: .omitted, time: .shortened)
    }

    private func setChromeHidden(_ hidden: Bool) {
        guard hidden != chromeHidden else { return }
        chromeHidden = hidden
        navigationController?.setNavigationBarHidden(hidden, animated: true)
        navigationController?.setToolbarHidden(hidden, animated: true)
        UIView.animate(withDuration: 0.25) { self.setNeedsStatusBarAppearanceUpdate() }
    }

    @objc private func handleSingleTap(_ tap: UITapGestureRecognizer) {
        setChromeHidden(!chromeHidden)
    }

    @objc private func handleDoubleTap(_ tap: UITapGestureRecognizer) {
        guard let cell = currentCell else { return }
        cell.zoomView.toggleZoom(at: tap.location(in: cell.zoomView.imageView))
    }

    // MARK: Info and delete

    private func showInfo() {
        guard let photo = currentPhoto else { return }
        let host = UIHostingController(rootView: PhotoInfoView(photo: photo))
        if let sheet = host.sheetPresentationController {
            sheet.detents = [.medium(), .large()]
            sheet.prefersGrabberVisible = true
        }
        present(host, animated: true)
    }

    private func confirmDelete() {
        guard let photo = currentPhoto else { return }
        let alert = UIAlertController(
            title: "Delete this photo?",
            message: "It will be removed from your server. The copy on this device isn't affected.",
            preferredStyle: .actionSheet
        )
        alert.addAction(UIAlertAction(title: "Delete Photo", style: .destructive) { [weak self] _ in self?.delete(photo) })
        alert.addAction(UIAlertAction(title: "Cancel", style: .cancel))
        alert.popoverPresentationController?.barButtonItem = trashItem
        present(alert, animated: true)
    }

    private func delete(_ photo: Photo) {
        guard let position = photos.firstIndex(of: photo) else { return }
        onDelete(photo)
        guard photos.count > 1 else {
            photos.removeAll()
            dismiss(animated: true)
            return
        }
        // Land on the next photo (the previous one at the end), like Photos.
        photos.remove(at: position)
        index = min(position, photos.count - 1)
        collectionView.performBatchUpdates {
            collectionView.deleteItems(at: [IndexPath(item: position, section: 0)])
        } completion: { [weak self] _ in
            guard let self else { return }
            self.collectionView.setContentOffset(CGPoint(x: CGFloat(self.index) * self.collectionView.bounds.width, y: 0), animated: false)
            self.updateTitle()
            self.prefetchNeighbours()
        }
    }

    // MARK: Paging

    func scrollViewDidScroll(_ scrollView: UIScrollView) {
        guard scrollView === collectionView, scrollView.bounds.width > 0, !photos.isEmpty else { return }
        let page = Int((scrollView.contentOffset.x / scrollView.bounds.width).rounded())
        let clamped = min(max(page, 0), photos.count - 1)
        guard clamped != index else { return }
        index = clamped
        updateTitle()
    }

    func scrollViewDidEndDecelerating(_ scrollView: UIScrollView) {
        prefetchNeighbours()
    }

    private func prefetchNeighbours() {
        let around = [index + 1, index - 1, index + 2, index - 2].filter(photos.indices.contains)
        PhotoViewerImageLoader.shared.prefetch(around.map { photos[$0] }, client: client)
    }

    // MARK: UICollectionViewDataSource

    func collectionView(_ collectionView: UICollectionView, numberOfItemsInSection section: Int) -> Int {
        photos.count
    }

    func collectionView(_ collectionView: UICollectionView, cellForItemAt indexPath: IndexPath) -> UICollectionViewCell {
        let cell = collectionView.dequeueReusableCell(withReuseIdentifier: PhotoPageCell.reuseIdentifier, for: indexPath)
        (cell as? PhotoPageCell)?.configure(with: photos[indexPath.item], client: client)
        return cell
    }

    func collectionView(_ collectionView: UICollectionView, prefetchItemsAt indexPaths: [IndexPath]) {
        let items = indexPaths.map(\.item).filter(photos.indices.contains)
        PhotoViewerImageLoader.shared.prefetch(items.map { photos[$0] }, client: client)
    }

    /// A photo swiped away comes back whole, as in Photos.
    func collectionView(_ collectionView: UICollectionView, didEndDisplaying cell: UICollectionViewCell, forItemAt indexPath: IndexPath) {
        (cell as? PhotoPageCell)?.zoomView.reset()
    }
}

/// Lets the viewer, not the navigation controller, decide whether the status bar shows.
final class PhotoViewerNavigationController: UINavigationController {
    override var childForStatusBarHidden: UIViewController? {
        topViewController
    }

    override var childForStatusBarStyle: UIViewController? {
        topViewController
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        overrideUserInterfaceStyle = .dark
        isToolbarHidden = false
    }
}
#endif
