import SwiftUI
import AppKit
import LauncherCore

@MainActor
final class AppModel: ObservableObject {
    @Published var database = LibraryDatabase()
    @Published var crossOver = CrossOver.detect()
    @Published var progress: OperationProgress?
    @Published var error: LauncherError?
    @Published var runningGames: Set<GameDefinition> = []
    @Published var quittingGame: GameDefinition?
    private var requestedQuit: GameDefinition?
    @Published var running = false
    @Published var busy = false
    @Published var message = ""
    @Published var staging: [String] = []
    @Published var developerMode = UserDefaults.standard.bool(forKey: "developerMode") {
        didSet { UserDefaults.standard.set(developerMode, forKey: "developerMode") }
    }
    @Published var selectedBottle = ""
    @Published var selectedGame: GameDefinition = .dungeons2
    @Published var accounts: [GameDefinition: AccountStatus] = [:]
    var account: AccountStatus? {
        get { accounts[selectedGame] }
        set { accounts[selectedGame] = newValue }
    }
    let nativeAccount = NativeAccount()
    let processes = GameProcess()
    var store: LibraryStore?
    var diagnostics: Diagnostics?
    var current: Installation? { database.current(for: selectedGame) }
    func current(for game: GameDefinition) -> Installation? { database.current(for: game) }
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
        guard !database.installations.contains(where: { $0.gameId == "dungeons2" }), let store else { return }
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
        runningGames = GameTermination.runningGames(installations: database.installations, crossOver: crossOver)
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
    func play(game: GameDefinition? = nil) async {
        let game = game ?? selectedGame
        let current = database.current(for: game)
        crossOver = CrossOver.detect()
        guard let current, let store, let crossOver, !running, !busy else { return }
        error = nil
        await perform {
            if current.isOfficialManaged {
                let status = try await self.nativeAccount.request(.status, game: game)
                if !status.signedIn { self.accounts[game] = try await self.nativeAccount.request(.login, game: game) }
                self.accounts[game] = try await self.nativeAccount.request(.check, game: game)
            }
            if current.isOfficialManaged {
                let profile = try CompatibilityProfile.managed(for: game)
                if profile.hashes.contains(where: { current.hashes[$0.key] != $0.value }) {
                    // A launcher update supplies a new verified runtime; repair only owned files.
                    try await store.repairManaged(current.id, progress: self.progressHandler())
                } else { _ = try await store.reverify(current.id, progress: self.progressHandler()) }
            } else { _ = try await store.reverify(current.id, progress: self.progressHandler()) }
            try await self.diagnostics?.record(.installationVerified)
        }
        guard error == nil, database.installations.first(where: { $0.id == current.id })?.state == .ready else { return }
        requestedQuit = nil
        running = true; runningGames.insert(game)
        defer {
            running = CrossOver.gameRunning()
            runningGames = GameTermination.runningGames(installations: database.installations, crossOver: crossOver)
            if requestedQuit == game { requestedQuit = nil }
        }
        do {
            try await diagnostics?.record(.launchStarted)
            let exit = try await processes.launch(current, crossOver: crossOver)
            try await diagnostics?.record(.launchExited, exitCode: exit)
            message = requestedQuit == game ? "Game stopped." : "CrossOver launch process exited with status \(exit)."
            if exit != 0 && requestedQuit != game { error = LauncherError("GAME_EXITED", "The CrossOver launch process exited with status \(exit).", recovery: "Review Diagnostics and check the game in CrossOver.") }
        } catch { self.error = error as? LauncherError ?? LauncherError("LAUNCH_FAILED", "The game could not start.") }
    }
    func quitGame(_ game: GameDefinition) async {
        guard quittingGame == nil, let crossOver else { return }
        quittingGame = game; requestedQuit = game
        defer { quittingGame = nil }
        let installations = database.installations
        do {
            try await Task.detached {
                try GameTermination.forceQuit(game: game, installations: installations, crossOver: crossOver)
            }.value
            // Allow Wine's child exit and the launcher's account-service cleanup to finish.
            for _ in 0..<20 {
                await refresh()
                if !runningGames.contains(game) { break }
                try? await Task.sleep(for: .milliseconds(100))
            }
            if runningGames.contains(game) {
                error = LauncherError("GAME_QUIT_INCOMPLETE", "The game is still exiting.", recovery: "Refresh and retry if it remains unresponsive.")
            } else { message = "Game stopped." }
        } catch {
            requestedQuit = nil
            self.error = error as? LauncherError ?? LauncherError("GAME_QUIT_FAILED", "The game could not be stopped.")
            await refresh()
        }
    }

    func installAndPlay(game: GameDefinition? = nil, launch: Bool = true) async {
        let game = game ?? selectedGame
        crossOver = CrossOver.detect()
        guard let store, !busy, !running else { return }
        error = nil
        await perform {
            if self.crossOver == nil { throw LauncherError("CROSSOVER_MISSING", "Install and license CrossOver first. Open Settings for the official download.") }
            self.progress = .init("Checking Microsoft sign-in")
            self.accounts[game] = try await self.nativeAccount.request(.status, game: game)
            if self.accounts[game]?.signedIn != true { self.accounts[game] = try await self.nativeAccount.request(.login, game: game) }
            self.accounts[game] = try await self.nativeAccount.request(.check, game: game)
            guard self.accounts[game]?.entitlement == "verified" else { throw LauncherError("ENTITLEMENT_NOT_CONFIRMED", "Microsoft did not confirm ownership.") }
            if let available = self.accounts[game]?.availableVersion { try game.validateVersion(available) }
            _ = try await store.install(game: game, progress: self.progressHandler())
            self.message = "Game, Microsoft components and CrossOver environment are ready."
        }
        if error == nil && launch { await play(game: game) }
    }
    func checkUpdates(game requestedGame: GameDefinition? = nil) async {
        let game = requestedGame ?? selectedGame
        await accountOperation(.check, game: game)
        guard error == nil, let available = accounts[game]?.availableVersion else { return }
        if available == current(for: game)?.version && (current(for: game)?.packageRevision == nil || current(for: game)?.packageRevision == accounts[game]?.availableRevision) { message = "Your game is up to date." }
        else {
            do { try game.validateVersion(available) } catch { self.error = error as? LauncherError; return }
            await installAndPlay(game: game, launch: false)
        }
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
        let game = install.game
        guard let store else { return }
        if install.isOfficialManaged {
            await perform {
                self.accounts[game] = try await self.nativeAccount.request(.check, game: install.game)
                try await store.repairManaged(install.id, progress: self.progressHandler())
                self.message = "Compatibility repaired and verified. Game data and saved games are preserved."
            }
            return
        }
        guard let reference = database.installations.first(where: { $0.id != install.id && $0.gameId == install.gameId && $0.state == .ready }) else {
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
    func accountOperation(_ command: NativeAccount.Command, game: GameDefinition? = nil) async {
        let game = game ?? selectedGame
        // Clear old ownership evidence immediately, including after a failed new check.
        accounts[game] = nil; error = nil
        await perform {
            self.progress = .init(command == .login ? "Waiting for Microsoft sign-in" : "Checking Microsoft account")
            self.accounts[game] = try await self.nativeAccount.request(command, game: game)
            if command == .logout { self.accounts.removeAll() }
        }
    }
    func exportDiagnostics() {
        let panel = NSSavePanel(); panel.nameFieldStringValue = "MinecraftMac-diagnostics.json"
        guard panel.runModal() == .OK, let url = panel.url, let diagnostics else { return }
        Task { await perform { try await diagnostics.report().write(to: url, options: .atomic); self.message = "Diagnostic report exported. Contains only fixed event codes, dates and exit status." } }
    }
    func open(_ url: URL) { NSWorkspace.shared.open(url) }
}
