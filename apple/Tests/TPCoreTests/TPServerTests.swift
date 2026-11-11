import Foundation
import Testing
@testable import TPCore

extension TPCoreTestsRoot {

@Suite struct TPServerTests {
    @Test func everyModuleIsAPathUnderTheServer() {
        let server = TPServer(url: URL(string: "https://cloud.example.com")!)

        #expect(server.storeURL.appending(path: "/api/files/list").absoluteString == "https://cloud.example.com/store/api/files/list")
        #expect(server.homeURL.appending(path: "/api/devices/blinds").absoluteString == "https://cloud.example.com/home/api/devices/blinds")
        #expect(server.adminURL.appending(path: "/api/system").absoluteString == "https://cloud.example.com/admin/api/system")
        #expect(server.authURL.appending(path: "/auth/passkey/me").absoluteString == "https://cloud.example.com/auth/passkey/me")
        #expect(server.homeWebSocketURL.absoluteString == "wss://cloud.example.com/home/api/ws")
        #expect(server.host == "cloud.example.com")
    }
}

}
