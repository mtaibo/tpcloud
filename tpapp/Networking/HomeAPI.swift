import Foundation

private let base = "https://tphome.migueltaibo.com/api"
private let client = APIClient.shared

enum HomeAPI {

    // MARK: - Devices

    static func blinds() async throws -> [BlindDevice] {
        try await client.get(URL(string: "\(base)/devices/blinds")!)
    }

    static func lights() async throws -> [LightDevice] {
        try await client.get(URL(string: "\(base)/devices/lights")!)
    }

    static func pendingDevices() async throws -> [PendingDevice] {
        try await client.get(URL(string: "\(base)/devices/pending")!)
    }

    static func deleteDevice(_ id: String) async throws {
        try await client.delete(URL(string: "\(base)/devices/\(id)")!)
    }

    // MARK: - Commands

    static func commandUp(_ id: String) async throws {
        var req = URLRequest(url: URL(string: "\(base)/commands/\(id)/up")!)
        req.httpMethod = "POST"
        _ = try await client.session.data(for: req)
    }

    static func commandDown(_ id: String) async throws {
        var req = URLRequest(url: URL(string: "\(base)/commands/\(id)/down")!)
        req.httpMethod = "POST"
        _ = try await client.session.data(for: req)
    }

    static func commandStop(_ id: String) async throws {
        var req = URLRequest(url: URL(string: "\(base)/commands/\(id)/stop")!)
        req.httpMethod = "POST"
        _ = try await client.session.data(for: req)
    }

    static func commandSet(_ id: String, position: Int) async throws {
        var req = URLRequest(url: URL(string: "\(base)/commands/\(id)/set/\(position)")!)
        req.httpMethod = "POST"
        _ = try await client.session.data(for: req)
    }

    // MARK: - Firmware

    static func firmwareList() async throws -> [FirmwareInfo] {
        try await client.get(URL(string: "\(base)/firmware/list")!)
    }

    static func activateFirmware(_ id: Int) async throws {
        var req = URLRequest(url: URL(string: "\(base)/firmware/\(id)/activate")!)
        req.httpMethod = "POST"
        _ = try await client.session.data(for: req)
    }

    static func deleteFirmware(_ id: Int) async throws {
        try await client.delete(URL(string: "\(base)/firmware/\(id)")!)
    }

    // MARK: - Admin

    static func adminReboot(_ id: String) async throws {
        var req = URLRequest(url: URL(string: "\(base)/admin/\(id)/reboot")!)
        req.httpMethod = "POST"
        _ = try await client.session.data(for: req)
    }

    static func adminReset(_ id: String) async throws {
        var req = URLRequest(url: URL(string: "\(base)/admin/\(id)/reset")!)
        req.httpMethod = "POST"
        _ = try await client.session.data(for: req)
    }

    static func adminOTA(_ id: String) async throws {
        var req = URLRequest(url: URL(string: "\(base)/admin/\(id)/ota")!)
        req.httpMethod = "POST"
        _ = try await client.session.data(for: req)
    }

    static func adminUpdate() async throws {
        var req = URLRequest(url: URL(string: "\(base)/admin/update")!)
        req.httpMethod = "POST"
        _ = try await client.session.data(for: req)
    }
}
