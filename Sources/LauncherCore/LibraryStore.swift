import Foundation
import Darwin

public actor LibraryStore {
    public let root: URL
    private let profile: CompatibilityProfile
    private let crossOver: CrossOver?
    private let runtimeTests: Bool
    private let isGameRunning: @Sendable () -> Bool
    private var database: LibraryDatabase
    private var busy = false
    private var lockFD: Int32 = -1
    private let fm = FileManager.default
    private var databaseURL: URL { root.appendingPathComponent("library.json") }
    public static var defaultRoot: URL {
        FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support/Minecraft on Mac")
    }
    public init(root: URL = LibraryStore.defaultRoot, profile: CompatibilityProfile? = nil, crossOver: CrossOver? = .detect(), runtimeTests: Bool = true, isGameRunning: @escaping @Sendable () -> Bool = { CrossOver.gameRunning() }) throws {
        self.root = root; self.profile = try profile ?? .bundled(); self.crossOver = crossOver; self.runtimeTests = runtimeTests; self.isGameRunning = isGameRunning
        _ = try FileSafety.child("library.json", of: root)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        let fd = Darwin.open(root.appendingPathComponent(".library-lock").path, O_CREAT | O_RDWR | O_NOFOLLOW, 0o600)
        guard fd >= 0, flock(fd, LOCK_EX | LOCK_NB) == 0 else {
            if fd >= 0 { close(fd) }
            throw LauncherError("LIBRARY_IN_USE", "Another launcher owns the library.", recovery: "Quit the other launcher instance and try again.")
        }
        do {
            let url = root.appendingPathComponent("library.json")
            if FileManager.default.fileExists(atPath: url.path) {
                self.database = try JSONDecoder().decode(LibraryDatabase.self, from: Data(contentsOf: url))
                guard database.schema == 2 else { throw LauncherError("DATABASE_VERSION", "Library schema is unsupported.") }
            } else { self.database = LibraryDatabase() }
            for folder in ["Games/dungeons2/Versions", "Games/bedrock/Versions", "Compatibility", "Cache", "Logs", "Tools"] {
                let path = try FileSafety.child(folder, of: root)
                try FileManager.default.createDirectory(at: path, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            }
            // Recover a deletion interrupted between folder rename and database commit.
            for install in database.installations where install.managed {
                let final = try FileSafety.child("\(install.version)-\(install.id.uuidString)", of: try install.game.versionRoot(in: root))
                let staging = try FileSafety.child(".staging-\(install.id.uuidString)", of: try install.game.versionRoot(in: root))
                if install.path.path == final.appendingPathComponent("Game").path, !fm.fileExists(atPath: final.path), fm.fileExists(atPath: staging.path) {
                    let receipt = try JSONDecoder().decode(Installation.self, from: Data(contentsOf: FileSafety.child("metadata.json", of: staging)))
                    guard receipt.id == install.id, receipt.gameId == install.gameId, receipt.managed, receipt.bottle == install.bottle else { throw LauncherError("MANIFEST_MISMATCH", "Interrupted deletion receipt does not match.") }
                    try FileSafety.validateTree(staging); try fm.moveItem(at: staging, to: final)
                }
            }
            let cacheReceipt = try FileSafety.child("cache-manifest.json", of: root)
            if !FileManager.default.fileExists(atPath: cacheReceipt.path) { try FileSafety.write([String:String](), to: cacheReceipt) }
            let dbURL = root.appendingPathComponent("library.json")
            if fm.fileExists(atPath: dbURL.path), let raw = try JSONSerialization.jsonObject(with: Data(contentsOf: dbURL)) as? [String:Any], raw["schema"] as? Int == 1 {
                let backup = try FileSafety.child("library.schema1.backup.json", of: root)
                if !fm.fileExists(atPath: backup.path) { try fm.copyItem(at: dbURL, to: backup) }
                try FileSafety.write(database, to: dbURL)
            }
            self.lockFD = fd
        } catch {
            flock(fd, LOCK_UN); close(fd)
            throw LauncherError("DATABASE_UNAVAILABLE", "The library could not be read safely.", recovery: "Preserve library.json and review file permissions. It has not been reset.")
        }
    }
    deinit { if lockFD >= 0 { flock(lockFD, LOCK_UN); close(lockFD) } }
    public func snapshot() -> LibraryDatabase {
        var result = database
        for index in result.installations.indices where result.installations[index].managed {
            let install = result.installations[index]
            if let folder = try? FileSafety.child("\(install.version)-\(install.id.uuidString)", of: versionsRoot(install.game)), install.path.path == folder.appendingPathComponent("Game").path {
                result.installations[index].storageBytes = try? FileSafety.diskUsage(folder)
            }
        }
        return result
    }
    public func staleStaging() -> [String] {
        GameDefinition.allCases.flatMap { game in
            ((try? fm.contentsOfDirectory(atPath: game.versionRoot(in: root).path)) ?? []).filter { $0.hasPrefix(".staging-") }.map { game.rawValue + "/" + $0 }
        }.sorted()
    }
    private func versionsRoot(_ game: GameDefinition) throws -> URL { try game.versionRoot(in: root) }
    private func checkIdle() throws {
        guard !busy else { throw LauncherError("OPERATION_BUSY", "A library operation is in progress.") }
        guard !isGameRunning() else { throw LauncherError("GAME_RUNNING", "Quit running games before changing the library.") }
    }
    private func persist(_ next: LibraryDatabase) throws {
        _ = try FileSafety.child("library.json", of: root)
        try next.validate(); try FileSafety.write(next, to: databaseURL); database = next
    }
    public func verify(_ install: Installation, progress: @Sendable (OperationProgress) -> Void = { _ in }) throws -> Installation {
        let profile = install.isOfficialManaged ? try CompatibilityProfile.managed(for: install.game) : self.profile
        guard profile.gameId == install.gameId else { throw LauncherError("GAME_IDENTITY_INVALID", "Compatibility profile belongs to another game.") }
        try install.game.validateVersion(install.version)
        guard try FileSafety.version(install.path, game: install.game) == profile.version, install.version == profile.version else {
            throw LauncherError("COMPATIBILITY_UNVERIFIED", "This build has no verified compatibility profile.", recovery: "Keep using the current verified build. Do not apply its DLLs to this build.")
        }
        if install.managed {
            let versions = try versionsRoot(install.game)
            let parent = install.path.deletingLastPathComponent()
            let allowed = ["\(install.version)-\(install.id.uuidString)", ".staging-\(install.id.uuidString)"]
            guard parent.deletingLastPathComponent().path == versions.path, allowed.contains(parent.lastPathComponent), install.path.lastPathComponent == "Game" else { throw LauncherError("MANIFEST_MISMATCH", "Managed installation escaped its game root.") }
            let receipt = try JSONDecoder().decode(Installation.self, from: Data(contentsOf: FileSafety.child("metadata.json", of: parent)))
            guard receipt.id == install.id, receipt.gameId == install.gameId, receipt.storeId == install.storeId, receipt.bottle == install.bottle, receipt.managed else { throw LauncherError("MANIFEST_MISMATCH", "Managed installation receipt differs.") }
        }
        var verified = install
        let hashes = install.hashes.isEmpty ? profile.hashes : install.hashes
        for (index, pair) in hashes.sorted(by: { $0.key < $1.key }).enumerated() {
            try Task.checkCancellation()
            progress(.init("Verifying files", completed: Int64(index), total: Int64(hashes.count)))
            let path = try FileSafety.child(pair.key, of: install.path)
            guard fm.fileExists(atPath: path.path), try FileSafety.hash(path) == pair.value else {
                throw LauncherError("FILE_INTEGRITY_FAILED", "An installation file is missing or changed.", recovery: "Repair compatibility files or restore the affected version from a verified package.")
            }
            if profile.hashes[pair.key] != nil && (path.pathExtension.lowercased() == "dll" || path.pathExtension.lowercased() == "exe") {
                guard try FileSafety.isX64PE(path) else { throw LauncherError("ARCHITECTURE_INVALID", "A required executable is not x86_64 PE.") }
            }
        }
        // Always bind trusted profile hashes as well, even if database hashes were modified.
        for (relative, expected) in profile.hashes {
            guard try FileSafety.hash(FileSafety.child(relative, of: install.path)) == expected else {
                throw LauncherError("COMPATIBILITY_FAILED", "Compatibility files no longer match the bundled profile.")
            }
        }
        guard let crossOver = self.crossOver ?? .detect() else { throw LauncherError("CROSSOVER_MISSING", "Install CrossOver to use Dungeons.") }
        if install.game == .bedrock {
            try BedrockRuntime.validate(install, root: root, crossOver: crossOver, runProbes: runtimeTests)
            verified.state = .ready; verified.lastSuccessfulTest = Date(); verified.compatibilityProfile = profile.id
            return verified
        }
        let bottle = try crossOver.bottleURL(install.bottle)
        let runtimeHash = profile.hashes["Dungeons/Binaries/WinGDK/xgameruntime.dll"]
        for location in [try FileSafety.child("xgameruntime.dll", of: install.path), bottle.appendingPathComponent("drive_c/windows/system32/xgameruntime.dll")] {
            guard try FileSafety.hash(location) == runtimeHash else {
                throw LauncherError("RUNTIME_MISMATCH", "The game root or bottle runtime differs from the tested configuration.", recovery: "Repair the bottle in CrossOver. Launcher does not overwrite a user-owned bottle.")
            }
        }
        if runtimeTests { try RuntimeSelfTests.run(install, crossOver: crossOver, progress: progress) }
        verified.state = .ready; verified.lastSuccessfulTest = Date(); verified.compatibilityProfile = profile.id
        if verified.hashes.isEmpty { verified.hashes = profile.hashes }
        progress(.init("Verified", completed: Int64(hashes.count), total: Int64(hashes.count)))
        return verified
    }
    public func registerExternal(_ path: URL, bottle: String, progress: @Sendable (OperationProgress) -> Void = { _ in }) throws -> Installation {
        try checkIdle(); busy = true; defer { busy = false }
        guard !database.installations.contains(where: { $0.path.standardizedFileURL == path.standardizedFileURL }) else {
            throw LauncherError("ALREADY_REGISTERED", "This installation is already in the library.")
        }
        try FileSafety.validateTree(path)
        var install = Installation(version: try FileSafety.version(path), path: path, bottle: bottle, managed: false)
        install = try verify(install, progress: progress)
        progress(.init("Measuring storage")); install.diskBytes = try FileSafety.diskUsage(path)
        var next = database; next.installations.append(install); if next.currentByGameId[install.gameId] == nil { next.currentByGameId[install.gameId] = install.id }
        try persist(next); return install
    }
    public func reverify(_ id: UUID, progress: @Sendable (OperationProgress) -> Void = { _ in }) throws -> Installation {
        try checkIdle(); busy = true; defer { busy = false }
        guard let index = database.installations.firstIndex(where: { $0.id == id }) else { throw LauncherError("VERSION_MISSING", "Installation is unavailable.") }
        do {
            let install = try verify(database.installations[index], progress: progress)
            var next = database; next.installations[index] = install; try persist(next); return install
        } catch {
            var next = database; next.installations[index].state = .failed; try persist(next); throw error
        }
    }
    public func select(_ id: UUID) throws {
        try checkIdle()
        guard let install = database.installations.first(where: { $0.id == id }), install.state == .ready else { throw LauncherError("VERSION_NOT_READY", "Only a verified installation can be selected.") }
        _ = try verify(install)
        var next = database; next.currentByGameId[install.gameId] = id; try persist(next)
    }
    public func changeBottle(_ id: UUID, bottle: String) throws {
        try checkIdle()
        guard let index = database.installations.firstIndex(where: { $0.id == id }) else { throw LauncherError("VERSION_MISSING", "Installation is unavailable.") }
        var install = database.installations[index]; install.bottle = bottle
        install = try verify(install)
        var next = database; next.installations[index] = install; try persist(next)
    }
    /// Copy-on-write snapshot. No entitlement inferred; this is a copy of a user-selected local installation.
    public func clone(_ id: UUID, progress: @Sendable (OperationProgress) -> Void = { _ in }) async throws -> Installation {
        try checkIdle(); busy = true; defer { busy = false }
        guard let source = database.installations.first(where: { $0.id == id }) else { throw LauncherError("VERSION_MISSING", "Installation is unavailable.") }
        _ = try verify(source, progress: progress); try FileSafety.validateTree(source.path)
        let newID = UUID(); let stageName = ".staging-\(newID.uuidString)"
        let stage = try FileSafety.child(stageName, of: versionsRoot(source.game))
        try fm.createDirectory(at: stage, withIntermediateDirectories: false)
        let destination = stage.appendingPathComponent("Game")
        var install = Installation(id: newID, game: source.game, version: source.version, path: destination, bottle: source.bottle, managed: true, compatibilityProfile: source.compatibilityProfile)
        try FileSafety.write(install, to: stage.appendingPathComponent("metadata.json"))
        do {
            progress(.init("Cloning local installation"))
            try await Self.copyClone(from: source.path, to: destination)
            try Task.checkCancellation(); try FileSafety.validateTree(destination)
            install = try verify(install, progress: progress)
            // Hash all copied game files. Future repair distinguishes game data from compatibility.
            let items = fm.enumerator(at: destination, includingPropertiesForKeys: [.isRegularFileKey])
            var all: [String:String] = [:]; var count: Int64 = 0
            while let file = items?.nextObject() as? URL {
                try Task.checkCancellation()
                guard try file.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile == true else { continue }
                progress(.init("Recording file integrity", completed: count))
                let relative = String(file.path.dropFirst(destination.path.count + 1))
                all[relative] = try FileSafety.hash(file); count += 1
            }
            install.origin = "snapshot"
            install.storageBytes = try FileSafety.diskUsage(destination)
            install.packageRevision = source.packageRevision
            install.hashes = all; install.diskBytes = try FileSafety.diskUsage(destination)
            let final = try FileSafety.child("\(source.version)-\(newID.uuidString)", of: versionsRoot(source.game))
            install.path = final.appendingPathComponent("Game")
            try FileSafety.write(install, to: stage.appendingPathComponent("metadata.json"))
            try fm.moveItem(at: stage, to: final)
            var next = database; next.installations.append(install)
            do { try persist(next) } catch {
                // Orphan is explicitly marked staging for recovery; current selection is unchanged.
                try? fm.moveItem(at: final, to: stage); throw error
            }
            progress(.init("Snapshot complete", completed: 1, total: 1)); return install
        } catch {
            // Keep manifest-backed interrupted staging visible. Never expose it as ready.
            throw error
        }
    }
    /// Prepare, test and commit a new official package without changing the working selection on failure.
    public func install(game definition: GameDefinition = .dungeons2, progress: @escaping @Sendable (OperationProgress) -> Void = { _ in }) async throws -> Installation {
        try checkIdle(); busy = true; defer { busy = false }
        guard let crossOver = self.crossOver ?? .detect() else { throw LauncherError("CROSSOVER_MISSING", "Install and license CrossOver first.", recovery: "Open Settings for the official CrossOver download. No developer tools are needed.") }
        let profile = try CompatibilityProfile.managed(for: definition)
        if definition == .bedrock { try crossOver.requireBedrockVersion() }
        guard let assets = Bundle.module.url(forResource: "Managed", withExtension: nil, subdirectory: "Resources") else { throw LauncherError("COMPONENT_MISSING", "Use the complete packaged app.") }
        let capacity = try root.resourceValues(forKeys: [.volumeAvailableCapacityForImportantUsageKey]).volumeAvailableCapacityForImportantUsage ?? 0
        guard capacity >= 35 * 1024 * 1024 * 1024 else { throw LauncherError("STORAGE_REQUIRED", "At least 35 GB of available storage is required for the game and temporary downloads.") }
        let id = UUID(); let stage = try FileSafety.child(".staging-\(id.uuidString)", of: versionsRoot(definition))
        try fm.createDirectory(at: stage, withIntermediateDirectories: false, attributes: [.posixPermissions:0o700])
        let game = stage.appendingPathComponent("Game")
        var install = Installation(id: id, game: definition, version: profile.version, path: game, bottle: database.installations.first(where: { $0.gameId == definition.rawValue && $0.isOfficialManaged })?.bottle ?? database.retainedEnvironmentsByGameId[definition.rawValue]?.first ?? (definition == .dungeons2 ? "MinecraftMac-\(id.uuidString)" : "MinecraftMac-bedrock-\(id.uuidString)"), managed: true, state: .preparing, compatibilityProfile: profile.id)
        try FileSafety.write(install, to: stage.appendingPathComponent("metadata.json"))
        try await ManagedInstaller.acquire(stage: stage, game: definition, progress: progress)
        try Task.checkCancellation(); try FileSafety.validateTree(stage)
        struct Receipt: Decodable { let schema: Int; let version: String; let revision: String?; let gameId: String; let storeId: String; let materialization: String; let hashes: [String:String] }
        let receipt = try JSONDecoder().decode(Receipt.self, from: Data(contentsOf: FileSafety.child("download-receipt.json", of: stage)))
        guard receipt.schema == 1, receipt.gameId == definition.rawValue, receipt.storeId == definition.storeId, receipt.materialization == "persistent-ssd", receipt.version == profile.version, !receipt.hashes.isEmpty else { throw LauncherError("PACKAGE_INVALID", "The package receipt is invalid.") }
        for (relative, expected) in receipt.hashes {
            let base = relative.hasPrefix("Components/") ? stage : game
            guard try FileSafety.hash(FileSafety.child(relative, of: base)) == expected else { throw LauncherError("PACKAGE_INVALID", "Downloaded package files failed verification.") }
        }
        install.origin = "download"
        install.packageRevision = receipt.revision
        if definition == .bedrock {
            let runtime = try BedrockRuntime.prepare(root: root)
            for (name, hash) in profile.hashes {
                let source = try FileSafety.child("x86_64-windows/" + name, of: runtime)
                guard try FileSafety.hash(source) == hash else { throw LauncherError("BEDROCK_RUNTIME_INVALID", "Bundled component changed.") }
                try ManagedInstaller.replace(source, destination: FileSafety.child(name, of: game))
            }
            for (source, target) in [("xgameruntime.gdk.dll", "xgameruntime.dll.threading"), ("XCurl.dll", "XCurl.dll"), ("curl-ca-bundle.crt", "curl-ca-bundle.crt")] {
                try ManagedInstaller.replace(FileSafety.child("Components/" + source, of: stage), destination: FileSafety.child(target, of: game))
            }
            let cert = try FileSafety.child("etc/ssl/certs/ca-bundle.crt", of: game)
            try fm.createDirectory(at: cert.deletingLastPathComponent(), withIntermediateDirectories: true)
            try ManagedInstaller.replace(FileSafety.child("curl-ca-bundle.crt", of: game), destination: cert)
            progress(.init("Preparing Bedrock CrossOver environment"))
            let owner = UUID(uuidString: String(install.bottle.dropFirst("MinecraftMac-bedrock-".count))) ?? id
            try await BedrockRuntime.provision(install, root: root, crossOver: crossOver, owner: owner)
        } else {
        let bin = try FileSafety.child("Dungeons/Binaries/WinGDK", of: game)
        for name in ["XCurl.dll", "xgameruntime.dll"] {
            let source = try FileSafety.child(name, of: assets)
            let expected = profile.hashes["Dungeons/Binaries/WinGDK/" + name]
            guard try FileSafety.hash(source) == expected else { throw LauncherError("COMPONENT_INTEGRITY_FAILED", "Bundled compatibility files failed verification.") }
            try ManagedInstaller.replace(source, destination: try FileSafety.child(name, of: bin))
        }
        for name in ["XCurl2504.dll", "xgameruntime.gdk.dll"] {
            let source = try FileSafety.child("Components/" + name, of: stage)
            guard try FileSafety.hash(source) == profile.hashes["Dungeons/Binaries/WinGDK/" + name] else { throw LauncherError("COMPONENT_INTEGRITY_FAILED", "Microsoft component checksum differs from the tested version.") }
            try ManagedInstaller.replace(source, destination: try FileSafety.child(name, of: bin))
        }
        let runtime = try FileSafety.child("xgameruntime.dll", of: bin)
        try ManagedInstaller.replace(runtime, destination: try FileSafety.child("xgameruntime.dll", of: game))
        progress(.init("Preparing CrossOver environment"))
        try await ManagedInstaller.provisionBottle(install.bottle, crossOver: crossOver, components: stage.appendingPathComponent("Components"), runtime: runtime, owner: UUID(uuidString: String(install.bottle.dropFirst("MinecraftMac-".count))) ?? id)
        }
        install.state = .testing
        try FileSafety.write(install, to: stage.appendingPathComponent("metadata.json"))
        let walk = fm.enumerator(at: game, includingPropertiesForKeys: [.isRegularFileKey])
        while let file = walk?.nextObject() as? URL {
            guard try file.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile == true else { continue }
            install.hashes[String(file.path.dropFirst(game.path.count + 1))] = try FileSafety.hash(file)
        }
        install.diskBytes = try FileSafety.diskUsage(game)
        install.storageBytes = try FileSafety.diskUsage(stage)
        install = try verify(install, progress: progress)
        let final = try FileSafety.child("\(install.version)-\(id.uuidString)", of: versionsRoot(install.game))
        install.path = final.appendingPathComponent("Game")
        try FileSafety.write(install, to: stage.appendingPathComponent("metadata.json"))
        try fm.moveItem(at: stage, to: final)
        var next = database; next.installations.append(install); next.currentByGameId[install.gameId] = install.id
        next.retainedEnvironmentsByGameId[install.gameId]?.removeAll { $0 == install.bottle }
        do { try persist(next) } catch { try? fm.moveItem(at: final, to: stage); throw error }
        progress(.init("Ready to play", completed: 1, total: 1)); return install
    }
    private static func copyClone(from: URL, to: URL) async throws {
        try await Task.detached {
            let process = Process(); process.executableURL = URL(fileURLWithPath: "/bin/cp")
            process.arguments = ["-cR", from.path, to.path]
            process.standardOutput = FileHandle.nullDevice; process.standardError = FileHandle.nullDevice
            try process.run(); process.waitUntilExit()
            guard process.terminationStatus == 0 else {
                throw LauncherError("CLONE_FAILED", "APFS copy-on-write cloning failed.", recovery: "Use an APFS volume. Temporary files are available in Storage for cleanup.")
            }
        }.value
    }
    public func remove(_ id: UUID, removeEnvironment: Bool = false) throws {
        try checkIdle()
        guard let install = database.installations.first(where: { $0.id == id }) else { throw LauncherError("VERSION_MISSING", "Installation is unavailable.") }
        var environment: URL?
        if removeEnvironment {
            guard install.managed, install.isOfficialManaged, !database.installations.contains(where: { $0.id != id && $0.bottle == install.bottle }),
                  let crossOver = self.crossOver ?? .detect() else { throw LauncherError("ENVIRONMENT_SHARED", "Only an owned environment unused by other versions can be deleted.") }
            let bottle = try crossOver.bottleURL(install.bottle)
            try ManagedInstaller.validateOwner(bottle: bottle, name: install.bottle, game: install.game)
            environment = bottle
        }
        var next = database; next.installations.removeAll { $0.id == id }
        if next.currentByGameId[install.gameId] == id { next.currentByGameId[install.gameId] = nil }
        if install.managed && install.isOfficialManaged {
            var retained = next.retainedEnvironmentsByGameId[install.gameId] ?? []
            retained.removeAll { $0 == install.bottle }
            if !removeEnvironment && !next.installations.contains(where: { $0.bottle == install.bottle }) { retained.append(install.bottle) }
            next.retainedEnvironmentsByGameId[install.gameId] = retained
        }
        if install.managed {
            let expected = try FileSafety.child("\(install.version)-\(install.id.uuidString)", of: versionsRoot(install.game))
            guard install.path.path == expected.appendingPathComponent("Game").path else { throw LauncherError("MANIFEST_MISMATCH", "Managed path does not match its recorded version.") }
            let receipt = try JSONDecoder().decode(Installation.self, from: Data(contentsOf: FileSafety.child("metadata.json", of: expected)))
            guard receipt.id == id, receipt.gameId == install.gameId, receipt.managed, receipt.bottle == install.bottle else { throw LauncherError("MANIFEST_MISMATCH", "Installation receipt does not match.") }
            try FileSafety.validateTree(expected)
            for relative in install.game.localSavePaths {
                if fm.fileExists(atPath: expected.appendingPathComponent("Game").appendingPathComponent(relative).path) { throw LauncherError("LOCAL_SAVES_PRESENT", "This older version contains local saves.", recovery: "Preserve or migrate these saves first. They were not deleted.") }
            }
            // A crash leaves receipt-backed staging, never a partially deleted playable version.
            let staging = try FileSafety.child(".staging-\(id.uuidString)", of: versionsRoot(install.game))
            guard !fm.fileExists(atPath: staging.path) else { throw LauncherError("OPERATION_BUSY", "Temporary cleanup for this version already exists.") }
            try fm.moveItem(at: expected, to: staging)
            do { try persist(next) } catch { try? fm.moveItem(at: staging, to: expected); throw error }
            do { try fm.removeItem(at: staging) } catch { throw LauncherError("CLEANUP_PENDING", "The version is unregistered; temporary files still need cleanup in Storage.") }
        } else { try persist(next) }
        if let environment {
            // Foundation removes symlinks themselves; CrossOver intentionally links host folders.
            do { try fm.removeItem(at: environment) } catch { throw LauncherError("ENVIRONMENT_CLEANUP_FAILED", "The version was deleted, but its environment could not be fully removed.", recovery: "Review the remaining owned environment in CrossOver. Microsoft login is preserved.") }
        }
    }
    public func removeRetainedEnvironment(_ name: String) throws {
        guard let gameId = database.retainedEnvironmentsByGameId.first(where: { $0.value.contains(name) })?.key, let game = GameDefinition(rawValue: gameId) else { throw LauncherError("ENVIRONMENT_SHARED", "Unknown retained environment.") }
        try checkIdle()
        guard database.retainedEnvironmentsByGameId[gameId]?.contains(name) == true, !database.installations.contains(where: { $0.bottle == name }), let crossOver = self.crossOver ?? .detect() else { throw LauncherError("ENVIRONMENT_SHARED", "This environment is not retained or is still in use.") }
        let bottle = try crossOver.bottleURL(name)
        try ManagedInstaller.validateOwner(bottle: bottle, name: name, game: game)
        try fm.removeItem(at: bottle)
        var next = database; next.retainedEnvironmentsByGameId[gameId]?.removeAll { $0 == name }; try persist(next)
    }
    public func clearSelection(game: GameDefinition = .dungeons2) throws { try checkIdle(); var next = database; next.currentByGameId[game.rawValue] = nil; try persist(next) }
    public func removeStaging(_ entry: String) throws {
        let parts = entry.split(separator: "/")
        let game = parts.count == 2 ? GameDefinition(rawValue: String(parts[0])) : .dungeons2
        guard let game, parts.count <= 2 else { throw LauncherError("UNSAFE_PATH", "Invalid staging game.") }
        let name = String(parts.last ?? "")
        try checkIdle()
        guard name.hasPrefix(".staging-"), UUID(uuidString: String(name.dropFirst(9))) != nil else { throw LauncherError("UNSAFE_PATH", "Invalid staging directory.") }
        let path = try FileSafety.child(name, of: versionsRoot(game))
        let receipt = try JSONDecoder().decode(Installation.self, from: Data(contentsOf: FileSafety.child("metadata.json", of: path)))
        guard receipt.managed, receipt.gameId == game.rawValue, name == ".staging-\(receipt.id.uuidString)" else { throw LauncherError("MANIFEST_MISMATCH", "Temporary installation receipt does not match.") }
        try FileSafety.validateTree(path); try fm.removeItem(at: path)
    }
    public func clearCache() throws {
        try checkIdle()
        let path = try FileSafety.child("Cache", of: root); try FileSafety.validateTree(path)
        let receipt = try FileSafety.child("cache-manifest.json", of: root)
        let files = try JSONDecoder().decode([String:String].self, from: Data(contentsOf: receipt))
        let walk = fm.enumerator(at: path, includingPropertiesForKeys: [.isRegularFileKey])
        while let item = walk?.nextObject() as? URL {
            guard try item.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile == true else { continue }
            let relative = String(item.path.dropFirst(path.path.count + 1))
            guard let expected = files[relative], try FileSafety.hash(item) == expected else {
                throw LauncherError("CACHE_UNTRACKED", "Cache contains an untracked or changed file.", recovery: "Inspect the Cache folder. Only manifest-recorded downloads can be removed by the launcher.")
            }
        }
        for relative in files.keys {
            let item = try FileSafety.child(relative, of: path)
            if fm.fileExists(atPath: item.path) { try fm.removeItem(at: item) }
        }
        try FileSafety.write([String:String](), to: receipt)
    }
    public func repairManaged(_ id: UUID, progress: @Sendable (OperationProgress) -> Void = { _ in }) throws {
        try checkIdle(); busy = true; defer { busy = false }
        guard let index = database.installations.firstIndex(where: { $0.id == id }), database.installations[index].managed, database.installations[index].isOfficialManaged else { throw LauncherError("MANAGED_REQUIRED", "Only a downloaded managed installation can use this repair.") }
        var install = database.installations[index]
        if install.game == .bedrock { try repairBedrock(&install, progress: progress); var next = database; next.installations[index] = install; try persist(next); return }
        let parent = try FileSafety.child("\(install.version)-\(id.uuidString)", of: versionsRoot(install.game))
        guard install.path.path == parent.appendingPathComponent("Game").path else { throw LauncherError("MANIFEST_MISMATCH", "Repair path does not match its receipt.") }
        let receipt = try JSONDecoder().decode(Installation.self, from: Data(contentsOf: FileSafety.child("metadata.json", of: parent)))
        guard receipt.id == id, receipt.gameId == install.gameId, receipt.managed, receipt.bottle == install.bottle else { throw LauncherError("MANIFEST_MISMATCH", "Repair ownership does not match.") }
        guard let crossOver = self.crossOver ?? .detect() else { throw LauncherError("CROSSOVER_MISSING", "CrossOver is unavailable.") }
        let bottle = try crossOver.bottleURL(install.bottle)
        let owner = try JSONDecoder().decode([String:String].self, from: Data(contentsOf: FileSafety.child("minecraftmac-owner.json", of: bottle)))
        guard owner["schema"] == "1", let ownerID = owner["owner"], UUID(uuidString: ownerID) != nil, install.bottle == "MinecraftMac-\(ownerID)" else { throw LauncherError("MANIFEST_MISMATCH", "The CrossOver environment is not owned by this launcher.") }
        let profile = try CompatibilityProfile.managed()
        guard install.version == profile.version, let assets = Bundle.module.url(forResource: "Managed", withExtension: nil, subdirectory: "Resources") else { throw LauncherError("COMPATIBILITY_UNVERIFIED", "No bundled repair profile is available.") }
        // Before touching compatibility files, verify all recorded game data and resources.
        let allowed = Set(profile.hashes.keys.filter { $0.hasSuffix(".dll") }).union(["xgameruntime.dll"])
        for (relative, expected) in install.hashes where !allowed.contains(relative) {
            guard try FileSafety.hash(FileSafety.child(relative, of: install.path)) == expected else { throw LauncherError("GAME_DATA_CHANGED", "Game data changed; install a fresh managed copy to repair it.") }
        }
        install.state = .testing; var pending = database; pending.installations[index] = install; try persist(pending)
        do {
            for (relative, expected) in profile.hashes where relative.hasSuffix(".dll") {
                let name = URL(fileURLWithPath: relative).lastPathComponent
                let source = ["XCurl.dll", "xgameruntime.dll"].contains(name) ? try FileSafety.child(name, of: assets) : try FileSafety.child("Components/" + name, of: parent)
                // libHttpClient is an original game component; keep its verified package copy.
                let actualSource = name == "libHttpClient.GDK.dll" ? try FileSafety.child(relative, of: install.path) : source
                guard try FileSafety.hash(actualSource) == expected else { throw LauncherError("COMPONENT_INTEGRITY_FAILED", "A repair component is missing or differs from the tested digest.") }
                let target = try FileSafety.child(relative, of: install.path)
                if (try? FileSafety.hash(target)) != expected { try ManagedInstaller.replace(actualSource, destination: target) }
                install.hashes[relative] = expected
            }
            let runtime = try FileSafety.child("Dungeons/Binaries/WinGDK/xgameruntime.dll", of: install.path)
            try ManagedInstaller.replace(runtime, destination: try FileSafety.child("xgameruntime.dll", of: install.path))
            try ManagedInstaller.replace(runtime, destination: try FileSafety.child("drive_c/windows/system32/xgameruntime.dll", of: bottle))
            install.hashes["xgameruntime.dll"] = profile.hashes["Dungeons/Binaries/WinGDK/xgameruntime.dll"]
            install.compatibilityProfile = profile.id; install = try verify(install, progress: progress)
            try FileSafety.write(install, to: parent.appendingPathComponent("metadata.json"))
            var next = database; next.installations[index] = install; try persist(next)
        } catch {
            install.state = .failed; var next = database; next.installations[index] = install; try persist(next); throw error
        }
    }
    public func repairCompatibility(_ id: UUID, referenceID: UUID, progress: @Sendable (OperationProgress) -> Void = { _ in }) throws {
        try checkIdle(); busy = true; defer { busy = false }
        guard let index = database.installations.firstIndex(where: { $0.id == id }), database.installations[index].managed,
              let reference = database.installations.first(where: { $0.id == referenceID }), reference.id != id, reference.gameId == database.installations[index].gameId, reference.gameId == "dungeons2" else {
            throw LauncherError("REPAIR_SOURCE_REQUIRED", "Repair requires a managed snapshot and a separate verified local installation.")
        }
        _ = try verify(reference, progress: progress)
        var install = database.installations[index]
        guard install.version == profile.version else { throw LauncherError("COMPATIBILITY_UNVERIFIED", "No repair profile for this build.") }
        let expectedParent = try FileSafety.child("\(install.version)-\(id.uuidString)", of: versionsRoot(install.game))
        guard install.path.path == expectedParent.appendingPathComponent("Game").path else { throw LauncherError("MANIFEST_MISMATCH", "Repair path does not match its manifest.") }
        install.state = .testing
        var pending = database; pending.installations[index] = install; try persist(pending)
        try FileSafety.write(install, to: expectedParent.appendingPathComponent("metadata.json"))
        do {
        // Per-file atomic replacement: interruption is recoverable; no saves or bottle files touched.
        for (relative, expected) in profile.hashes where relative.hasSuffix(".dll") {
            let destination = try FileSafety.child(relative, of: install.path)
            if (try? FileSafety.hash(destination)) == expected { continue }
            progress(.init("Repairing compatibility"))
            let source = try FileSafety.child(relative, of: reference.path)
            let temp = destination.deletingLastPathComponent().appendingPathComponent(".repair-\(UUID().uuidString)")
            try fm.copyItem(at: source, to: temp)
            defer { try? fm.removeItem(at: temp) }
            guard try FileSafety.hash(temp) == expected else { throw LauncherError("REPAIR_SOURCE_CHANGED", "Repair source changed during copying.") }
            guard Darwin.rename(temp.path, destination.path) == 0 else { throw LauncherError("REPAIR_FAILED", "Could not atomically replace compatibility file.") }
        }
        install = try verify(install, progress: progress)
        try FileSafety.write(install, to: expectedParent.appendingPathComponent("metadata.json"))
        var next = database; next.installations[index] = install; try persist(next)
        } catch {
            install.state = .failed
            var next = database; next.installations[index] = install
            try? FileSafety.write(install, to: expectedParent.appendingPathComponent("metadata.json"))
            try persist(next); throw error
        }
    }
    private func repairBedrock(_ install: inout Installation, progress: @Sendable (OperationProgress) -> Void) throws {
        let parent = try FileSafety.child("\(install.version)-\(install.id.uuidString)", of: versionsRoot(.bedrock))
        guard install.path.path == parent.appendingPathComponent("Game").path else { throw LauncherError("MANIFEST_MISMATCH", "Bedrock repair path does not match.") }
        let receipt = try JSONDecoder().decode(Installation.self, from: Data(contentsOf: FileSafety.child("metadata.json", of: parent)))
        guard receipt.id == install.id, receipt.gameId == install.gameId, receipt.bottle == install.bottle else { throw LauncherError("MANIFEST_MISMATCH", "Repair receipt does not match.") }
        guard let crossOver = self.crossOver ?? .detect() else { throw LauncherError("CROSSOVER_MISSING", "CrossOver is unavailable.") }
        try crossOver.requireBedrockVersion()
        let bottle = try crossOver.bottleURL(install.bottle)
        try ManagedInstaller.validateOwner(bottle: bottle, name: install.bottle, game: .bedrock)
        let profile = try BedrockRuntime.profile()
        for (name, hash) in install.hashes where profile.hashes[name] == nil {
            guard try FileSafety.hash(FileSafety.child(name, of: install.path)) == hash else { throw LauncherError("GAME_DATA_CHANGED", "Bedrock game files changed.", recovery: "Install a fresh authenticated managed copy. The current executable and worlds are preserved.") }
        }
        let runtime = try BedrockRuntime.prepare(root: root)
        for name in profile.hashes.keys {
            try ManagedInstaller.replace(FileSafety.child("x86_64-windows/" + name, of: runtime), destination: FileSafety.child(name, of: install.path))
            install.hashes[name] = profile.hashes[name]
        }
        try ManagedInstaller.replace(FileSafety.child("gameinput.dll", of: install.path), destination: FileSafety.child("drive_c/windows/system32/gameinput.dll", of: bottle))
        install = try verify(install, progress: progress)
        try FileSafety.write(install, to: FileSafety.child("metadata.json", of: parent))
    }

}
