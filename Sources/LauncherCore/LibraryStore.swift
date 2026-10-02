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
                guard database.schema == 1 else { throw LauncherError("DATABASE_VERSION", "Library schema is unsupported.") }
            } else { self.database = LibraryDatabase() }
            for folder in ["Games/dungeons2/Versions", "Compatibility", "Cache", "Logs", "Tools"] {
                let path = try FileSafety.child(folder, of: root)
                try FileManager.default.createDirectory(at: path, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            }
            let cacheReceipt = try FileSafety.child("cache-manifest.json", of: root)
            if !FileManager.default.fileExists(atPath: cacheReceipt.path) { try FileSafety.write([String:String](), to: cacheReceipt) }
            self.lockFD = fd
        } catch {
            flock(fd, LOCK_UN); close(fd)
            throw LauncherError("DATABASE_UNAVAILABLE", "The library could not be read safely.", recovery: "Preserve library.json and review file permissions. It has not been reset.")
        }
    }
    deinit { if lockFD >= 0 { flock(lockFD, LOCK_UN); close(lockFD) } }
    public func snapshot() -> LibraryDatabase { database }
    public func staleStaging() -> [String] {
        ((try? fm.contentsOfDirectory(atPath: versionsRoot.path)) ?? []).filter { $0.hasPrefix(".staging-") }.sorted()
    }
    private var versionsRoot: URL { root.appendingPathComponent("Games/dungeons2/Versions") }
    private func checkIdle() throws {
        guard !busy else { throw LauncherError("OPERATION_BUSY", "A library operation is in progress.") }
        guard !isGameRunning() else { throw LauncherError("GAME_RUNNING", "Quit Dungeons before changing the library.") }
    }
    private func persist(_ next: LibraryDatabase) throws {
        _ = try FileSafety.child("library.json", of: root)
        try FileSafety.write(next, to: databaseURL); database = next
    }
    public func verify(_ install: Installation, progress: @Sendable (OperationProgress) -> Void = { _ in }) throws -> Installation {
        guard try FileSafety.version(install.path) == profile.version, install.version == profile.version else {
            throw LauncherError("COMPATIBILITY_UNVERIFIED", "This build has no verified compatibility profile.", recovery: "Keep using the current verified build. Do not apply its DLLs to this build.")
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
            if path.pathExtension.lowercased() == "dll" || path.pathExtension.lowercased() == "exe" {
                guard try FileSafety.isX64PE(path) else { throw LauncherError("ARCHITECTURE_INVALID", "A required executable is not x86_64 PE.") }
            }
        }
        // Always bind trusted profile hashes as well, even if database hashes were modified.
        for (relative, expected) in profile.hashes {
            guard try FileSafety.hash(FileSafety.child(relative, of: install.path)) == expected else {
                throw LauncherError("COMPATIBILITY_FAILED", "Compatibility files no longer match the bundled profile.")
            }
        }
        guard let crossOver else { throw LauncherError("CROSSOVER_MISSING", "Install CrossOver to use Dungeons.") }
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
        var next = database; next.installations.append(install); if next.current == nil { next.current = install.id }
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
        var next = database; next.current = id; try persist(next)
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
        let stage = try FileSafety.child(stageName, of: versionsRoot)
        try fm.createDirectory(at: stage, withIntermediateDirectories: false)
        let destination = stage.appendingPathComponent("Game")
        var install = Installation(id: newID, version: source.version, path: destination, bottle: source.bottle, managed: true)
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
            install.hashes = all; install.diskBytes = try FileSafety.diskUsage(destination)
            let final = try FileSafety.child("\(source.version)-\(newID.uuidString)", of: versionsRoot)
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
    public func remove(_ id: UUID) throws {
        try checkIdle()
        guard let install = database.installations.first(where: { $0.id == id }) else { throw LauncherError("VERSION_MISSING", "Installation is unavailable.") }
        guard database.current != id else { throw LauncherError("CURRENT_VERSION", "Select another version or clear the current selection first.") }
        if install.managed {
            let expected = try FileSafety.child("\(install.version)-\(install.id.uuidString)", of: versionsRoot)
            guard install.path.path == expected.appendingPathComponent("Game").path else {
                throw LauncherError("MANIFEST_MISMATCH", "Managed path does not match its recorded version.")
            }
            let receipt = try JSONDecoder().decode(Installation.self, from: Data(contentsOf: FileSafety.child("metadata.json", of: expected)))
            guard receipt.id == id, receipt.managed else { throw LauncherError("MANIFEST_MISMATCH", "Installation receipt does not match.") }
            try FileSafety.validateTree(expected)
            // Commit removal before physical cleanup; an interruption leaves an orphan, never a usable partial version.
            var next = database; next.installations.removeAll { $0.id == id }; try persist(next)
            try fm.removeItem(at: expected)
        } else {
            var next = database; next.installations.removeAll { $0.id == id }; try persist(next)
        }
    }
    public func clearSelection() throws { try checkIdle(); var next = database; next.current = nil; try persist(next) }
    public func removeStaging(_ name: String) throws {
        try checkIdle()
        guard name.hasPrefix(".staging-"), UUID(uuidString: String(name.dropFirst(9))) != nil else { throw LauncherError("UNSAFE_PATH", "Invalid staging directory.") }
        let path = try FileSafety.child(name, of: versionsRoot)
        let receipt = try JSONDecoder().decode(Installation.self, from: Data(contentsOf: FileSafety.child("metadata.json", of: path)))
        guard receipt.managed, name == ".staging-\(receipt.id.uuidString)" else { throw LauncherError("MANIFEST_MISMATCH", "Temporary installation receipt does not match.") }
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
    public func repairCompatibility(_ id: UUID, referenceID: UUID, progress: @Sendable (OperationProgress) -> Void = { _ in }) throws {
        try checkIdle(); busy = true; defer { busy = false }
        guard let index = database.installations.firstIndex(where: { $0.id == id }), database.installations[index].managed,
              let reference = database.installations.first(where: { $0.id == referenceID }), reference.id != id else {
            throw LauncherError("REPAIR_SOURCE_REQUIRED", "Repair requires a managed snapshot and a separate verified local installation.")
        }
        _ = try verify(reference, progress: progress)
        var install = database.installations[index]
        guard install.version == profile.version else { throw LauncherError("COMPATIBILITY_UNVERIFIED", "No repair profile for this build.") }
        let expectedParent = try FileSafety.child("\(install.version)-\(id.uuidString)", of: versionsRoot)
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
}
