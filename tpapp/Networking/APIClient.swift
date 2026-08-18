import Foundation

enum APIError: LocalizedError {
    case unauthorized
    case notFound
    case serverError(Int)
    case decodingError(Error)
    case networkError(Error)

    var errorDescription: String? {
        switch self {
        case .unauthorized: return "No autenticado"
        case .notFound: return "No encontrado"
        case .serverError(let code): return "Error del servidor (\(code))"
        case .decodingError(let e): return "Error de datos: \(e.localizedDescription)"
        case .networkError(let e): return "Error de red: \(e.localizedDescription)"
        }
    }
}

class APIClient {
    static let shared = APIClient()

    let session: URLSession = {
        let config = URLSessionConfiguration.default
        config.httpCookieStorage = HTTPCookieStorage.shared
        config.httpShouldSetCookies = true
        config.httpCookieAcceptPolicy = .always
        return URLSession(configuration: config)
    }()

    private init() {}

    func get<T: Decodable>(_ url: URL) async throws -> T {
        var req = URLRequest(url: url)
        req.httpMethod = "GET"
        return try await perform(req)
    }

    func post<T: Decodable>(_ url: URL, body: some Encodable) async throws -> T {
        var req = URLRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = try JSONEncoder().encode(body)
        return try await perform(req)
    }

    func postRaw(_ url: URL, json: [String: Any] = [:]) async throws {
        var req = URLRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = try JSONSerialization.data(withJSONObject: json)
        let (_, response) = try await session.data(for: req)
        guard let http = response as? HTTPURLResponse else { return }
        if http.statusCode == 401 { throw APIError.unauthorized }
        if http.statusCode == 404 { throw APIError.notFound }
        if http.statusCode >= 400 { throw APIError.serverError(http.statusCode) }
    }

    func patch<T: Decodable>(_ url: URL, body: some Encodable) async throws -> T {
        var req = URLRequest(url: url)
        req.httpMethod = "PATCH"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = try JSONEncoder().encode(body)
        return try await perform(req)
    }

    func patchJSON(_ url: URL, json: [String: Any]) async throws {
        var req = URLRequest(url: url)
        req.httpMethod = "PATCH"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = try JSONSerialization.data(withJSONObject: json)
        let (_, response) = try await session.data(for: req)
        guard let http = response as? HTTPURLResponse else { return }
        if http.statusCode == 401 { throw APIError.unauthorized }
        if http.statusCode >= 400 { throw APIError.serverError(http.statusCode) }
    }

    func delete(_ url: URL) async throws {
        var req = URLRequest(url: url)
        req.httpMethod = "DELETE"
        let (_, response) = try await session.data(for: req)
        guard let http = response as? HTTPURLResponse else { return }
        if http.statusCode == 401 { throw APIError.unauthorized }
        if http.statusCode == 404 { throw APIError.notFound }
        if http.statusCode >= 400 { throw APIError.serverError(http.statusCode) }
    }

    func perform<T: Decodable>(_ req: URLRequest) async throws -> T {
        do {
            let (data, response) = try await session.data(for: req)
            guard let http = response as? HTTPURLResponse else {
                throw APIError.serverError(0)
            }
            if http.statusCode == 401 { throw APIError.unauthorized }
            if http.statusCode == 404 { throw APIError.notFound }
            if http.statusCode >= 400 { throw APIError.serverError(http.statusCode) }
            do {
                return try JSONDecoder().decode(T.self, from: data)
            } catch {
                throw APIError.decodingError(error)
            }
        } catch let error as APIError {
            throw error
        } catch {
            throw APIError.networkError(error)
        }
    }
}
