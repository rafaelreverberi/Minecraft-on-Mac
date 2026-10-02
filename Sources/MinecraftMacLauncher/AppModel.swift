import SwiftUI
import AppKit
import LauncherCore

@MainActor
final class AppModel: ObservableObject {
    @Published var database = LibraryDatabase()
    @Published var crossOver = CrossOver.detect()
    @Published var progress: OperationProgress?
    @Published var error: LauncherError?
    @Published var running = false
    @Published var busy = false
    @Published var message = ""
    @Published var staging: [String] = []
    @Published var developerMode = UserDefaults.standard.bool(forKey: "developerMode") {
        didSet { UserDefaults.standard.set(developerMode, forKey: "developerMode") }
    }
    @Published var selectedBottle = ""
    @Published var account: AccountStatus?
    let nativeAccount = NativeAccount()
    let processes = GameProcess()
    var store: LibraryStore?
    var diagnostics: Diagnostics?
    var current: Installation? { database.installations.first { $0.id == database.current } }
    var bottles: [String] { crossOver?.bottles() ?? [] }
    init() {
        do {
            store = try LibraryStore(); diagnostics = Diagnostics(root: LibraryStore.defaultRoot)
        } catch { self.error = error as? LauncherError ?? LauncherError("STARTUP_FAILED", "Launcher storage is unavailable.") }
        selectedBottle = bottles.contains("Dungeons2Xbox") ? "Dungeons2Xbox" : (bottles.first ?? "")
    }
    func load() async {
        await refresh()
        guard database.installations.isEmpty, let store else { return }
        let legacy = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Games/MinecraftDungeons2-FullyDecrypted")
        if FileManager.default.fileExists(atPath: legacy.appendingPathComponent("MicrosoftGame.config").path), !selectedBottle.isEmpty {
            await perform {
                _ = try await store.registerExternal(legacy, bottle: self.selectedBottle, progress: self.progressHandler())
                self.message = "Existing installation registered. Original files remain in their location."
            }
        }
    }
    func refresh() async {
        if let store { database = await store.snapshot(); staging = await store.staleStaging() }
        crossOver = CrossOver.detect(); running = CrossOver.gameRunning()
    }
    func progressHandler() -> @Sendable (OperationProgress) -> Void {
        { [weak self] update in Task { @MainActor in if let self, self.busy { self.progress = update } } }
    }
    func perform(_ operation: @MainActor () async throws -> Void) async {
        guard !busy, !running else { return }
        busy = true; message = ""; defer { busy = false; progress = nil }
        do { try await operation() } catch {
            self.error = error as? LauncherError ?? LauncherError("OPERATION_FAILED", "The operation could not finish safely.", recovery: "Your original installation is preserved. Review Diagnostics and retry.")
            try? await diagnostics?.record(.operationFailed)
        }
        await refresh()
    }
    func chooseInstallation() {
        let panel = NSOpenPanel(); panel.canChooseFiles = false; panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false; panel.message = "Select the folder containing MicrosoftGame.config."
        guard panel.runModal() == .OK, let path = panel.url, let store else { return }
        Task { await perform { _ = try await store.registerExternal(path, bottle: self.selectedBottle, progress: self.progressHandler()) } }
    }
    func play() async {
        guard let current, let store, let crossOver, !running, !busy else { return }
        error = nil
        await perform {
            _ = try await store.reverify(current.id, progress: self.progressHandler())
            try await self.diagnostics?.record(.installationVerified)
        }
        guard error == nil, database.installations.first(where: { $0.id == current.id })?.state == .ready else { return }
        running = true; defer { running = CrossOver.gameRunning() }
        do {
            try await diagnostics?.record(.launchStarted)
            let exit = try await processes.launch(current, crossOver: crossOver)
            try await diagnostics?.record(.launchExited, exitCode: exit)
            message = "CrossOver launch process exited with status \(exit)."
            if exit != 0 { error = LauncherError("GAME_EXITED", "The CrossOver launch process exited with status \(exit).", recovery: "Review Diagnostics and check the game in CrossOver.") }
        } catch { self.error = error as? LauncherError ?? LauncherError("LAUNCH_FAILED", "The game could not start.") }
    }
    func clone(_ install: Installation) async {
        guard let store else { return }
        await perform {
            _ = try await store.clone(install.id, progress: self.progressHandler())
            self.message = "Managed snapshot created. Select it in Versions to use it."
            try await self.diagnostics?.record(.snapshotCreated)
        }
    }
    func verify(_ install: Installation) async {
        guard let store else { return }
        await perform { _ = try await store.reverify(install.id, progress: self.progressHandler()); self.message = "File and CrossOver readiness checks passed." }
    }
    func repair(_ install: Installation) async {
        guard let store else { return }
        guard let reference = database.installations.first(where: { $0.id != install.id && $0.state == .ready }) else {
            error = LauncherError("REPAIR_SOURCE_REQUIRED", "Register a separate verified installation to repair this snapshot."); return
        }
        await perform { try await store.repairCompatibility(install.id, referenceID: reference.id, progress: self.progressHandler()); self.message = "Compatibility files verified and repaired." }
    }
    func select(_ install: Installation) async {
        guard let store else { return }; await perform { try await store.select(install.id) }
    }
    func remove(_ install: Installation) async {
        guard let store else { return }
        await perform { try await store.remove(install.id); try await self.diagnostics?.record(.versionRemoved) }
    }
    func accountOperation(_ command: NativeAccount.Command) async {
        // Clear old ownership evidence immediately, including after a failed new check.
        account = nil
        await perform {
            self.progress = .init(command == .login ? "Waiting for Microsoft sign-in" : "Checking Microsoft account")
            self.account = try await self.nativeAccount.request(command)
        }
    }
    func exportDiagnostics() {
        let panel = NSSavePanel(); panel.nameFieldStringValue = "MinecraftMac-diagnostics.json"
        guard panel.runModal() == .OK, let url = panel.url, let diagnostics else { return }
        Task { await perform { try await diagnostics.report().write(to: url, options: .atomic); self.message = "Diagnostic report exported. Contains only fixed event codes, dates and exit status." } }
    }
    func open(_ url: URL) { NSWorkspace.shared.open(url) }
}
