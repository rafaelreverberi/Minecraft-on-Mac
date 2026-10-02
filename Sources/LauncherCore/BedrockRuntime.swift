import Foundation
import Darwin

/// Bedrock's WineGDK loader, registrations and probes are isolated from Dungeons.
public enum BedrockRuntime {
    public static let dlls = ["wintypes", "twinapi.appcore", "windows.web", "windows.storage", "gameinput", "windows.ui.core.textinput", "windows.storage.applicationdata", "windows.applicationmodel", "xgameruntime", "XCurl"]
    public static func profile() throws -> CompatibilityProfile {
        guard let url = Bundle.module.url(forResource: "compatibility-bedrock", withExtension: "json", subdirectory: "Resources") else { throw LauncherError("BEDROCK_RUNTIME_INVALID", "The packaged Bedrock runtime is missing.") }
        let p = try JSONDecoder().decode(CompatibilityProfile.self, from: Data(contentsOf: url))
        guard p.gameId == "bedrock", p.version == GameDefinition.bedrock.baseline, p.hashes.count == dlls.count - 1 else { throw LauncherError("BEDROCK_RUNTIME_INVALID", "Bedrock profile is invalid.") }
        return p
    }
    private struct Provenance: Decodable { let hashes: [String:String] }
    public static func assets() throws -> URL {
        guard let url = Bundle.module.url(forResource: "Bedrock", withExtension: nil, subdirectory: "Resources") else { throw LauncherError("BEDROCK_RUNTIME_INVALID", "Use the complete packaged launcher.") }; return url
    }
    public static func runtimeRoot(_ root: URL) throws -> URL { try FileSafety.child("Compatibility/bedrock/\(try profile().id)", of: root) }
    public static func prepare(root: URL) throws -> URL {
        let source = try assets(), destination = try runtimeRoot(root)
        let manifest = try JSONDecoder().decode(Provenance.self, from: Data(contentsOf: FileSafety.child("provenance.json", of: source)))
        guard Set(manifest.hashes.keys) == Set(dlls.filter { $0 != "XCurl" }.map { $0 + ".dll" } + ["classes.json", "xgameruntime.so", "bedrock-probe.exe"]) else { throw LauncherError("BEDROCK_RUNTIME_INVALID", "Unexpected runtime inventory.") }
        for (name, hash) in manifest.hashes { guard try FileSafety.hash(FileSafety.child(name, of: source)) == hash else { throw LauncherError("BEDROCK_RUNTIME_INVALID", "A bundled runtime component changed.") } }
        if !FileManager.default.fileExists(atPath: destination.path) {
            let parent = destination.deletingLastPathComponent()
            try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
            let stage = try FileSafety.child(".runtime-\(UUID().uuidString)", of: parent)
            try FileManager.default.createDirectory(at: stage, withIntermediateDirectories: false)
            defer { try? FileManager.default.removeItem(at: stage) }
            for (name, _) in manifest.hashes {
                let relative = name.hasSuffix(".dll") ? "x86_64-windows/" + name : (name.hasSuffix(".so") ? "x86_64-unix/" + name : name)
                let target = try FileSafety.child(relative, of: stage)
                try FileManager.default.createDirectory(at: target.deletingLastPathComponent(), withIntermediateDirectories: true)
                try FileManager.default.copyItem(at: FileSafety.child(name, of: source), to: target)
            }
            try FileManager.default.moveItem(at: stage, to: destination)
        }
        try FileSafety.validateTree(destination)
        for (name, hash) in manifest.hashes {
            let relative = name.hasSuffix(".dll") ? "x86_64-windows/" + name : (name.hasSuffix(".so") ? "x86_64-unix/" + name : name)
            guard try FileSafety.hash(FileSafety.child(relative, of: destination)) == hash else { throw LauncherError("BEDROCK_RUNTIME_INVALID", "The managed runtime changed.") }
        }
        return destination
    }
    public static func validate(_ install: Installation, root: URL, crossOver: CrossOver, runProbes: Bool) throws {
        try GameDefinition.bedrock.validateVersion(install.version)
        try crossOver.requireBedrockVersion()
        guard install.isOfficialManaged, !install.hashes.isEmpty else { throw LauncherError("BEDROCK_PACKAGE_INVALID", "Bedrock requires an official managed package receipt.") }
        let runtime = try prepare(root: root)
        let bottle = try crossOver.bottleURL(install.bottle)
        try ManagedInstaller.validateOwner(bottle: bottle, name: install.bottle, game: .bedrock)
        guard try FileSafety.isX64PE(FileSafety.child("Minecraft.Windows.exe", of: install.path)),
              try FileSafety.hash(FileSafety.child("drive_c/windows/system32/gameinput.dll", of: bottle)) == profile().hashes["gameinput.dll"] else { throw LauncherError("BEDROCK_RUNTIME_INVALID", "Bedrock executable or GameInput is invalid.") }
        let config = try String(contentsOf: FileSafety.child("cxbottle.conf", of: bottle), encoding: .utf8)
        guard config.contains("\"CX_GRAPHICS_BACKEND\" = \"d3dmetal\""), config.contains(runtime.path) else { throw LauncherError("BEDROCK_RUNTIME_INVALID", "The owned environment lost its runtime configuration.") }
        if runProbes { try probe(install, runtime: runtime, crossOver: crossOver) }
    }
    public static func provision(_ install: Installation, root: URL, crossOver: CrossOver, owner: UUID) async throws {
        try crossOver.requireBedrockVersion()
        let runtime = try prepare(root: root)
        let path = try FileSafety.child(install.bottle, of: crossOver.bottlesRoot)
        if FileManager.default.fileExists(atPath: path.path) {
            try ManagedInstaller.validateOwner(bottle: path, name: install.bottle, game: .bedrock)
            guard try FileSafety.hash(FileSafety.child("drive_c/windows/system32/gameinput.dll", of: path)) == profile().hashes["gameinput.dll"] else { throw LauncherError("BEDROCK_RUNTIME_INVALID", "Retained GameInput needs repair. Your current environment is unchanged.") }
            return
        }
        else {
            guard install.bottle == "MinecraftMac-bedrock-\(owner.uuidString)" else { throw LauncherError("BOTTLE_INVALID", "Invalid Bedrock environment name.") }
            let bin = crossOver.wine.deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("bin/cxbottle")
            try await ManagedInstaller.run(bin, arguments: ["--bottle", install.bottle, "--create", "--template", "win11_64", "--description", "Minecraft on Mac • Bedrock", "--param", "EnvironmentVariables:CX_GRAPHICS_BACKEND=d3dmetal"])
            try FileSafety.write(["schema":"2", "gameId":"bedrock", "owner":owner.uuidString], to: FileSafety.child("minecraftmac-owner.json", of: path))
        }
        let bottle = try crossOver.bottleURL(install.bottle)
        let cx = crossOver.wine.deletingLastPathComponent().deletingLastPathComponent()
        let search = [runtime.path, cx.appendingPathComponent("lib/wine/x86_64-windows").path, cx.appendingPathComponent("lib/wine/i386-windows").path, cx.appendingPathComponent("lib/wine").path].joined(separator: ":")
        guard !search.contains("\""), !search.contains("\n"), !search.contains("\r") else { throw LauncherError("UNSAFE_PATH", "Unsupported runtime path.") }
        let configURL = try FileSafety.child("cxbottle.conf", of: bottle)
        var config = try String(contentsOf: configURL, encoding: .utf8)
        let begin = ";; BEGIN MINECRAFT MAC BEDROCK", end = ";; END MINECRAFT MAC BEDROCK"
        if let a = config.range(of: begin), let b = config.range(of: end, range: a.upperBound..<config.endIndex) { config.removeSubrange(a.lowerBound..<b.upperBound) }
        config += "\n\(begin)\n[Wine]\n\"DllPath\" = \"\(search)\"\n[EnvironmentVariables]\n\"CX_GRAPHICS_BACKEND\" = \"d3dmetal\"\n\(end)\n"
        try Data(config.utf8).write(to: configURL, options: .atomic)
        try await ManagedInstaller.run(crossOver.wine, arguments: ["--bottle", install.bottle, "--debugmsg", "-all", "wineboot", "-u"])
        let classes = try JSONDecoder().decode([String:String].self, from: Data(contentsOf: FileSafety.child("classes.json", of: runtime)))
        var registry = "Windows Registry Editor Version 5.00\n\n"
        for (name, dll) in classes.sorted(by: { $0.key < $1.key }) {
            guard name.allSatisfy({ $0.isLetter || $0 == "." }), dlls.contains(String(dll.dropLast(4))) else { throw LauncherError("BEDROCK_RUNTIME_INVALID", "WinRT registration is invalid.") }
            let path = "Z:" + runtime.appendingPathComponent("x86_64-windows/" + dll).path.replacingOccurrences(of: "/", with: "\\")
            registry += "[HKEY_LOCAL_MACHINE\\Software\\Microsoft\\WindowsRuntime\\ActivatableClassId\\\(name)]\n\"DllPath\"=\"\(path.replacingOccurrences(of: "\\", with: "\\\\"))\"\n\n"
        }
        registry += "[HKEY_CURRENT_USER\\Software\\Wine\\WineDbg]\n\"ShowCrashDialog\"=dword:00000000\n"
        let reg = try FileSafety.child("register-components.reg", of: install.path.deletingLastPathComponent())
        try registry.data(using: .utf16)!.write(to: reg, options: .atomic)
        try await ManagedInstaller.run(crossOver.wine, arguments: ["--bottle", install.bottle, "--debugmsg", "-all", "regedit", "/S", reg.path])
        let msi = try FileSafety.child("Installers/GameInputRedist.msi", of: install.path)
        do { try await ManagedInstaller.run(crossOver.wine, arguments: ["--bottle", install.bottle, "--debugmsg", "-all", "msiexec", "/i", msi.path, "/qn", "/norestart"], timeout: 600, accepted: [0,194]) }
        catch { throw LauncherError("BEDROCK_GAMEINPUT_FAILED", "GameInput installation failed.", recovery: "Your current version and worlds are preserved. Retry the managed installation.") }
        try ManagedInstaller.replace(FileSafety.child("gameinput.dll", of: install.path), destination: FileSafety.child("drive_c/windows/system32/gameinput.dll", of: bottle))
    }
    private static func probe(_ install: Installation, runtime: URL, crossOver: CrossOver) throws {
        let p = Process(); p.executableURL = crossOver.wine
        p.arguments = ["--bottle", install.bottle, "--debugmsg", "-all", "--dll", install.game.overrides, "--workdir", install.path.path, runtime.appendingPathComponent("bedrock-probe.exe").path]
        p.environment = CrossOver.environment(for: .bedrock)
        p.standardOutput = FileHandle.nullDevice; p.standardError = FileHandle.nullDevice
        try p.run()
        let timer = DispatchWorkItem { if p.isRunning { p.terminate() } }; DispatchQueue.global().asyncAfter(deadline: .now()+60, execute: timer)
        defer { timer.cancel() }; p.waitUntilExit()
        guard p.terminationStatus == 0 else { throw LauncherError("BEDROCK_RUNTIME_INVALID", "Bedrock WinRT or runtime probes failed.", recovery: "The current working version is unchanged.") }
    }
}
