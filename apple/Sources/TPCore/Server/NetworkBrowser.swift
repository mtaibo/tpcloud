import Foundation
import Network

/// TPCloud hosts announcing themselves on this network (Bonjour): set-up networks
/// (`_tpcloud._tcp`) and machines waiting to be set up (`_tpcloud-setup._tcp`). Their TXT record
/// carries the host name the address is built from.
@MainActor
public final class NetworkBrowser: ObservableObject {
    public struct Found: Identifiable, Hashable, Sendable {
        public let name: String
        public let host: String
        public let port: Int
        public let networkID: String?
        public var id: String { "\(host):\(port)" }

        /// The network's address (HTTPS) or the setup page's (plain HTTP on the local network).
        public var url: URL {
            URL(string: port == 443 ? "https://\(host)" : port == 80 ? "http://\(host)" : "http://\(host):\(port)")!
        }
    }

    @Published public private(set) var networks: [Found] = []
    @Published public private(set) var waiting: [Found] = []

    private var browsers: [NWBrowser] = []

    public init() {}

    public func start() {
        guard browsers.isEmpty else { return }
        browsers = [browse("_tpcloud._tcp") { [weak self] in self?.networks = $0 },
                    browse("_tpcloud-setup._tcp") { [weak self] in self?.waiting = $0 }]
    }

    public func stop() {
        browsers.forEach { $0.cancel() }
        browsers = []
    }

    private func browse(_ type: String, update: @escaping @MainActor ([Found]) -> Void) -> NWBrowser {
        let browser = NWBrowser(for: .bonjourWithTXTRecord(type: type, domain: nil), using: .tcp)
        browser.browseResultsChangedHandler = { results, _ in
            let found: [Found] = results.compactMap { result in
                guard case .service(let name, _, _, _) = result.endpoint,
                      case .bonjour(let txt) = result.metadata else { return nil }
                let entries = txt.dictionary
                guard let host = entries["host"], !host.isEmpty else { return nil }
                let port = type == "_tpcloud._tcp" ? 443 : (entries["port"].flatMap(Int.init) ?? 80)
                return Found(name: name, host: host, port: port, networkID: entries["id"])
            }
            let unique = Array(Set(found)).sorted { $0.name < $1.name }
            Task { @MainActor in update(unique) }
        }
        browser.start(queue: .main)
        return browser
    }
}
