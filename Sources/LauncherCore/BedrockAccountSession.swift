import Foundation

/// A private, per-launch socket. No connection to the reference /tmp/xodus.sock.
public final class BedrockAccountSession: @unchecked Sendable {
    private let process: Process
    private let input: Pipe
    private let root: URL
    public var environment: [String:String] { ["MML_BEDROCK_RUNTIME_DIR":root.path] }
    private init(process: Process, root: URL, input: Pipe) { self.process = process; self.root = root; self.input = input }
    public static func start() throws -> BedrockAccountSession {
        let root = URL(fileURLWithPath: "/private/tmp/mml-bedrock-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false, attributes: [.posixPermissions: 0o700])
        let p = Process(); let output = Pipe(); let input = Pipe(); p.standardInput = input
        p.executableURL = try NativeAccount.helperURL(); p.arguments = ["bedrock-service", root.path]
        p.environment = CrossOver.environment(for: .bedrock); p.standardOutput = output; p.standardError = FileHandle.nullDevice
        do {
            try p.run()
            let timer = DispatchWorkItem { if p.isRunning { p.terminate() } }
            DispatchQueue.global().asyncAfter(deadline: .now()+90, execute: timer); defer { timer.cancel() }
            var data = Data()
            while let byte = try output.fileHandleForReading.read(upToCount: 1), !byte.isEmpty, byte.first != 10 {
                data.append(byte); guard data.count <= 64 else { throw LauncherError("BEDROCK_LAUNCH_FAILED", "Account service returned invalid data.") }
            }
            guard String(decoding: data, as: UTF8.self) == "{\"schema\":1,\"ready\":true}", p.isRunning else { throw LauncherError("BEDROCK_LAUNCH_FAILED", "Bedrock account service could not start.") }
            return BedrockAccountSession(process: p, root: root, input: input)
        } catch { if p.isRunning { p.terminate(); p.waitUntilExit() }; try? FileManager.default.removeItem(at: root); throw error }
    }
    public func stop() {
        try? input.fileHandleForWriting.close()
        if process.isRunning { process.terminate(); process.waitUntilExit() }
        try? FileManager.default.removeItem(at: root)
    }
    deinit { stop() }
}
