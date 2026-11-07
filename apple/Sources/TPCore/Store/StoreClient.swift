import Foundation

public struct StoreClient: Sendable {
    private let client: HTTPClient

    public init(client: HTTPClient) {
        self.client = client
    }

    // MARK: - Browsing

    public func listDirectory(path: String = "", location: StoreLocation = .external) async throws -> DirectoryListing {
        try await client.get("/api/files/list", query: ["path": path, "location": location.rawValue])
    }

    public func diskInfo() async throws -> DiskInfo {
        try await client.get("/api/files/disk")
    }

    // MARK: - File operations

    public func createFolder(path: String, location: StoreLocation = .external) async throws {
        let _: [String: String] = try await client.post("/api/files/mkdir", body: PathBody(path: path, location: location.rawValue))
    }

    public func rename(path: String, newName: String, location: StoreLocation = .external) async throws {
        let _: [String: String] = try await client.post(
            "/api/files/rename",
            body: RenameBody(path: path, newName: newName, location: location.rawValue)
        )
    }

    public func move(path: String, destDir: String, location: StoreLocation = .external) async throws {
        let _: [String: String] = try await client.post(
            "/api/files/move",
            body: MoveBody(path: path, destDir: destDir, location: location.rawValue)
        )
    }

    public func delete(path: String, location: StoreLocation = .external) async throws {
        let _: [String: String] = try await client.delete("/api/files", query: ["path": path, "location": location.rawValue])
    }

    /// A folder's custom icon, by the web frontend's icon name (e.g. "Briefcase"), so both
    /// clients show the same one. `nil` goes back to the default folder.
    public func setFolderIcon(path: String, location: StoreLocation = .external, iconName: String?) async throws {
        if let iconName {
            let _: [String: Bool] = try await client.put(
                "/api/files/icon",
                body: IconBody(path: path, location: location.rawValue, iconName: iconName)
            )
        } else {
            let _: [String: Bool] = try await client.delete("/api/files/icon", query: ["path": path, "location": location.rawValue])
        }
    }

    // MARK: - Download / Upload

    public func download(path: String, location: StoreLocation = .external) async throws -> Data {
        try await client.getData("/api/files/download", query: ["path": path, "location": location.rawValue])
    }

    public func downloadZip(path: String, location: StoreLocation = .external) async throws -> Data {
        try await client.getData("/api/files/download-zip", query: ["path": path, "location": location.rawValue])
    }

    /// A file (or, for a folder, a zip of it) streamed to a temporary file the caller must move
    /// or delete, reporting into `progress` as it goes.
    public func downloadFile(path: String, location: StoreLocation = .external, asZip: Bool = false, progress: Progress? = nil) async throws -> URL {
        try await client.downloadFile(
            asZip ? "/api/files/download-zip" : "/api/files/download",
            query: ["path": path, "location": location.rawValue],
            progress: progress
        )
    }

    public func thumbnailURL(path: String, location: StoreLocation = .external, size: Int = 160) -> URL {
        var components = URLComponents(url: client.baseURL.appending(path: "/api/files/thumbnail"), resolvingAgainstBaseURL: false)!
        components.queryItems = [
            URLQueryItem(name: "path", value: path),
            URLQueryItem(name: "location", value: location.rawValue),
            URLQueryItem(name: "size", value: String(size))
        ]
        return components.url!
    }

    public func upload(path: String, location: StoreLocation = .external, filename: String, data: Data, mimeType: String = "application/octet-stream") async throws {
        let _: [String: [String]] = try await client.upload(
            "/api/files/upload",
            query: ["path": path, "location": location.rawValue],
            files: [HTTPClient.UploadFile(filename: filename, mimeType: mimeType, data: data)]
        )
    }

    /// Same size as the web frontend's chunks. Each one is its own request, so no request comes near
    /// Cloudflare's 100 MB body limit, which a big video in a single `upload` request went over.
    public static let uploadChunkSize = 32 * 1024 * 1024

    /// Uploads a file from disk in chunks through `/upload-chunk`, reading one chunk at a time
    /// instead of the whole file into memory, and retrying a chunk that fails on the way (the
    /// server writes each one at its own offset, so sending it twice is harmless). `progress`
    /// counts the file's bytes as they're sent.
    @concurrent
    public func uploadFile(
        at fileURL: URL,
        to path: String,
        location: StoreLocation = .external,
        filename: String,
        progress: Progress? = nil
    ) async throws {
        let fileSize = try fileURL.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
        // `/upload-chunk` turns down an empty body, so empty files go the plain way.
        guard fileSize > 0 else {
            try await upload(path: path, location: location, filename: filename, data: Data())
            progress?.completedUnitCount = progress?.totalUnitCount ?? 0
            return
        }

        let chunkSize = Self.uploadChunkSize
        let totalChunks = (fileSize + chunkSize - 1) / chunkSize
        let uploadID = UUID().uuidString
        progress?.totalUnitCount = Int64(fileSize)

        let handle = try FileHandle(forReadingFrom: fileURL)
        defer { try? handle.close() }

        do {
            try await uploadChunks(from: handle, uploadID: uploadID, totalChunks: totalChunks, fileSize: fileSize,
                                   to: path, location: location, filename: filename, progress: progress)
        } catch {
            // Chunks that did land would otherwise linger in the folder as an upload that never
            // finishes. Detached, because a cancelled upload would cancel this request with it.
            if totalChunks > 1 {
                Task.detached { [client] in
                    let _: [String: String]? = try? await client.delete("/api/files/cancel-upload/\(uploadID)")
                }
            }
            throw error
        }
        progress?.completedUnitCount = Int64(fileSize)
    }

    private func uploadChunks(
        from handle: FileHandle,
        uploadID: String,
        totalChunks: Int,
        fileSize: Int,
        to path: String,
        location: StoreLocation,
        filename: String,
        progress: Progress?
    ) async throws {
        let chunkSize = Self.uploadChunkSize
        for index in 0..<totalChunks {
            try Task.checkCancellation()
            try handle.seek(toOffset: UInt64(index * chunkSize))
            guard let chunk = try handle.read(upToCount: chunkSize), !chunk.isEmpty else {
                throw APIError.transport("The file changed while it was being uploaded")
            }
            let query = [
                "upload_id": uploadID,
                "chunk_index": String(index),
                "total_chunks": String(totalChunks),
                "chunk_size": String(chunkSize),
                "filename": filename,
                "file_size": String(fileSize),
                "path": path,
                "location": location.rawValue
            ]
            let sentBefore = Int64(index * chunkSize)
            let chunkProgress = Progress(totalUnitCount: Int64(chunk.count))
            let observation = chunkProgress.observe(\.completedUnitCount) { [weak progress] chunkProgress, _ in
                progress?.completedUnitCount = sentBefore + chunkProgress.completedUnitCount
            }
            defer { observation.invalidate() }
            try await Self.retrying {
                let _: ChunkResponse = try await client.postData("/api/files/upload-chunk", query: query, body: chunk, progress: chunkProgress)
            }
        }
    }

    /// Up to 4 tries with a growing pause in between, for the errors a flaky connection or a busy
    /// server give; a refused request (4xx) or an expired session fails straight away.
    private static func retrying(_ operation: () async throws -> Void) async throws {
        var attempt = 0
        while true {
            do {
                try await operation()
                return
            } catch let error as APIError {
                attempt += 1
                let isTransient: Bool
                switch error {
                case .transport: isTransient = true
                case .server(let status, _): isTransient = status >= 500 || status == 408 || status == 429
                default: isTransient = false
                }
                guard isTransient, attempt < 4 else { throw error }
                try await Task.sleep(for: .seconds(1 << (attempt - 1)))
            }
        }
    }

    // MARK: - Shares

    public func shares(path: String? = nil) async throws -> [ShareLink] {
        var query: [String: String] = [:]
        if let path { query["path"] = path }
        return try await client.get("/api/shares", query: query)
    }

    public func createShare(
        location: StoreLocation,
        path: String,
        hidden: Bool = false,
        slug: String? = nil,
        password: String? = nil,
        editable: Bool = false,
        isPublic: Bool = false
    ) async throws -> ShareLink {
        try await client.post(
            "/api/shares",
            body: CreateShareBody(
                location: location.rawValue,
                path: path,
                hidden: hidden,
                slug: slug,
                password: password,
                editable: editable,
                isPublic: isPublic
            )
        )
    }

    public func updateShare(
        token: String,
        password: String? = nil,
        clearPassword: Bool = false,
        editable: Bool? = nil,
        isPublic: Bool? = nil
    ) async throws -> ShareLink {
        try await client.patch(
            "/api/shares/\(token)",
            body: UpdateShareBody(password: password, clearPassword: clearPassword, editable: editable, isPublic: isPublic)
        )
    }

    public func deleteShare(_ token: String) async throws {
        let _: [String: String] = try await client.delete("/api/shares/\(token)")
    }
}

private struct ChunkResponse: Decodable {
    let status: String
}

private struct PathBody: Encodable {
    let path: String
    let location: String
}

private struct IconBody: Encodable {
    let path: String
    let location: String
    let iconName: String
}

private struct RenameBody: Encodable {
    let path: String
    let newName: String
    let location: String
}

private struct MoveBody: Encodable {
    let path: String
    let destDir: String
    let location: String
}

private struct CreateShareBody: Encodable {
    let location: String
    let path: String
    let hidden: Bool
    let slug: String?
    let password: String?
    let editable: Bool
    let isPublic: Bool

    // `.convertToSnakeCase` would otherwise turn `isPublic` into "is_public", but the backend expects "public".
    enum CodingKeys: String, CodingKey {
        case location, path, hidden, slug, password, editable
        case isPublic = "public"
    }
}

private struct UpdateShareBody: Encodable {
    let password: String?
    let clearPassword: Bool
    let editable: Bool?
    let isPublic: Bool?

    enum CodingKeys: String, CodingKey {
        case password, clearPassword, editable
        case isPublic = "public"
    }
}
