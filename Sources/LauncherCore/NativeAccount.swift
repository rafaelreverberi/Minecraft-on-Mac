import Foundation

public struct AccountStatus: Codable, Sendable {
    public let schema: Int
    public let status: String
    public let signedIn: Bool
    public let gamertag: String?
    public let entitlement: String
    public let availableVersion: String?
    public let availableRevision: String?
    public let errorCode: String?
    public static func decode(_ data: Data) throws -> Self {
        guard data.count <= 16384 else { throw LauncherError("HELPER_PROTOCOL_FAILED", "Account helper exceeded its response limit.") }
        let allowedFields: Set<String> = ["schema", "status", "signedIn", "gamertag", "entitlement", "availableVersion", "availableRevision", "errorCode"]
        guard let object = try? JSONSerialization.jsonObject(with: data) as? [String:Any], Set(object.keys).isSubset(of: allowedFields) else {
            throw LauncherError("HELPER_PROTOCOL_FAILED", "Account helper returned unexpected fields.")
        }
        let result: Self
        do { result = try JSONDecoder().decode(Self.self, from: data) } catch {
            throw LauncherError("HELPER_PROTOCOL_FAILED", "Account helper returned an invalid response.")
        }
        let allowedCodes = ["COMMAND_INVALID", "KEYCHAIN_UNAVAILABLE", "SIGN_OUT_FAILED", "NETWORK_UNAVAILABLE", "MICROSOFT_SERVICE_FAILED", "LOGIN_CANCELLED", "SIGN_IN_REQUIRED", "DEVICE_AUTH_REQUIRED", "STORE_LOOKUP_FAILED", "ENTITLEMENT_NOT_CONFIRMED", "PACKAGE_LOOKUP_FAILED", "PACKAGE_VERSION_INVALID"]
        guard result.schema == 1, ["ok", "error"].contains(result.status), ["unknown", "verified"].contains(result.entitlement),
              result.availableRevision.map({ UUID(uuidString: $0) != nil }) ?? true,
              result.errorCode.map({ allowedCodes.contains($0) }) ?? true,
              result.gamertag.map({ $0.utf8.count <= 256 && !$0.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }) }) ?? true,
              result.availableVersion.map({ (try? PackageVersion($0)) != nil }) ?? true,
              result.entitlement != "verified" || (result.status == "ok" && result.signedIn && result.availableVersion != nil) else {
            throw LauncherError("HELPER_PROTOCOL_FAILED", "Account helper returned inconsistent data.")
        }
        return result
    }
}
public actor NativeAccount {
    public enum Command: String, Sendable { case status, login, logout, check }
    private var busy = false
    public init() {}
    public nonisolated static func helperURL() throws -> URL {
        guard let executable = Bundle.main.executableURL else { throw LauncherError("HELPER_MISSING", "Use the packaged macOS app for account operations.") }
        let helper = executable.deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Helpers/minecraft-native-helper")
        guard let digestURL = Bundle.module.url(forResource: "helper-sha256", withExtension: "txt", subdirectory: "Resources"),
              let expected = try? String(contentsOf: digestURL, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines),
              expected.count == 64, (try? FileSafety.hash(helper)) == expected else {
            throw LauncherError("HELPER_INTEGRITY_FAILED", "The native Microsoft helper is missing or differs from the bundled digest.", recovery: "Rebuild the app or restore a verified release. No credentials were accessed.")
        }
        // One immutable, content-addressed identity for account client, service and broker.
        let root = LibraryStore.defaultRoot
        let stable = try FileSafety.child("Tools/accounts/" + expected + "/minecraft-native-helper", of: root)
        if !FileManager.default.fileExists(atPath: stable.path) {
            try FileManager.default.createDirectory(at: stable.deletingLastPathComponent(), withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            try FileManager.default.copyItem(at: helper, to: stable)
            try FileManager.default.setAttributes([.posixPermissions: 0o555], ofItemAtPath: stable.path)
        }
        guard try FileSafety.hash(stable) == expected else { throw LauncherError("HELPER_INTEGRITY_FAILED", "Stable account helper changed.") }
        return stable
    }
    public func request(_ command: Command, game: GameDefinition = .dungeons2) async throws -> AccountStatus {
        guard !busy else { throw LauncherError("ACCOUNT_BUSY", "An account operation is already running.") }
        busy = true; defer { busy = false }
        let helper = try Self.helperURL()
        let data = try await Task.detached {
            let process = Process(); let pipe = Pipe()
            process.executableURL = helper; process.arguments = [command.rawValue, game.rawValue]
            process.environment = CrossOver.environment().filter { !["WINEDLLOVERRIDES", "WINEDEBUG", "XCURL_COMPAT_IDENTITY"].contains($0.key) }
            process.standardOutput = pipe; process.standardError = FileHandle.nullDevice
            try process.run()
            let deadline: Double = command == .login ? 600 : 150
            let timeout = DispatchWorkItem { if process.isRunning { process.terminate() } }
            DispatchQueue.global().asyncAfter(deadline: .now() + deadline, execute: timeout)
            defer { timeout.cancel() }
            var response = Data(); var exceeded = false
            while let bytes = try pipe.fileHandleForReading.read(upToCount: 4096), !bytes.isEmpty {
                if response.count + bytes.count <= 16384 { response.append(bytes) } else { exceeded = true; if process.isRunning { process.terminate() } }
            }
            process.waitUntilExit()
            guard process.terminationStatus == 0, !exceeded else {
                throw LauncherError("ACCOUNT_OPERATION_FAILED", "Microsoft account operation failed or timed out.", recovery: "Retry sign-in or the ownership check. Raw authentication output has been discarded.")
            }
            return response
        }.value
        let status = try AccountStatus.decode(data)
        if status.status == "error" {
            let code = status.errorCode ?? "MICROSOFT_SERVICE_FAILED"
            let message: String
            switch code {
            case "ENTITLEMENT_NOT_CONFIRMED": message = "Microsoft did not confirm a content license for \(game.name). No game data was downloaded."
            case "SIGN_IN_REQUIRED", "LOGIN_CANCELLED": message = "Microsoft sign-in is required or was cancelled."
            case "KEYCHAIN_UNAVAILABLE": message = "The macOS Keychain is unavailable."
            default: message = "The Microsoft service could not complete this operation."
            }
            throw LauncherError(code, message, recovery: "Sign in with the account that owns the game and retry. Ownership remains unverified on any failure.")
        }
        return status
    }
}
