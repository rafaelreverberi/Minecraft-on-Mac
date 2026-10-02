import SwiftUI
import AppKit
import LauncherCore

@main
struct MinecraftMacLauncherApp: App {
    @StateObject private var model = AppModel()
    init() { NSApplication.shared.setActivationPolicy(.regular) }
    var body: some Scene {
        WindowGroup("Minecraft on Mac") { LauncherView().environmentObject(model).frame(minWidth: 900, minHeight: 650).task { await model.load(); NSApplication.shared.activate(ignoringOtherApps: true); if ProcessInfo.processInfo.arguments.contains("--smoke-launch-current") { await model.play() }; if ProcessInfo.processInfo.arguments.contains("--setup") { await model.installAndPlay() }; if ProcessInfo.processInfo.arguments.contains("--setup-bedrock") { await model.installAndPlay(game: .bedrock) }; if ProcessInfo.processInfo.arguments.contains("--repair-current"), let current = model.current { await model.repair(current); if model.error == nil { await model.play() } } } }
        Settings { SettingsView().environmentObject(model).frame(width: 580, height: 440).padding(24) }
    }
}
enum Page: String, CaseIterable, Identifiable {
    case dungeons = "Minecraft Dungeons II", bedrock = "Minecraft: Bedrock", downloads = "Downloads", versions = "Versions", settings = "Settings", diagnostics = "Diagnostics"
    var id: Self { self }
    var symbol: String {
        switch self { case .dungeons: "shield.lefthalf.filled"; case .bedrock: "cube.fill"; case .downloads: "arrow.down.circle"; case .versions: "clock.arrow.circlepath"; case .settings: "gearshape"; case .diagnostics: "waveform.path.ecg" }
    }
}
struct LauncherView: View {
    @EnvironmentObject var model: AppModel
    @State private var page: Page? = ProcessInfo.processInfo.arguments.contains("--setup-bedrock") ? .bedrock : .dungeons
    var body: some View {
        NavigationSplitView {
            List(selection: $page) {
                Section("Games") { row(.dungeons); row(.bedrock) }
                Section("Library") { row(.downloads); row(.versions) }
                Section { row(.settings); row(.diagnostics) }
            }.listStyle(.sidebar)
            VStack(alignment: .leading, spacing: 5) {
                Text("Minecraft on Mac").font(.headline)
                Text("Development preview • Apple Silicon").font(.caption).foregroundStyle(.secondary)
            }.padding().frame(maxWidth: .infinity, alignment: .leading)
        } detail: {
            VStack(spacing: 0) {
                ScrollView {
                    Group {
                        switch page ?? .dungeons {
                        case .dungeons: DungeonsView()
                        case .bedrock: BedrockView()
                        case .versions: VersionsView()
                        case .downloads: DownloadsView()
                        case .settings: SettingsView()
                        case .diagnostics: DiagnosticsView()
                        }
                    }.padding(30).frame(maxWidth: 1050, alignment: .leading)
                }
                if let progress = model.progress {
                    VStack(alignment: .leading, spacing: 8) {
                        Text(progress.phase).font(.headline)
                        if let fraction = progress.fraction { ProgressView(value: fraction) } else { ProgressView().controlSize(.small) }
                        if progress.total > 0 { Text(progress.phase.contains("Downloading") || ["Preparing game files", "Verifying encrypted package"].contains(progress.phase) ? "\(ByteCountFormatter.string(fromByteCount: progress.completed, countStyle: .file)) / \(ByteCountFormatter.string(fromByteCount: progress.total, countStyle: .file))" : "\(progress.completed) / \(progress.total) files").font(.caption).foregroundStyle(.secondary) }
                    }.padding().frame(maxWidth: .infinity, alignment: .leading).background(.regularMaterial)
                }
                if !model.message.isEmpty { Text(model.message).font(.callout).foregroundStyle(.secondary).padding().frame(maxWidth: .infinity, alignment: .leading) }
            }.background(Color(nsColor: .windowBackgroundColor))
        }.navigationSplitViewColumnWidth(min: 220, ideal: 245)
        .onChange(of: page, initial: true) { _, value in
            if value == .bedrock { model.selectedGame = .bedrock }
            else if value == .dungeons { model.selectedGame = .dungeons2 }
        }
        .onReceive(NotificationCenter.default.publisher(for: NSApplication.didBecomeActiveNotification)) { _ in Task { await model.refresh() } }
        .alert("Operation could not finish", isPresented: Binding(get: { model.error != nil }, set: { if !$0 { model.error = nil } })) {
            Button("OK") { model.error = nil }
            Button("Copy Details") { NSPasteboard.general.clearContents(); NSPasteboard.general.setString(model.error?.errorDescription ?? "", forType: .string) }
        } message: { Text(model.error?.errorDescription ?? "") }
    }
    func row(_ page: Page) -> some View { Label(page.rawValue, systemImage: page.symbol).tag(page) }
}
struct DungeonsView: View {
    @EnvironmentObject var model: AppModel
    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            ZStack(alignment: .bottomLeading) {
                LinearGradient(colors: [Color(red: 0.10, green: 0.26, blue: 0.20), Color(red: 0.06, green: 0.10, blue: 0.15)], startPoint: .topLeading, endPoint: .bottomTrailing)
                Image(systemName: "shield.lefthalf.filled").font(.system(size: 150)).foregroundStyle(.white.opacity(0.09)).frame(maxWidth: .infinity, alignment: .trailing).padding(40)
                VStack(alignment: .leading, spacing: 12) {
                    Text("YOUR ADVENTURE, ON MAC").font(.caption.weight(.semibold)).tracking(2).foregroundStyle(.white.opacity(0.7))
                    Text("Minecraft\nDungeons II").font(.system(size: 40, weight: .bold, design: .rounded)).foregroundStyle(.white)
                    Text("Local compatibility • CrossOver").foregroundStyle(.white.opacity(0.8))
                }.padding(30)
            }.frame(height: 250).clipShape(RoundedRectangle(cornerRadius: 16))
            if let install = model.current(for: .dungeons2) {
                HStack(alignment: .top, spacing: 24) {
                    metric("Installed", "Version \(install.version)", "shippingbox")
                    metric("Compatibility", install.state == .ready ? "Files and runtime verified" : "Needs verification", "checkmark.shield")
                    metric("Storage", ByteCountFormatter.string(fromByteCount: install.diskBytes, countStyle: .file), "internaldrive")
                }
                GroupBox {
                    VStack(alignment: .leading, spacing: 8) {
                        LabeledContent("CrossOver bottle", value: install.bottle)
                        LabeledContent("Account", value: install.compatibilityProfile.hasPrefix("dungeons2-managed-") ? (model.accounts[.dungeons2]?.gamertag ?? "Microsoft • macOS Keychain") : "Existing game login")
                        LabeledContent("Available version", value: model.accounts[.dungeons2]?.availableVersion ?? "Not checked")
                        LabeledContent("Installation", value: install.managed ? "Launcher-managed snapshot" : "External reference · protected")
                    }.padding(8)
                }
                HStack(spacing: 12) {
                    Button { Task { await model.play(game: .dungeons2) } } label: { Label(model.running ? "Game running" : "Play", systemImage: "play.fill").padding(.horizontal, 24).padding(.vertical, 5) }
                        .buttonStyle(.borderedProminent).tint(Color(red: 0.16, green: 0.49, blue: 0.30)).disabled(model.busy || model.running)
                    Button("Verify") { Task { await model.verify(install) } }.disabled(model.busy || model.running)
                    Button("Repair") { Task { await model.repair(install) } }.disabled(!install.managed || model.busy || model.running)
                    Button("Open Folder") { model.open(install.path) }
                }
                Text("Reference launch preserves the existing Microsoft/Xbox login. File checks do not establish entitlement or validate gameplay, saves and multiplayer.").font(.caption).foregroundStyle(.secondary)
            } else {
                Text("Sign in and play").font(.title2.bold())
                Text("Sign in with your Microsoft account. The launcher verifies your game license, downloads the game and components, and creates its CrossOver environment.").foregroundStyle(.secondary)
                BottlePicker()
                Button("Choose Installation…") { model.chooseInstallation() }.buttonStyle(.borderedProminent).disabled(model.busy || model.selectedBottle.isEmpty)
            }
            AccountView(game: .dungeons2)
            GroupBox("Automatic installation") {
                VStack(alignment: .leading, spacing: 10) {
                    Text("Game downloads, compatibility components and the game environment are managed automatically. CrossOver must be installed and licensed. Allow at least 35 GB free during setup.").font(.callout).foregroundStyle(.secondary)
                    HStack {
                        Button(model.current(for: .dungeons2) == nil ? "Sign In, Install & Play" : "Install a Fresh Managed Copy") { Task { await model.installAndPlay(game: .dungeons2) } }.buttonStyle(.borderedProminent)
                        if model.current(for: .dungeons2) != nil { Button("Check for Updates") { Task { await model.checkUpdates(game: .dungeons2) } } }
                    }.disabled(model.busy || model.running)
                    Text("Installation downloads Microsoft runtime components under their publisher's license terms.").font(.caption).foregroundStyle(.secondary)
                    Link("Microsoft GDK license", destination: URL(string: "https://github.com/microsoft/GDK/blob/main/LICENSE.md")!)
                }.padding(8).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }
    func metric(_ label: String, _ value: String, _ icon: String) -> some View {
        VStack(alignment: .leading, spacing: 8) { Label(label, systemImage: icon).font(.caption).foregroundStyle(.secondary); Text(value).font(.headline) }.frame(maxWidth: .infinity, alignment: .leading)
    }
}
struct BedrockView: View {
    private let game: GameDefinition = .bedrock
    @EnvironmentObject var model: AppModel
    @State private var pendingEnvironment: String?
    @State private var pendingStaging: String?
    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            ZStack(alignment: .bottomLeading) {
                LinearGradient(colors: [Color(red: 0.10, green: 0.26, blue: 0.20), Color(red: 0.06, green: 0.10, blue: 0.15)], startPoint: .topLeading, endPoint: .bottomTrailing)
                Image(systemName: "cube.fill").font(.system(size: 150)).foregroundStyle(.white.opacity(0.09)).frame(maxWidth: .infinity, alignment: .trailing).padding(40)
                VStack(alignment: .leading, spacing: 12) {
                    Text("YOUR ADVENTURE, ON MAC").font(.caption.weight(.semibold)).tracking(2).foregroundStyle(.white.opacity(0.7))
                    Text(game.name).font(.system(size: 40, weight: .bold, design: .rounded)).foregroundStyle(.white)
                    Text("Local compatibility • CrossOver").foregroundStyle(.white.opacity(0.8))
                }.padding(30)
            }.frame(height: 250).clipShape(RoundedRectangle(cornerRadius: 16))
            if let install = model.current(for: game) {
                HStack(alignment: .top, spacing: 24) {
                    metric("Installed", "Version \(install.version)", "shippingbox")
                    metric("Compatibility", install.state == .ready ? "Files and runtime verified" : "Needs verification", "checkmark.shield")
                    metric("Storage", ByteCountFormatter.string(fromByteCount: install.diskBytes, countStyle: .file), "internaldrive")
                }
                GroupBox {
                    VStack(alignment: .leading, spacing: 8) {
                        LabeledContent("CrossOver bottle", value: install.bottle)
                        LabeledContent("Account", value: install.isOfficialManaged ? (model.accounts[game]?.gamertag ?? "Microsoft • macOS Keychain") : "Existing game login")
                        LabeledContent("Available version", value: model.accounts[game]?.availableVersion ?? "Not checked")
                        LabeledContent("Installation", value: install.isOfficialManaged ? "Launcher-managed official Microsoft package" : install.displayKind)
                    }.padding(8)
                }
                HStack(spacing: 12) {
                    Button { Task { await model.play(game: game) } } label: { Label(model.running ? "Game running" : "Play", systemImage: "play.fill").padding(.horizontal, 24).padding(.vertical, 5) }
                        .buttonStyle(.borderedProminent).tint(Color(red: 0.16, green: 0.49, blue: 0.30)).disabled(model.busy || model.running)
                    Button("Verify") { Task { await model.verify(install) } }.disabled(model.busy || model.running)
                    Button("Repair") { Task { await model.repair(install) } }.disabled(!install.managed || model.busy || model.running)
                    Button("Open Folder") { model.open(install.path) }
                }
                Text("Reference launch preserves the existing Microsoft/Xbox login. File checks do not establish entitlement or validate gameplay, saves and multiplayer.").font(.caption).foregroundStyle(.secondary)
            } else {
                Text("Sign in and play").font(.title2.bold())
                Text("Sign in with your Microsoft account. The launcher verifies your game license, downloads the game and components, and creates its CrossOver environment.").foregroundStyle(.secondary)
            }
            Group {
                Text("Bedrock 1.26.5203.0 • CrossOver 26.3.x • Apple silicon • D3DMetal").font(.caption).foregroundStyle(.secondary)
                Text("Game files are decrypted once during installation for persistent SSD use. Worlds live in the owned CrossOver environment. A newer build may upgrade worlds; selecting an older executable does not restore world data. Back up worlds before launching a new version.").font(.callout).foregroundStyle(.secondary)
                if let install = model.current(for: game) { Text(install.path.path).font(.caption.monospaced()).textSelection(.enabled) }
            }
            AccountView(game: .bedrock)
            GroupBox("Automatic installation") {
                VStack(alignment: .leading, spacing: 10) {
                    Text("Game downloads, compatibility components and the game environment are managed automatically. CrossOver must be installed and licensed. Allow at least 35 GB free during setup.").font(.callout).foregroundStyle(.secondary)
                    HStack {
                        Button(model.current(for: game) == nil ? "Sign In, Install & Play" : "Install a Fresh Managed Copy") { Task { await model.installAndPlay(game: game) } }.buttonStyle(.borderedProminent)
                        if model.current(for: game) != nil { Button("Check for Updates") { Task { await model.checkUpdates(game: game) } } }
                    }.disabled(model.busy || model.running)
                    Text("Installation downloads Microsoft runtime components under their publisher's license terms.").font(.caption).foregroundStyle(.secondary)
                    Link("Microsoft GDK license", destination: URL(string: "https://github.com/microsoft/GDK/blob/main/LICENSE.md")!)
                }.padding(8).frame(maxWidth: .infinity, alignment: .leading)
            }
            GroupBox("Bedrock storage and environment") {
                VStack(alignment: .leading, spacing: 10) {
                    Button("Open Bedrock Data") { model.open(LibraryStore.defaultRoot.appendingPathComponent("Games/bedrock")) }
                    ForEach(model.database.retainedEnvironmentsByGameId[game.rawValue] ?? [], id: \.self) { name in
                        Text("Saved Bedrock environment: \(name)").font(.callout)
                        Text("Kept for your worlds; reused on the next Bedrock installation.").font(.caption).foregroundStyle(.secondary)
                        Button("Delete Bedrock Environment and Worlds…", role: .destructive) { pendingEnvironment = name }.disabled(model.busy || model.running)
                    }
                    ForEach(model.staging.filter { $0.hasPrefix("bedrock/") }, id: \.self) { name in
                        HStack { Text("Interrupted Bedrock installation"); Spacer(); Button("Delete Temporary Files…") { pendingStaging = name }.disabled(model.busy || model.running) }
                    }
                }.padding(8).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .confirmationDialog("Permanently delete this Bedrock environment and all its worlds?", isPresented: Binding(get: { pendingEnvironment != nil }, set: { if !$0 { pendingEnvironment = nil } }), titleVisibility: .visible) {
            if let name = pendingEnvironment { Button("Delete Bedrock Environment and Worlds", role: .destructive) { pendingEnvironment = nil; Task { await model.perform { try await model.store?.removeRetainedEnvironment(name) } } } }
        } message: { Text("Deletes only this retained, launcher-owned Bedrock environment. Other game environments and the Microsoft Keychain login are preserved.") }
        .confirmationDialog("Delete interrupted Bedrock installation?", isPresented: Binding(get: { pendingStaging != nil }, set: { if !$0 { pendingStaging = nil } }), titleVisibility: .visible) {
            if let name = pendingStaging { Button("Delete Temporary Files", role: .destructive) { Task { await model.perform { try await model.store?.removeStaging(name) } }; pendingStaging = nil } }
        }
    }
    func metric(_ label: String, _ value: String, _ icon: String) -> some View {
        VStack(alignment: .leading, spacing: 8) { Label(label, systemImage: icon).font(.caption).foregroundStyle(.secondary); Text(value).font(.headline) }.frame(maxWidth: .infinity, alignment: .leading)
    }
}
struct BottlePicker: View {
    @EnvironmentObject var model: AppModel
    var body: some View {
        Picker("CrossOver bottle", selection: $model.selectedBottle) {
            if model.bottles.isEmpty { Text("No bottles detected").tag("") }
            ForEach(model.bottles, id: \.self) { Text($0).tag($0) }
        }.frame(maxWidth: 440)
    }
}
struct VersionsView: View {
    @EnvironmentObject var model: AppModel
    @State private var pendingRemoval: Installation?
    @State private var removeEnvironment = false
    @State private var cloneConfirmation: Installation?
    @State private var clearCurrent = false
    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Versions").font(.largeTitle.bold())
            Text("Installed game copies, not launcher bundles. Current marks the copy used by Start Game.").foregroundStyle(.secondary)
            ForEach(model.database.installations) { install in
                GroupBox {
                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            Text("\(install.game.name) • \(install.version)").font(.title2.bold())
                            if install.id == model.database.currentByGameId[install.gameId] { Text("Current").font(.caption.bold()).padding(5).background(.green.opacity(0.15), in: Capsule()) }
                            Spacer(); Text(install.displayKind).foregroundStyle(.secondary)
                        }
                        Text("\(ByteCountFormatter.string(fromByteCount: install.storageBytes ?? install.diskBytes, countStyle: .file)) of files • \(install.state.rawValue)").font(.callout).foregroundStyle(.secondary)
                        Text(install.managed ? "This launcher manages this copy. Version deletion includes its downloaded component cache." : "Your original local copy. Removing this entry only unregisters it; its files stay intact.").font(.callout).foregroundStyle(.secondary)
                        Text(install.path.path).font(.caption.monospaced()).textSelection(.enabled)
                        Button("Show in Finder") { NSWorkspace.shared.open(install.managed ? install.path.deletingLastPathComponent() : install.path) }
                        DisclosureGroup("Game environment") {
                            Text(install.bottle).font(.caption.monospaced()).textSelection(.enabled)
                            Text("The CrossOver environment stores prerequisites and account-specific saves separately from game versions.").font(.caption).foregroundStyle(.secondary)
                        }
                        HStack {
                            Button("Use") { Task { await model.select(install) } }.disabled(install.id == model.database.currentByGameId[install.gameId] || install.state != .ready)
                            Button("Verify") { Task { await model.verify(install) } }
                            Button("Create Snapshot…") { cloneConfirmation = install }
                            Button(install.managed ? "Delete Version…" : "Unregister…", role: .destructive) { removeEnvironment = false; pendingRemoval = install }
                        }.disabled(model.busy || model.running)
                        if model.developerMode { Text(install.compatibilityProfile).font(.caption.monospaced()) }
                    }.padding(10)
                }
            }
            if model.database.installations.isEmpty { Text("No registered installations.").foregroundStyle(.secondary) }
            BottlePicker()
            HStack {
                Button("Register Installation…") { model.chooseInstallation() }.disabled(model.busy || model.selectedBottle.isEmpty || model.running)
                if let current = model.current {
                    Button("Change Bottle") { Task { await model.perform { try await model.store?.changeBottle(current.id, bottle: model.selectedBottle) } } }.disabled(model.busy || model.running)
                    Button("Clear Current Selection…") { clearCurrent = true }.disabled(model.busy || model.running)
                }
            }
        }
        .sheet(item: $pendingRemoval) { install in
            VStack(alignment: .leading, spacing: 16) {
                Text(install.managed ? "Delete Version \(install.version)?" : "Unregister local installation?").font(.title2.bold())
                Text(install.managed ? "Deletes the complete version folder, including game files, downloaded component cache and installation metadata." : "Removes this entry from the launcher. Your original game files and environment are preserved.")
                if install.id == model.database.currentByGameId[install.gameId] { Text("This is the current installation. Start Game will be unavailable until you select or install another copy.").foregroundStyle(.secondary) }
                if install.managed && install.isOfficialManaged {
                    let shared = model.database.installations.contains { $0.id != install.id && $0.bottle == install.bottle }
                    Toggle("Also delete this game's environment and all its saves", isOn: $removeEnvironment).disabled(shared)
                    Text(shared ? "Another version uses this environment, so it must be kept." : "Off by default. Enabling this permanently removes the owned CrossOver environment and its saved games. Microsoft Keychain login is preserved.").font(.callout).foregroundStyle(.secondary)
                }
                HStack {
                    Button("Cancel") { pendingRemoval = nil }.keyboardShortcut(.cancelAction)
                    Spacer()
                    Button(install.managed ? "Delete Version" : "Unregister", role: .destructive) {
                        let deleteEnvironment = removeEnvironment
                        pendingRemoval = nil
                        Task { await model.remove(install, removeEnvironment: deleteEnvironment) }
                    }.disabled(model.busy || model.running)
                }
            }.padding(24).frame(width: 520)
        }
        .confirmationDialog("Create a managed snapshot?", isPresented: Binding(get: { cloneConfirmation != nil }, set: { if !$0 { cloneConfirmation = nil } }), titleVisibility: .visible) {
            if let install = cloneConfirmation { Button("Clone and Verify") { Task { await model.clone(install) }; cloneConfirmation = nil } }
        } message: { Text("Copies this local installation using APFS copy-on-write and hashes all game files. This can take several minutes. Current selection stays available.") }
        .confirmationDialog("Clear current selection?", isPresented: $clearCurrent, titleVisibility: .visible) {
            Button("Clear Selection") { Task { await model.perform { try await model.store?.clearSelection(game: model.selectedGame) } } }
        } message: { Text("Play will be unavailable until you select another version. You can then remove the previous version.") }
    }
}
struct DownloadsView: View {
    var body: some View {
        ContentUnavailableView { Label("Downloads", systemImage: "arrow.down.circle") } description: {
            Text("Downloads and preparation appear here while the launcher installs a game. Start with Sign In, Install & Play on the game page.")
        }
    }
}
struct SettingsView: View {
    @EnvironmentObject var model: AppModel
    @State private var cacheConfirmation = false
    @State private var pendingStaging: String?
    @State private var pendingEnvironment: String?
    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Settings").font(.largeTitle.bold())
            GroupBox("CrossOver") {
                VStack(alignment: .leading, spacing: 10) {
                    Text(model.crossOver == nil ? "CrossOver not detected" : "CrossOver detected").font(.headline)
                    BottlePicker()
                    if let crossover = model.crossOver {
                        Button("Open Selected Bottle") { if let url = try? crossover.bottleURL(model.selectedBottle) { model.open(url) } }
                    }
                    Link("Download CrossOver from CodeWeavers", destination: URL(string: "https://www.codeweavers.com/crossover")!)
                    Text("The launcher creates its own game environment automatically. The bottle picker is only needed for external installations.").font(.caption).foregroundStyle(.secondary)
                }.padding(8).frame(maxWidth: .infinity, alignment: .leading)
            }
            GroupBox("Storage") {
                VStack(alignment: .leading, spacing: 10) {
                    Button("Open Launcher Data") { model.open(LibraryStore.defaultRoot) }
                    Button("Clear Download Cache…") { cacheConfirmation = true }.disabled(model.busy || model.running)
                    ForEach(model.database.retainedEnvironments ?? [], id: \.self) { name in
                        VStack(alignment: .leading, spacing: 6) {
                            Text("Saved game environment (no installed version)").font(.callout)
                            Text("Kept for your saves; reused on the next installation.").font(.caption).foregroundStyle(.secondary)
                            Button("Delete Environment and Saves…", role: .destructive) { pendingEnvironment = name }.disabled(model.busy || model.running)
                        }
                    }
                    ForEach(model.staging.filter { !$0.hasPrefix("bedrock/") }, id: \.self) { name in
                        HStack { Text("Interrupted snapshot").font(.callout); Spacer(); Button("Delete Temporary Files…") { pendingStaging = name }.disabled(model.busy || model.running) }
                    }
                }.padding(8).frame(maxWidth: .infinity, alignment: .leading)
            }
            Toggle("Developer Mode", isOn: $model.developerMode)
            GroupBox("Dependencies") {
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(DependencyManager.inspect(developerMode: model.developerMode)) { dependency in
                        HStack { Image(systemName: dependency.available ? "checkmark.circle.fill" : "circle.dashed").foregroundStyle(dependency.available ? .green : .secondary); Text(dependency.name); Spacer(); Text(dependency.requiredForPlay ? "Play" : "Development").font(.caption).foregroundStyle(.secondary) }
                    }
                }.padding(8)
            }
            Text("No telemetry. Credentials are never included in diagnostics. Native Microsoft credentials use a separate macOS Keychain namespace. The reference game retains its existing login.").font(.caption).foregroundStyle(.secondary)
        }
        .confirmationDialog("Permanently delete this environment and all its saves?", isPresented: Binding(get: { pendingEnvironment != nil }, set: { if !$0 { pendingEnvironment = nil } }), titleVisibility: .visible) {
            if let name = pendingEnvironment { Button("Delete Environment and Saves", role: .destructive) { pendingEnvironment = nil; Task { await model.perform { try await model.store?.removeRetainedEnvironment(name) } } } }
        } message: { Text("Deletes only this retained, launcher-owned CrossOver environment. Microsoft Keychain login and other game environments are preserved.") }
        .confirmationDialog("Clear download cache?", isPresented: $cacheConfirmation, titleVisibility: .visible) {
            Button("Clear Cache", role: .destructive) { Task { await model.perform { try await model.store?.clearCache(); try await model.diagnostics?.record(.cacheCleared) } } }
        } message: { Text("Installed versions and saved games are preserved.") }
        .confirmationDialog("Delete interrupted snapshot?", isPresented: Binding(get: { pendingStaging != nil }, set: { if !$0 { pendingStaging = nil } }), titleVisibility: .visible) {
            if let name = pendingStaging { Button("Delete Temporary Files", role: .destructive) { Task { await model.perform { try await model.store?.removeStaging(name) } }; pendingStaging = nil } }
        }
    }
}
struct DiagnosticsView: View {
    @EnvironmentObject var model: AppModel
    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Diagnostics").font(.largeTitle.bold())
            Text("Reports contain fixed event codes, timestamps and process exit status. Raw game output is discarded because it can contain authentication secrets.").foregroundStyle(.secondary)
            Button("Export Diagnostic Report…") { model.exportDiagnostics() }
            GroupBox("Validation scope") {
                Text("The bundled compatibility profile pins the known game build and DLL hashes. Live menu, gameplay, save persistence and multiplayer must be confirmed separately. Native ABI probes are maintained in the developer test suite.").padding(8).frame(maxWidth: .infinity, alignment: .leading)
            }
            if model.developerMode {
                Button("Open Launcher Data") { model.open(LibraryStore.defaultRoot) }
                if let current = model.current {
                    ForEach(current.hashes.sorted(by: { $0.key < $1.key }), id: \.key) { key, value in
                        VStack(alignment: .leading) { Text(key).font(.caption.bold()); Text(value).font(.caption.monospaced()).textSelection(.enabled) }
                    }
                }
            }
        }
    }
}

struct AccountView: View {
    var game: GameDefinition = .dungeons2
    @EnvironmentObject var model: AppModel
    @State private var signOut = false
    var body: some View {
        GroupBox("Microsoft package account") {
            VStack(alignment: .leading, spacing: 12) {
                LabeledContent("Account", value: model.accounts[game]?.gamertag ?? (model.accounts[game]?.signedIn == true ? "Connected" : "Not checked"))
                LabeledContent("Content license", value: model.accounts[game]?.entitlement == "verified" ? "Verified by Microsoft" : "Not verified")
                LabeledContent("Available package", value: model.accounts[game]?.availableVersion ?? "Not checked")
                HStack {
                    Button("Sign In") { Task { await model.accountOperation(.login, game: game) } }
                    Button("Check Ownership / Updates") { Task { await model.accountOperation(.check, game: game) } }
                    Button("Sign Out…") { signOut = true }
                }.disabled(model.busy || model.running)
                Text("Managed installations use this same Microsoft login through the macOS Keychain bridge. External reference installations retain their existing account.").font(.caption).foregroundStyle(.secondary)
            }.padding(8).frame(maxWidth: .infinity, alignment: .leading)
        }
        .confirmationDialog("Sign out of the native package account?", isPresented: $signOut, titleVisibility: .visible) {
            Button("Sign Out", role: .destructive) { Task { await model.accountOperation(.logout, game: game) } }
        } message: { Text("Removes only this launcher's Microsoft Keychain entries. The working game's current login stays intact. Sign in again to switch the native package account.") }
    }
}
