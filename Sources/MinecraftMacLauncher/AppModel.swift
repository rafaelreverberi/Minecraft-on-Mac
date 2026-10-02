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
        account = try? await nativeAccount.request(.status)
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
        busy = true; error = nil; message = ""; defer { busy = false; progress = nil }
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
        crossOver = CrossOver.detect()
        guard let current, let store, let crossOver, !running, !busy else { return }
        error = nil
        await perform {
            if current.compatibilityProfile.hasPrefix("dungeons2-managed-") {
                let status = try await self.nativeAccount.request(.status)
                if !status.signedIn { self.account = try await self.nativeAccount.request(.login) }
                self.account = try await self.nativeAccount.request(.check)
            }
            if current.compatibilityProfile.hasPrefix("dungeons2-managed-") {
                let profile = try CompatibilityProfile.managed()
                if profile.hashes.contains(where: { current.hashes[$0.key] != $0.value }) {
                    // A launcher update supplies a new verified runtime; repair only owned files.
                    try await store.repairManaged(current.id, progress: self.progressHandler())
                } else { _ = try await store.reverify(current.id, progress: self.progressHandler()) }
            } else { _ = try await store.reverify(current.id, progress: self.progressHandler()) }
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
    func installAndPlay() async {
        crossOver = CrossOver.detect()
        guard let store, !busy, !running else { return }
        error = nil
        await perform {
            if self.crossOver == nil { throw LauncherError("CROSSOVER_MISSING", "Install and license CrossOver first. Open Settings for the official download.") }
            self.progress = .init("Checking Microsoft sign-in")
            self.account = try await self.nativeAccount.request(.status)
            if self.account?.signedIn != true { self.account = try await self.nativeAccount.request(.login) }
            self.account = try await self.nativeAccount.request(.check)
            guard self.account?.entitlement == "verified" else { throw LauncherError("ENTITLEMENT_NOT_CONFIRMED", "Microsoft did not confirm ownership.") }
            _ = try await store.install(progress: self.progressHandler())
            self.message = "Game, Microsoft components and CrossOver environment are ready."
        }
        if error == nil { await play() }
    }
    func checkUpdates() async {
        await accountOperation(.check)
        guard error == nil, let available = account?.availableVersion else { return }
        if available == current?.version && (current?.packageRevision == nil || current?.packageRevision == account?.availableRevision) { message = "Your game is up to date." }
        else if available != "1.1.1.0" { error = LauncherError("COMPATIBILITY_UNVERIFIED", "A new game version is available, but its compatibility profile has not been verified yet.", recovery: "The current working version is preserved. Update the launcher when support for this version is released.") }
        else { await installAndPlay() }
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
        if install.compatibilityProfile.hasPrefix("dungeons2-managed-") {
            await perform {
                self.account = try await self.nativeAccount.request(.check)
                try await store.repairManaged(install.id, progress: self.progressHandler())
                self.message = "Compatibility repaired and verified. Game data and saved games are preserved."
            }
            return
        }
        guard let reference = database.installations.first(where: { $0.id != install.id && $0.state == .ready }) else {
            error = LauncherError("REPAIR_SOURCE_REQUIRED", "Register a separate verified installation to repair this snapshot."); return
        }
        await perform { try await store.repairCompatibility(install.id, referenceID: reference.id, progress: self.progressHandler()); self.message = "Compatibility files verified and repaired." }
    }
    func select(_ install: Installation) async {
        guard let store else { return }; await perform { try await store.select(install.id) }
    }
    func remove(_ install: Installation, removeEnvironment: Bool = false) async {
        guard let store else { return }
        await perform { try await store.remove(install.id, removeEnvironment: removeEnvironment); try await self.diagnostics?.record(.versionRemoved) }
    }
    func accountOperation(_ command: NativeAccount.Command) async {
        // Clear old ownership evidence immediately, including after a failed new check.
        account = nil; error = nil
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
