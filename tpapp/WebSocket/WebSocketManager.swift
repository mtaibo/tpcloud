import Foundation
import Observation

@Observable
class WebSocketManager {
    private var task: URLSessionWebSocketTask?
    private let url = URL(string: "wss://tphome.migueltaibo.com/api/ws")!
    var isConnected = false

    var onBlindStateUpdate: ((String, Int, Int) -> Void)?  // id, position, motorState
    var onDeviceOnline: ((String, Bool) -> Void)?          // id, isOnline

    func connect() {
        task = APIClient.shared.session.webSocketTask(with: url)
        task?.resume()
        isConnected = true
        receiveLoop()
    }

    func disconnect() {
        task?.cancel(with: .normalClosure, reason: nil)
        task = nil
        isConnected = false
    }

    private func receiveLoop() {
        task?.receive { [weak self] result in
            guard let self else { return }
            switch result {
            case .failure:
                self.isConnected = false
                DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                    self.connect()
                }
            case .success(let msg):
                if case .string(let text) = msg {
                    self.handle(text)
                }
                self.receiveLoop()
            }
        }
    }

    private func handle(_ text: String) {
        guard
            let data = text.data(using: .utf8),
            let msg = try? JSONDecoder().decode(WSMessage.self, from: data)
        else { return }

        DispatchQueue.main.async { [self] in
            switch msg.event {
            case "device_state":
                if let id = msg.data.id,
                   let state = msg.data.state,
                   let pos = state.position,
                   let motor = state.motorState {
                    onBlindStateUpdate?(id, pos, motor)
                }
            case "device_online":
                if let id = msg.data.id { onDeviceOnline?(id, true) }
            case "device_offline":
                if let id = msg.data.id { onDeviceOnline?(id, false) }
            default:
                break
            }
        }
    }
}
