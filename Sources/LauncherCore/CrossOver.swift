import Foundation

public struct CrossOver: Sendable {
    public let wine: URL
    public let bottlesRoot: URL
    public init(wine: URL, bottlesRoot: URL) { self.wine = wine; self.bottlesRoot = bottlesRoot }
    public static func detect(home: URL = FileManager.default.homeDirectoryForCurrentUser) -> Self? {
        let suffix = "Contents/SharedSupport/CrossOver/CrossOver-Hosted Application/wine"
        for app in [home.appendingPathComponent("Applications/CrossOver.app"), URL(fileURLWithPath: "/Applications/CrossOver.app")] {
            let wine = app.appendingPathComponent(suffix)
            if FileManager.default.isExecutableFile(atPath: wine.path) {
                return Self(wine: wine, bottlesRoot: home.appendingPathComponent("Library/Application Support/CrossOver/Bottles"))
            }
        }
        return nil
    }
    public func bottleURL(_ name: String) throws -> URL {
        guard !name.isEmpty, !name.hasPrefix("."), !name.contains("/"), !name.contains("\\"), !name.contains("\n") else {
            throw LauncherError("BOTTLE_INVALID", "Choose a detected CrossOver bottle.")
        }
        let root = try FileSafety.child(name, of: bottlesRoot)
        guard FileManager.default.fileExists(atPath: root.appendingPathComponent("cxbottle.conf").path),
              FileManager.default.fileExists(atPath: root.appendingPathComponent("drive_c/windows/system32").path) else {
            throw LauncherError("BOTTLE_MISSING", "The selected CrossOver bottle is unavailable.", recovery: "Create or repair the bottle in CrossOver, then select it again.")
        }
        return root
    }
    public func bottles() -> [String] {
        ((try? FileManager.default.contentsOfDirectory(atPath: bottlesRoot.path)) ?? []).filter { (try? bottleURL($0)) != nil }.sorted()
    }
    public func launchArguments(_ install: Installation) -> [String] {
        let bin = install.path.appendingPathComponent("Dungeons/Binaries/WinGDK")
        return ["--debugmsg", "-all", "--bottle", install.bottle, "--dll", "xgameruntime=n;XCurl=n", "--workdir", bin.path,
                bin.appendingPathComponent("Dungeons-WinGDK-Shipping.exe").path]
    }
    public static func environment() -> [String: String] {
        // Allowlist avoids inherited token/debug injection. No credentials are passed on the command line.
        let original = ProcessInfo.processInfo.environment
        var env: [String: String] = [:]
        for key in ["HOME", "USER", "LOGNAME", "TMPDIR", "LANG", "PATH", "DISPLAY"] { env[key] = original[key] }
        env["XCURL_COMPAT_IDENTITY"] = "1"; env["WINEDLLOVERRIDES"] = "xgameruntime=n;XCurl=n"; env["WINEDEBUG"] = "-all"
        return env
    }
    public static func gameRunning() -> Bool {
        let p = Process(); let pipe = Pipe(); p.executableURL = URL(fileURLWithPath: "/bin/ps")
        p.arguments = ["-axo", "comm="]; p.standardOutput = pipe; p.standardError = FileHandle.nullDevice
        guard (try? p.run()) != nil else { return true } // fail closed
        let data = pipe.fileHandleForReading.readDataToEndOfFile(); p.waitUntilExit()
        guard p.terminationStatus == 0 else { return true }
        return String(decoding: data, as: UTF8.self).split(separator: "\n").contains {
            $0.lowercased().contains("dungeons-wingdk-shipping.exe")
        }
    }
}
public actor GameProcess {
    private var process: Process?
    public init() {}
    public func launch(_ install: Installation, crossOver: CrossOver) async throws -> Int32 {
        guard process == nil, !CrossOver.gameRunning() else {
            throw LauncherError("GAME_ALREADY_RUNNING", "Dungeons is already running.", recovery: "Quit the game before starting another instance.")
        }
        _ = try crossOver.bottleURL(install.bottle)
        let p = Process(); p.executableURL = crossOver.wine; p.arguments = crossOver.launchArguments(install)
        var broker: CredentialBroker?
        if install.compatibilityProfile.hasPrefix("dungeons2-managed-") { broker = try CredentialBroker.start() }
        defer { broker?.stop() }
        p.environment = CrossOver.environment().merging(broker?.environment ?? [:]) { _, new in new }; p.currentDirectoryURL = install.path.appendingPathComponent("Dungeons/Binaries/WinGDK")
        p.standardOutput = FileHandle.nullDevice; p.standardError = FileHandle.nullDevice
        process = p
        defer { process = nil }
        return try await withCheckedThrowingContinuation { continuation in
            p.terminationHandler = { process in continuation.resume(returning: process.terminationStatus) }
            do { try p.run() } catch { p.terminationHandler = nil; continuation.resume(throwing: LauncherError("LAUNCH_FAILED", "CrossOver could not start the game.")) }
        }
    }
}
