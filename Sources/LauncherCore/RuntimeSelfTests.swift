import Foundation

public enum RuntimeSelfTests {
    /// No account calls. Copies only local runtime files into an isolated temporary directory.
    public static func run(_ install: Installation, crossOver: CrossOver,
                           progress: @Sendable (OperationProgress) -> Void = { _ in }) throws {
        guard let manifest = Bundle.module.url(forResource: "probe-sha256", withExtension: "json", subdirectory: "Resources") else {
            throw LauncherError("SELF_TESTS_MISSING", "The release ABI probes are missing.", recovery: "Rebuild the app with scripts/build_probes.py.")
        }
        let hashes = try JSONDecoder().decode([String:String].self, from: Data(contentsOf: manifest))
        let fm = FileManager.default
        let temp = fm.temporaryDirectory.resolvingSymlinksInPath().appendingPathComponent("MinecraftMac-probe-\(UUID().uuidString)")
        try fm.createDirectory(at: temp, withIntermediateDirectories: false, attributes: [.posixPermissions: 0o700])
        defer { try? fm.removeItem(at: temp) }
        for name in ["XCurl.dll", "XCurl2504.dll", "xgameruntime.dll", "xgameruntime.gdk.dll", "libHttpClient.GDK.dll"] {
            let source = try FileSafety.child("Dungeons/Binaries/WinGDK/\(name)", of: install.path)
            try fm.copyItem(at: source, to: temp.appendingPathComponent(name))
        }
        let probes = ["token-abi.exe", "xcurl_exports.exe", "runtime_exports.exe"]
        guard Set(hashes.keys) == Set(probes) else { throw LauncherError("SELF_TESTS_INVALID", "Unexpected ABI probe manifest.") }
        for (index, name) in probes.enumerated() {
            try Task.checkCancellation()
            guard let source = Bundle.module.url(forResource: name, withExtension: nil, subdirectory: "Resources/Probes"),
                  try FileSafety.hash(source) == hashes[name] else {
                throw LauncherError("SELF_TESTS_INVALID", "ABI probe integrity failed.")
            }
            let target = temp.appendingPathComponent(name); try fm.copyItem(at: source, to: target)
            progress(.init("Testing \(name)", completed: Int64(index), total: 3))
            let process = Process(); process.executableURL = crossOver.wine
            process.arguments = ["--debugmsg", "-all", "--bottle", install.bottle, "--dll", "xgameruntime=n;XCurl=n", "--workdir", temp.path, target.path]
            process.currentDirectoryURL = temp; process.environment = CrossOver.environment()
            process.standardOutput = FileHandle.nullDevice; process.standardError = FileHandle.nullDevice
            try process.run()
            let timeout = DispatchWorkItem { if process.isRunning { process.terminate() } }
            DispatchQueue.global().asyncAfter(deadline: .now() + 45, execute: timeout)
            process.waitUntilExit(); timeout.cancel()
            guard process.terminationStatus == 0, process.terminationReason == .exit else {
                throw LauncherError("ABI_TEST_FAILED", "\(name) failed with exit status \(process.terminationStatus).", recovery: "Current version selection is unchanged. Repair runtime components and rerun verification.")
            }
        }
        progress(.init("Runtime tests passed", completed: 3, total: 3))
    }
}
