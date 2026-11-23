import Foundation

/// The API of a machine waiting to be set up (cli/src/setup_server.rs), on the local network over
/// plain HTTP: the same answers the terminal and the setup page give, applied by the same code.
public actor SetupClient {
    public struct Hello: Decodable, Sendable {
        public let version: String
        public let host: String
        public let claimed: Bool
        public let open: Bool
    }

    public struct Disk: Decodable, Sendable, Identifiable, Hashable {
        public let path: String
        public let sizeText: String
        public let model: String
        public let contents: [String]
        public let busy: String?
        public var id: String { path }
    }

    public struct Facts: Decodable, Sendable {
        public let first: Bool
        public let host: String
        public let disks: [Disk]
        public let pool: String?
        public let tailscale: String?
        public let hasToken: Bool
        public let needsOwner: Bool
    }

    /// The answers, as the API takes them (see `parse_answers`).
    public struct Answers: Codable, Sendable {
        public struct Files: Codable, Sendable {
            public var kind: String
            public var path: String?
            public var disks: [String]?
            public var level: String?
            public var encrypt: Bool?

            public init(kind: String, path: String?, disks: [String]?, level: String?, encrypt: Bool?) {
                self.kind = kind
                self.path = path
                self.disks = disks
                self.level = level
                self.encrypt = encrypt
            }
        }
        public struct Domain: Codable, Sendable {
            public var kind: String
            public var hostname: String?
            public var new: Bool?

            public init(kind: String, hostname: String?, new: Bool?) {
                self.kind = kind
                self.hostname = hostname
                self.new = new
            }
        }
        public struct Owner: Codable, Sendable {
            public var email: String
            public var password: String

            public init(email: String, password: String) {
                self.email = email
                self.password = password
            }
        }
        public var name: String
        public var files: Files
        public var backupDir: String
        public var store: Bool
        public var home: Bool
        public var local: Bool
        /// The network's name at home, `<this>.local` (servers from before it was asked: nil).
        public var localHost: String?
        public var localNames: [String]
        public var tailscale: Bool
        public var domain: Domain
        public var owner: Owner?
        public var overwriteDns: Bool?
    }

    public struct Problem: Decodable, Sendable {
        public let field: String
        public let message: String
    }

    public struct Progress: Decodable, Sendable {
        public struct Line: Decodable, Sendable, Hashable {
            public let kind: String
            public let text: String
        }
        public let state: String
        public let log: [Line]
        public let signIn: String?
        public let recoveryKey: String?
        public let dnsTaken: String?
        public let error: String?
        public let url: String?
        public let others: [String]
    }

    public struct Failure: LocalizedError {
        public let message: String
        public let problems: [Problem]
        public var errorDescription: String? { message }
    }

    private struct ErrorBody: Decodable {
        let detail: String?
        let problems: [Problem]?
    }

    private let base: URL
    private var token: String?
    private let session: URLSession = {
        let config = URLSessionConfiguration.ephemeral
        config.timeoutIntervalForRequest = 15
        return URLSession(configuration: config)
    }()

    public init(base: URL) {
        self.base = base
    }

    public func hello() async throws -> Hello { try await call("GET", "api/setup") }

    public func claim(code: String) async throws {
        struct Body: Encodable { let code: String }
        struct Token: Decodable { let token: String }
        let reply: Token = try await call("POST", "api/setup/claim", body: Body(code: code))
        token = reply.token
    }

    public func facts() async throws -> (Facts, Answers) {
        struct Reply: Decodable { let facts: Facts; let answers: Answers }
        let reply: Reply = try await call("GET", "api/setup/facts")
        return (reply.facts, reply.answers)
    }

    public func check(_ answers: Answers) async throws -> [Problem] {
        struct Reply: Decodable { let problems: [Problem] }
        let reply: Reply = try await call("POST", "api/setup/check", body: answers)
        return reply.problems
    }

    public func apply(_ answers: Answers) async throws {
        struct Reply: Decodable { let state: String }
        let _: Reply = try await call("POST", "api/setup/apply", body: answers)
    }

    public func progress() async throws -> Progress { try await call("GET", "api/setup/progress") }

    private func call<R: Decodable>(_ method: String, _ path: String) async throws -> R {
        try await call(method, path, body: Optional<Answers>.none)
    }

    private func call<R: Decodable, B: Encodable>(_ method: String, _ path: String, body: B?) async throws -> R {
        var request = URLRequest(url: base.appending(path: path))
        request.httpMethod = method
        if let token { request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization") }
        if let body {
            let encoder = JSONEncoder()
            encoder.keyEncodingStrategy = .convertToSnakeCase
            request.httpBody = try encoder.encode(body)
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        }
        let (data, response) = try await session.data(for: request)
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            let err = try? decoder.decode(ErrorBody.self, from: data)
            throw Failure(message: err?.problems?.first?.message ?? err?.detail ?? "Something went wrong. Try again.", problems: err?.problems ?? [])
        }
        return try decoder.decode(R.self, from: data)
    }
}
