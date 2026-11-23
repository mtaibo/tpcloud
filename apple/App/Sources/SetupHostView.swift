import SwiftUI
import TPCore

/// Setting up a machine that has TPCloud installed and nothing set up: the same questions as
/// `sudo tpcloud setup` and the setup page, answered here and applied on the machine by the same
/// code. When it's done, the new network is added and signed in to.
struct SetupHostView: View {
    let found: NetworkBrowser.Found
    @ObservedObject var networks: NetworkStore
    let onDone: () -> Void

    private enum Phase { case pairing, form, applying }

    @State private var client: SetupClient?
    @State private var phase = Phase.pairing
    @State private var hello: SetupClient.Hello?
    @State private var code = ""
    @State private var facts: SetupClient.Facts?
    @State private var answers: SetupClient.Answers?
    @State private var repeatPassword = ""
    @State private var erase = ""
    @State private var backups = false
    @State private var otherNames = ""
    @State private var progress: SetupClient.Progress?
    @State private var keySaved = false
    @State private var busy = false
    @State private var errorMessage: String?

    var body: some View {
        Group {
            switch phase {
            case .pairing: pairing
            case .form: form
            case .applying: applying
            }
        }
        .navigationTitle(found.name)
        .task {
            let client = SetupClient(base: found.url)
            self.client = client
            hello = try? await client.hello()
        }
    }

    // MARK: Pairing

    private var pairing: some View {
        Form {
            Section {
                TextField("Pairing code", text: $code)
                    .font(.title2.monospacedDigit())
                    .multilineTextAlignment(.center)
                    #if os(iOS)
                    .keyboardType(.numberPad)
                    #endif
            } footer: {
                Text(hello?.open == true
                     ? "Shown where TPCloud was installed, and by `sudo tpcloud status`. For a few minutes after it starts, it can also be set up without it."
                     : "Shown where TPCloud was installed, and by `sudo tpcloud status` on that machine.")
            }
            Section {
                Button {
                    Task { await pair() }
                } label: {
                    HStack {
                        Text(code.isEmpty ? "Continue Without a Code" : "Continue")
                        Spacer()
                        if busy { ProgressView() }
                    }
                }
                .disabled(busy || (hello?.open != true && code.count != 6))
            } footer: {
                errorFooter
            }
        }
    }

    private func pair() async {
        guard let client else { return }
        busy = true
        errorMessage = nil
        defer { busy = false }
        do {
            try await client.claim(code: code.trimmingCharacters(in: .whitespaces))
            let (facts, answers) = try await client.facts()
            self.facts = facts
            var a = answers
            if facts.needsOwner { a.owner = .init(email: "", password: "") }
            backups = !a.backupDir.isEmpty
            otherNames = a.localNames.joined(separator: ", ")
            self.answers = a
            phase = .form
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    // MARK: Questions

    private var freeDisks: [SetupClient.Disk] { (facts?.disks ?? []).filter { $0.busy == nil } }

    private func binding<T>(_ keyPath: WritableKeyPath<SetupClient.Answers, T>) -> Binding<T> {
        Binding(get: { answers![keyPath: keyPath] }, set: { answers![keyPath: keyPath] = $0 })
    }

    @ViewBuilder
    private var form: some View {
        if let facts, answers != nil {
            Form {
                Section("Name") {
                    TextField("Home, the family's name…", text: binding(\.name))
                }

                Section {
                    if !freeDisks.isEmpty && facts.pool == nil {
                        Picker("Where", selection: Binding(
                            get: { answers!.files.kind },
                            set: { kind in
                                answers!.files = kind == "pool"
                                    ? .init(kind: "pool", path: nil, disks: freeDisks.count == 1 ? [freeDisks[0].path] : [], level: "raid1", encrypt: true)
                                    : .init(kind: "folder", path: "/var/lib/tpcloud/files", disks: nil, level: nil, encrypt: nil)
                            }
                        )) {
                            Text("A Folder").tag("folder")
                            Text("New Storage").tag("pool")
                        }
                        .pickerStyle(.segmented)
                    }
                    if answers!.files.kind == "folder" {
                        TextField("/path/to/folder", text: Binding(get: { answers!.files.path ?? "" }, set: { answers!.files.path = $0 }))
                            .fontDesign(.monospaced)
                            .autocorrectionDisabled()
                    } else {
                        ForEach(facts.disks) { disk in
                            Button {
                                var list = answers!.files.disks ?? []
                                if let i = list.firstIndex(of: disk.path) { list.remove(at: i) } else { list.append(disk.path) }
                                answers!.files.disks = list
                                erase = ""
                            } label: {
                                HStack {
                                    VStack(alignment: .leading) {
                                        Text("\(disk.path) · \(disk.sizeText)").foregroundStyle(disk.busy == nil ? .primary : .secondary)
                                        Text(disk.busy ?? (disk.contents.isEmpty ? (disk.model.isEmpty ? "Empty" : disk.model) : "Has \(disk.contents.joined(separator: ", "))"))
                                            .font(.footnote).foregroundStyle(.secondary)
                                    }
                                    Spacer()
                                    if answers!.files.disks?.contains(disk.path) == true {
                                        Image(systemName: "checkmark").foregroundStyle(.tint)
                                    }
                                }
                            }
                            .disabled(disk.busy != nil)
                        }
                        let count = answers!.files.disks?.count ?? 0
                        if count > 1 {
                            Picker("Copies", selection: Binding(get: { answers!.files.level ?? "raid1" }, set: { answers!.files.level = $0 })) {
                                Text("Mirror (one disk can fail)").tag("raid1")
                                if count >= 4 { Text("Mirrored pairs").tag("raid10") }
                                Text("All the space, no copy").tag("single")
                            }
                        }
                        if count > 0 {
                            Toggle("Encrypt", isOn: Binding(get: { answers!.files.encrypt ?? true }, set: { answers!.files.encrypt = $0 }))
                            TextField("Type ERASE to confirm", text: $erase)
                                .autocorrectionDisabled()
                        }
                    }
                } header: {
                    Text("Files")
                } footer: {
                    Text(answers!.files.kind == "folder"
                         ? "On a disk that's already set up: nothing is erased."
                         : "Everything on the disks you pick is erased. Encrypted, a disk taken away can't be read; this machine opens them by itself.")
                }

                Section {
                    Toggle("Nightly Backups", isOn: $backups)
                    if backups {
                        TextField("/path/to/backups", text: binding(\.backupDir))
                            .fontDesign(.monospaced)
                            .autocorrectionDisabled()
                    }
                } header: {
                    Text("Backups")
                } footer: {
                    Text("A copy of TPCloud's own data (accounts, the photo index, settings) every night, best on another disk. Not your files.")
                }

                Section("Modules") {
                    Toggle("Store", isOn: binding(\.store))
                    Toggle("Home", isOn: binding(\.home))
                }

                Section {
                    Toggle("Answer on This Network", isOn: binding(\.local))
                    if answers!.local {
                        LabeledContent("Name") {
                            HStack(spacing: 2) {
                                TextField("tpcloud", text: Binding(get: { answers!.localHost ?? "tpcloud" }, set: { answers!.localHost = $0.lowercased() }))
                                    .multilineTextAlignment(.trailing)
                                    .autocorrectionDisabled()
                                    #if os(iOS)
                                    .textInputAutocapitalization(.never)
                                    #endif
                                Text(".local").foregroundStyle(.secondary)
                            }
                        }
                        TextField("Another name (optional)", text: $otherNames)
                            .autocorrectionDisabled()
                            #if os(iOS)
                            .textInputAutocapitalization(.never)
                            #endif
                    }
                } header: {
                    Text("At Home")
                } footer: {
                    Text("At https://\(answers!.localHost ?? "tpcloud").local, with this machine's own certificate. Another name: one your router's DNS gives it.")
                }

                Section {
                    Toggle("Tailscale", isOn: binding(\.tailscale)).disabled(facts.tailscale == nil)
                    Picker("Your Domain", selection: Binding(
                        get: { answers!.domain.kind },
                        set: { answers!.domain = .init(kind: $0, hostname: answers!.domain.hostname, new: $0 == "cloudflare" ? (answers!.domain.new ?? true) : nil) }
                    )) {
                        Text("No").tag("none")
                        Text("Cloudflare").tag("cloudflare")
                        Text("Open Port").tag("open_port")
                    }
                    if answers!.domain.kind != "none" {
                        TextField("cloud.example.com", text: Binding(get: { answers!.domain.hostname ?? "" }, set: { answers!.domain.hostname = $0 }))
                            .autocorrectionDisabled()
                            #if os(iOS)
                            .textInputAutocapitalization(.never)
                            .keyboardType(.URL)
                            #endif
                    }
                    if answers!.domain.kind == "cloudflare" && facts.hasToken {
                        Toggle("Keep the Tunnel That's There", isOn: Binding(get: { answers!.domain.new == false }, set: { answers!.domain.new = !$0 }))
                    }
                } header: {
                    Text("From Outside")
                } footer: {
                    Text(facts.tailscale.map { "Tailscale as \($0). " } ?? "Install Tailscale on the machine to use it. ")
                        + Text("Cloudflare: a tunnel, no ports opened; you sign in to Cloudflare once.")
                }

                if facts.needsOwner {
                    Section {
                        TextField("Email", text: Binding(get: { answers!.owner?.email ?? "" }, set: { answers!.owner?.email = $0 }))
                            .textContentType(.username)
                            .autocorrectionDisabled()
                            #if os(iOS)
                            .textInputAutocapitalization(.never)
                            .keyboardType(.emailAddress)
                            #endif
                        SecureField("Password", text: Binding(get: { answers!.owner?.password ?? "" }, set: { answers!.owner?.password = $0 }))
                            .textContentType(.newPassword)
                        SecureField("Same Password Again", text: $repeatPassword)
                            .textContentType(.newPassword)
                    } header: {
                        Text("Your Account")
                    } footer: {
                        Text("The first account, and the admin. 12 characters or more.")
                    }
                }

                Section {
                    Button {
                        Task { await apply() }
                    } label: {
                        HStack {
                            Text(answers!.files.kind == "pool" ? "Erase the Disks and Set It Up" : "Set It Up").fontWeight(.semibold)
                            Spacer()
                            if busy { ProgressView() }
                        }
                    }
                    .disabled(busy)
                } footer: {
                    errorFooter
                }
            }
        }
    }

    private func localProblem() -> String? {
        guard let a = answers else { return nil }
        if a.files.kind == "pool" {
            if (a.files.disks ?? []).isEmpty { return "Pick at least one disk." }
            if erase != "ERASE" { return "Type ERASE to confirm the disks can be erased." }
        }
        if let owner = a.owner, owner.password != repeatPassword { return "The passwords don't match." }
        return nil
    }

    private func apply(overwriteDNS: Bool = false) async {
        guard let client, var a = answers else { return }
        errorMessage = localProblem()
        if errorMessage != nil { return }
        if !backups { a.backupDir = "" }
        a.localNames = otherNames.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
        a.overwriteDns = overwriteDNS
        busy = true
        defer { busy = false }
        do {
            if let problem = try await client.check(a).first {
                errorMessage = problem.message
                return
            }
            try await client.apply(a)
            progress = nil
            phase = .applying
            await follow()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    // MARK: Applying

    private var applying: some View {
        List {
            if let progress {
                Section {
                    ForEach(Array(progress.log.enumerated()), id: \.offset) { index, line in
                        HStack {
                            Text(line.text)
                            Spacer()
                            if line.kind == "done" || progress.state == "done" || index < progress.log.count - 1 {
                                Image(systemName: "checkmark").foregroundStyle(.green)
                            } else if progress.state == "running" {
                                ProgressView()
                            }
                        }
                    }
                    if progress.log.isEmpty && progress.state == "running" {
                        ProgressView().frame(maxWidth: .infinity)
                    }
                }
                if let link = progress.signIn.flatMap(URL.init(string:)), progress.state == "running" {
                    Section {
                        Link("Open Cloudflare's Sign-In", destination: link)
                    } header: {
                        Text("Cloudflare")
                    } footer: {
                        Text("Sign in and pick the domain of the address you gave. This carries on by itself afterwards.")
                    }
                }
                if let key = progress.recoveryKey {
                    Section {
                        Text(key).font(.body.monospaced()).textSelection(.enabled)
                        Toggle("I've Written It Down", isOn: $keySaved)
                    } header: {
                        Text("Recovery Key")
                    } footer: {
                        Text("The machine opens its disks by itself. If it ever breaks, this key opens them on any Linux. It's shown only now.")
                    }
                }
                if progress.state == "failed" {
                    if let host = progress.dnsTaken {
                        Section {
                            Button("Point \(host) at This Machine") { Task { await apply(overwriteDNS: true) } }
                        } footer: {
                            Text("\(host) already points somewhere else in Cloudflare.")
                        }
                    }
                    Section {
                        Button("Back to the Answers") { phase = .form }
                    } footer: {
                        Text(progress.error ?? "").foregroundStyle(.red)
                    }
                }
                if progress.state == "done" && (progress.recoveryKey == nil || keySaved) {
                    Section {
                        Button {
                            Task { await addTheNetwork() }
                        } label: {
                            HStack {
                                Text("Add \(answers?.name ?? "It") and Sign In").fontWeight(.semibold)
                                Spacer()
                                if busy { ProgressView() }
                            }
                        }
                        .disabled(busy)
                    } footer: {
                        VStack(alignment: .leading) {
                            Text("Set up at \(progress.url ?? "").")
                            errorFooter
                        }
                    }
                }
            }
        }
    }

    private func follow() async {
        guard let client else { return }
        while !Task.isCancelled {
            if let p = try? await client.progress() {
                progress = p
                if p.state != "running" { return }
            }
            try? await Task.sleep(for: .seconds(1))
        }
    }

    /// The machine restarts as TPCloud a few seconds after it's set up: its card is read from the
    /// address it gave (trying for a while) and the network added.
    private func addTheNetwork() async {
        guard let url = progress?.url else { return }
        busy = true
        errorMessage = nil
        defer { busy = false }
        for attempt in 0..<20 {
            if let loaded = try? await NetworkCardLoader.load(url) {
                await networks.add(loaded, reachedAt: URL(string: url)!)
                onDone()
                return
            }
            if attempt == 19 { break }
            try? await Task.sleep(for: .seconds(3))
        }
        errorMessage = "It isn't answering at \(url) yet. Add it from the list of networks in a moment."
    }

    @ViewBuilder
    private var errorFooter: some View {
        if let errorMessage {
            Label(errorMessage, systemImage: "exclamationmark.circle.fill").foregroundStyle(.red)
        }
    }
}
