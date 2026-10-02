import SwiftUI
import AppKit
import LauncherCore

@main
struct MinecraftMacLauncherApp: App {
    @StateObject private var model = AppModel()
    init() { NSApplication.shared.setActivationPolicy(.regular) }
    var body: some Scene {
        WindowGroup("Minecraft on Mac") { LauncherView().environmentObject(model).frame(minWidth: 900, minHeight: 650).task { await model.load(); NSApplication.shared.activate(ignoringOtherApps: true) } }
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
    @State private var page: Page? = .dungeons
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
                        if progress.total > 0 { Text("\(progress.completed) / \(progress.total) files").font(.caption).foregroundStyle(.secondary) }
                    }.padding().frame(maxWidth: .infinity, alignment: .leading).background(.regularMaterial)
                }
                if !model.message.isEmpty { Text(model.message).font(.callout).foregroundStyle(.secondary).padding().frame(maxWidth: .infinity, alignment: .leading) }
            }.background(Color(nsColor: .windowBackgroundColor))
        }.navigationSplitViewColumnWidth(min: 220, ideal: 245)
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
            if let install = model.current {
                HStack(alignment: .top, spacing: 24) {
                    metric("Installed", "Version \(install.version)", "shippingbox")
                    metric("Compatibility", install.state == .ready ? "Reference hashes verified" : "Needs verification", "checkmark.shield")
                    metric("Storage", ByteCountFormatter.string(fromByteCount: install.diskBytes, countStyle: .file), "internaldrive")
                }
                GroupBox {
                    VStack(alignment: .leading, spacing: 8) {
                        LabeledContent("CrossOver bottle", value: install.bottle)
                        LabeledContent("Account", value: "Managed by the existing game login")
                        LabeledContent("Available version", value: "Not checked — package service pending")
                        LabeledContent("Installation", value: install.managed ? "Launcher-managed snapshot" : "External reference · protected")
                    }.padding(8)
                }
                HStack(spacing: 12) {
                    Button { Task { await model.play() } } label: { Label(model.running ? "Game running" : "Play", systemImage: "play.fill").padding(.horizontal, 24).padding(.vertical, 5) }
                        .buttonStyle(.borderedProminent).tint(Color(red: 0.16, green: 0.49, blue: 0.30)).disabled(model.busy || model.running)
                    Button("Verify") { Task { await model.verify(install) } }.disabled(model.busy || model.running)
                    Button("Repair") { Task { await model.repair(install) } }.disabled(!install.managed || model.busy || model.running)
                    Button("Open Folder") { model.open(install.path) }
                }
                Text("Reference launch preserves the existing Microsoft/Xbox login. File checks do not establish entitlement or validate gameplay, saves and multiplayer.").font(.caption).foregroundStyle(.secondary)
            } else {
                Text("Bring your existing installation").font(.title2.bold())
                Text("Register a locally installed copy of Dungeons II and choose its prepared CrossOver bottle. Your original game files stay where they are.").foregroundStyle(.secondary)
                BottlePicker()
                Button("Choose Installation…") { model.chooseInstallation() }.buttonStyle(.borderedProminent).disabled(model.busy || model.selectedBottle.isEmpty)
            }
            GroupBox("Fresh install and updates") {
                Text("Microsoft ownership and package integration is under development. Downloads stay unavailable until the native Keychain broker and verified installation receipts are implemented.").font(.callout).foregroundStyle(.secondary).padding(8).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }
    func metric(_ label: String, _ value: String, _ icon: String) -> some View {
        VStack(alignment: .leading, spacing: 8) { Label(label, systemImage: icon).font(.caption).foregroundStyle(.secondary); Text(value).font(.headline) }.frame(maxWidth: .infinity, alignment: .leading)
    }
}
struct BedrockView: View {
    var body: some View {
        ContentUnavailableView { Label("Minecraft: Bedrock", systemImage: "cube.fill") } description: { Text("Coming later\nBedrock installation and launch are not available in this release.") }
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
    @State private var cloneConfirmation: Installation?
    @State private var clearCurrent = false
    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Versions").font(.largeTitle.bold())
            Text("Keep a working installation while preparing a separate snapshot.").foregroundStyle(.secondary)
            ForEach(model.database.installations) { install in
                GroupBox {
                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            Text(install.version).font(.title2.bold())
                            if install.id == model.database.current { Text("Current").font(.caption.bold()).padding(5).background(.green.opacity(0.15), in: Capsule()) }
                            Spacer(); Text(install.managed ? "Managed snapshot" : "External reference").foregroundStyle(.secondary)
                        }
                        Text("\(ByteCountFormatter.string(fromByteCount: install.diskBytes, countStyle: .file)) • \(install.bottle) • \(install.state.rawValue)").font(.callout).foregroundStyle(.secondary)
                        HStack {
                            Button("Use") { Task { await model.select(install) } }.disabled(install.id == model.database.current || install.state != .ready)
                            Button("Verify") { Task { await model.verify(install) } }
                            Button("Create Snapshot…") { cloneConfirmation = install }
                            Button(install.managed ? "Remove…" : "Unregister…", role: .destructive) { pendingRemoval = install }.disabled(install.id == model.database.current)
                        }.disabled(model.busy || model.running)
                        if model.developerMode { Text(install.path.path).font(.caption.monospaced()).textSelection(.enabled); Text(install.compatibilityProfile).font(.caption.monospaced()) }
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
        .confirmationDialog("Remove this installation?", isPresented: Binding(get: { pendingRemoval != nil }, set: { if !$0 { pendingRemoval = nil } }), titleVisibility: .visible) {
            if let install = pendingRemoval { Button(install.managed ? "Remove Managed Game Files" : "Unregister Only", role: .destructive) { Task { await model.remove(install) }; pendingRemoval = nil } }
        } message: { Text("Managed game files are removed only from the recorded version folder. External files, saved games, login and CrossOver bottles are preserved.") }
        .confirmationDialog("Create a managed snapshot?", isPresented: Binding(get: { cloneConfirmation != nil }, set: { if !$0 { cloneConfirmation = nil } }), titleVisibility: .visible) {
            if let install = cloneConfirmation { Button("Clone and Verify") { Task { await model.clone(install) }; cloneConfirmation = nil } }
        } message: { Text("Copies this local installation using APFS copy-on-write and hashes all game files. This can take several minutes. Current selection stays available.") }
        .confirmationDialog("Clear current selection?", isPresented: $clearCurrent, titleVisibility: .visible) {
            Button("Clear Selection") { Task { await model.perform { try await model.store?.clearSelection() } } }
        } message: { Text("Play will be unavailable until you select another version. You can then remove the previous version.") }
    }
}
struct DownloadsView: View {
    var body: some View {
        ContentUnavailableView { Label("Downloads", systemImage: "arrow.down.circle") } description: {
            Text("No active downloads.\nFresh installations require the pending secure Microsoft entitlement and package integration.")
        }
    }
}
struct SettingsView: View {
    @EnvironmentObject var model: AppModel
    @State private var cacheConfirmation = false
    @State private var pendingStaging: String?
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
                    Text("Create or repair bottles in CrossOver. Existing user bottles are never deleted by this preview.").font(.caption).foregroundStyle(.secondary)
                }.padding(8).frame(maxWidth: .infinity, alignment: .leading)
            }
            GroupBox("Storage") {
                VStack(alignment: .leading, spacing: 10) {
                    Button("Open Launcher Data") { model.open(LibraryStore.defaultRoot) }
                    Button("Clear Download Cache…") { cacheConfirmation = true }.disabled(model.busy || model.running)
                    ForEach(model.staging, id: \.self) { name in
                        HStack { Text("Interrupted snapshot").font(.callout); Spacer(); Button("Delete Temporary Files…") { pendingStaging = name }.disabled(model.busy || model.running) }
                    }
                }.padding(8).frame(maxWidth: .infinity, alignment: .leading)
            }
            Toggle("Developer Mode", isOn: $model.developerMode)
            Text("No telemetry. Credentials are never included in diagnostics. Production account management is pending a secure native broker.").font(.caption).foregroundStyle(.secondary)
        }
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
