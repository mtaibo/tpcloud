import Foundation

struct BlindDevice: Codable, Identifiable {
    let id: String
    let mac: String
    let firmwareVersion: String?
    let online: Bool
    let lastSeen: String?
    let upTime: Int?
    let downTime: Int?
    let downPos: Int?
    let invertedRelays: Bool?
    var position: Int?
    var motorState: Int?
    enum CodingKeys: String, CodingKey {
        case id, mac, online
        case firmwareVersion = "firmware_version"
        case lastSeen = "last_seen"
        case upTime = "up_time"
        case downTime = "down_time"
        case downPos = "down_pos"
        case invertedRelays = "inverted_relays"
        case position
        case motorState = "motor_state"
    }

    // Convenience initializer for copy-with-modifications pattern
    func withPosition(_ pos: Int, motorState motor: Int) -> BlindDevice {
        BlindDevice(
            id: id, mac: mac, firmwareVersion: firmwareVersion,
            online: online, lastSeen: lastSeen,
            upTime: upTime, downTime: downTime,
            downPos: downPos, invertedRelays: invertedRelays,
            position: pos, motorState: motor
        )
    }

    func withOnline(_ isOnline: Bool) -> BlindDevice {
        BlindDevice(
            id: id, mac: mac, firmwareVersion: firmwareVersion,
            online: isOnline, lastSeen: lastSeen,
            upTime: upTime, downTime: downTime,
            downPos: downPos, invertedRelays: invertedRelays,
            position: position, motorState: motorState
        )
    }
}

struct LightDevice: Codable, Identifiable {
    let id: String
    let mac: String
    let firmwareVersion: String?
    let online: Bool
    let lastSeen: String?
    var on: Bool
    enum CodingKeys: String, CodingKey {
        case id, mac, online, on
        case firmwareVersion = "firmware_version"
        case lastSeen = "last_seen"
    }
}

struct PendingDevice: Codable, Identifiable {
    var id: String { mac }
    let mac: String
}

struct FirmwareInfo: Codable, Identifiable {
    let id: Int
    let name: String
    let chip: String
    let target: String
    let version: String
    let notes: String?
    let uploadedAt: String
    let active: Bool
    enum CodingKeys: String, CodingKey {
        case id, name, chip, target, version, notes, active
        case uploadedAt = "uploaded_at"
    }
}

// WebSocket message types
struct WSMessage: Codable {
    let event: String
    let data: WSData
}

struct WSData: Codable {
    let id: String?
    let state: WSState?
    let hardware: WSHardware?
    let prefs: WSPrefs?
}

struct WSState: Codable {
    let position: Int?
    let motorState: Int?
    enum CodingKeys: String, CodingKey {
        case position
        case motorState = "motor_state"
    }
}

struct WSHardware: Codable {
    let mac: String?
    let firmwareVersion: String?
    enum CodingKeys: String, CodingKey {
        case mac
        case firmwareVersion = "firmware_version"
    }
}

struct WSPrefs: Codable {
    let upTime: Int?
    let downTime: Int?
    let downPos: Int?
    let invertedRelays: Bool?
    enum CodingKeys: String, CodingKey {
        case upTime = "up_time"
        case downTime = "down_time"
        case downPos = "down_pos"
        case invertedRelays = "inverted_relays"
    }
}
