import Foundation
import Security

public enum ManagedInstaller {
    struct Event: Decodable {
        let schema: Int
        let type: String
        let phase: String?
        let completed: Int64?
        let total: Int64?
        let version: String?
        let code: String?
    }
    public static func acquire(stage: URL, game: GameDefinition = .dungeons2, progress: @escaping @Sendable (OperationProgress) -> Void) async throws {
        let helper = try NativeAccount.helperURL()
        try await Task.detached {
            let p = Process(); let pipe = Pipe()
            p.executableURL = helper; p.arguments = ["install", stage.path, game.rawValue]
            p.environment = CrossOver.environment().filter { !["WINEDLLOVERRIDES", "WINEDEBUG", "XCURL_COMPAT_IDENTITY"].contains($0.key) }
            p.standardOutput = pipe; p.standardError = FileHandle.nullDevice
            try p.run()
            let timeout = DispatchWorkItem { if p.isRunning { p.terminate() } }
            DispatchQueue.global().asyncAfter(deadline: .now() + 14400, execute: timeout)
            defer { timeout.cancel() }
            var pending = Data(); var finished = false; var invalid = false; var lastPhase = ""; var failureCode = "INSTALL_FAILED"
            let phases = ["authorizing":"Verifying Microsoft license", "downloading-game":"Downloading game", "verifying-package":"Verifying encrypted package", "decrypting":"Preparing game files", "downloading-components":"Downloading Microsoft components"]
            while let chunk = try pipe.fileHandleForReading.read(upToCount: 4096), !chunk.isEmpty {
                pending.append(chunk)
                while let end = pending.firstIndex(of: 10) {
                    let line = pending.prefix(upTo: end); pending.removeSubrange(...end)
                    guard line.count <= 4096, let event = try? JSONDecoder().decode(Event.self, from: line), event.schema == 1 else { invalid = true; break }
                    if event.type == "progress", let phase = event.phase, let label = phases[phase], let done = event.completed, let total = event.total, done >= 0, total >= 0, total == 0 || done <= total {
                        if lastPhase != phase { try FileSafety.write(["phase":phase], to: stage.appendingPathComponent("operation-status.json")); lastPhase = phase }
                        progress(.init(label, completed: done, total: total))
                    } else if event.type == "complete", event.version == game.baseline, !finished { finished = true }
                    else if event.type == "error", let code = event.code, ["INSTALL_FAILED", "BEDROCK_VERSION_TOO_OLD", "BEDROCK_COMPATIBILITY_UNVERIFIED", "BEDROCK_PACKAGE_INTEGRITY_UNVERIFIED"].contains(code) { invalid = true; failureCode = code }
                    else { invalid = true }
                }
                if invalid || pending.count > 4096 { invalid = true; if p.isRunning { p.terminate() }; break }
            }
            p.waitUntilExit()
            guard p.terminationStatus == 0, finished, !invalid, pending.isEmpty else {
                throw LauncherError(failureCode, failureCode == "BEDROCK_PACKAGE_INTEGRITY_UNVERIFIED" ? "Microsoft did not supply a trusted SHA-256 for this Bedrock package revision. A reviewed package pin is required." : "The licensed download or extraction did not complete.", recovery: "Check the connection, Microsoft account and available storage, then retry. Your current version is preserved; interrupted files are listed in Storage.")
            }
        }.value
    }
    public static func run(_ executable: URL, arguments: [String], timeout: Double = 180, accepted: Set<Int32> = [0]) async throws {
        try await Task.detached {
            let p = Process(); p.executableURL = executable; p.arguments = arguments
            p.environment = CrossOver.environment(); p.standardOutput = FileHandle.nullDevice; p.standardError = FileHandle.nullDevice
            try p.run()
            let timer = DispatchWorkItem { if p.isRunning { p.terminate() } }
            DispatchQueue.global().asyncAfter(deadline: .now() + timeout, execute: timer)
            defer { timer.cancel() }; p.waitUntilExit()
            guard accepted.contains(p.terminationStatus) else { throw LauncherError("ENVIRONMENT_SETUP_FAILED", "CrossOver could not prepare the game environment.") }
        }.value
    }
    public static func provisionBottle(_ name: String, crossOver: CrossOver, components: URL, runtime: URL, owner: UUID) async throws {
        guard name == "MinecraftMac-\(owner.uuidString)" else { throw LauncherError("BOTTLE_INVALID", "Managed environment name is invalid.") }
        let path = try FileSafety.child(name, of: crossOver.bottlesRoot)
        if FileManager.default.fileExists(atPath: path.path) {
            let receipt = try JSONDecoder().decode([String:String].self, from: Data(contentsOf: FileSafety.child("minecraftmac-owner.json", of: path)))
            guard receipt["schema"] == "1", receipt["owner"] == owner.uuidString else { throw LauncherError("BOTTLE_EXISTS", "An environment without a matching launcher receipt will not be overwritten.") }
        } else {
            let cxbottle = crossOver.wine.deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("bin/cxbottle")
            try await run(cxbottle, arguments: ["--bottle", name, "--create", "--template", "win11_64", "--description", "Minecraft on Mac managed environment"])
        }
        let bottle = try crossOver.bottleURL(name)
        try FileSafety.write(["schema":"1", "owner":owner.uuidString], to: bottle.appendingPathComponent("minecraftmac-owner.json"))
        // CrossOver templates contain intentional symlinks elsewhere. Only write these regular, owned paths.
        let system = try FileSafety.child("drive_c/windows/system32", of: bottle)
        try replace(runtime, destination: try FileSafety.child("xgameruntime.dll", of: system))
        let redist = try FileSafety.child("VC_redist.x64.exe", of: components)
        guard try FileSafety.hash(redist) == "843068991daaa1f73ad9f6239bce4d0f6a07a51f18c37ea2a867e9beca71295c" else { throw LauncherError("COMPONENT_INTEGRITY_FAILED", "Microsoft Visual C++ installer checksum differs.") }
        try await run(crossOver.wine, arguments: ["--debugmsg", "-all", "--bottle", name, redist.path, "/install", "/quiet", "/norestart"], timeout: 600, accepted: [0, 194])
        for name in ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"] {
            guard try FileSafety.isX64PE(FileSafety.child(name, of: system)) else { throw LauncherError("ENVIRONMENT_SETUP_FAILED", "The Microsoft Visual C++ runtime was not installed correctly.") }
        }
    }
    public static func validateOwner(bottle: URL, name: String, game: GameDefinition) throws {
        let receipt = try JSONDecoder().decode([String:String].self, from: Data(contentsOf: FileSafety.child("minecraftmac-owner.json", of: bottle)))
        guard let id = receipt["owner"], UUID(uuidString: id) != nil else { throw LauncherError("MANIFEST_MISMATCH", "Invalid environment receipt.") }
        let legacy = game == .dungeons2 && receipt["schema"] == "1" && receipt["gameId"] == nil && name == "MinecraftMac-\(id)"
        let scoped = receipt["schema"] == "2" && receipt["gameId"] == game.rawValue && name == "MinecraftMac-\(game.rawValue)-\(id)"
        guard legacy || scoped else { throw LauncherError("ENVIRONMENT_CROSS_GAME", "Environment ownership does not match this game.") }
    }
    static func replace(_ source: URL, destination: URL) throws {
        let temporary = destination.deletingLastPathComponent().appendingPathComponent(".provision-\(UUID().uuidString)")
        try FileManager.default.copyItem(at: source, to: temporary)
        defer { try? FileManager.default.removeItem(at: temporary) }
        guard rename(temporary.path, destination.path) == 0 else { throw LauncherError("ENVIRONMENT_WRITE_FAILED", "The managed environment could not be updated.") }
    }
}
/// The per-launch capability is random, is only passed via stdin/environment, and dies with the helper.
public final class CredentialBroker: @unchecked Sendable {
    private let process: Process
    private let input: Pipe
    public let environment: [String:String]
    private init(process: Process, input: Pipe, environment: [String:String]) { self.process = process; self.input = input; self.environment = environment }
    public static func start() throws -> CredentialBroker {
        var bytes = [UInt8](repeating: 0, count: 32)
        guard SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) == errSecSuccess else { throw LauncherError("BRIDGE_FAILED", "Secure account bridge could not start.") }
        let key = bytes.map { String(format: "%02x", $0) }.joined()
        let p = Process(); let input = Pipe(); let output = Pipe()
        p.executableURL = try NativeAccount.helperURL(); p.arguments = ["bridge"]
        p.environment = CrossOver.environment(); p.standardInput = input; p.standardOutput = output; p.standardError = FileHandle.nullDevice
        try p.run()
        do {
            try input.fileHandleForWriting.write(contentsOf: Data((key + "\n").utf8))
            let timeout = DispatchWorkItem { if p.isRunning { p.terminate() } }
            DispatchQueue.global().asyncAfter(deadline: .now() + 15, execute: timeout)
            defer { timeout.cancel() }
            var line = Data()
            while let byte = try output.fileHandleForReading.read(upToCount: 1), !byte.isEmpty, byte.first != 10 {
                line.append(byte); guard line.count <= 128 else { throw LauncherError("BRIDGE_FAILED", "Account bridge returned invalid data.") }
            }
            struct Ready: Decodable { let schema: Int; let port: UInt16 }
            let ready = try JSONDecoder().decode(Ready.self, from: line)
            guard ready.schema == 1, ready.port > 0, p.isRunning else { throw LauncherError("BRIDGE_FAILED", "Account bridge is unavailable.") }
            return CredentialBroker(process: p, input: input, environment: ["MML_BRIDGE_KEY":key, "MML_BRIDGE_PORT":String(ready.port)])
        } catch { if p.isRunning { p.terminate() }; throw LauncherError("BRIDGE_FAILED", "The macOS Keychain bridge could not start.") }
    }
    public func stop() { try? input.fileHandleForWriting.close(); if process.isRunning { process.terminate() } }
    deinit { stop() }
}
