import Foundation
import Darwin

/// A PID alone is not an identity: the kernel start time and executable must still match.
struct GameProcessIdentity: Equatable, Sendable {
    let pid: Int32
    let parent: Int32
    let uid: UInt32
    let startedSeconds: UInt64
    let startedMicroseconds: UInt64
    let executable: String
    let command: String

    func sameProcess(as other: Self) -> Bool {
        pid == other.pid && uid == other.uid && startedSeconds == other.startedSeconds &&
        startedMicroseconds == other.startedMicroseconds && executable == other.executable
    }
}

public enum GameTermination {
    static func identity(_ pid: Int32, command: String = "") -> GameProcessIdentity? {
        guard pid > 1, pid != getpid() else { return nil }
        var info = proc_bsdinfo()
        guard proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &info, Int32(MemoryLayout.size(ofValue: info))) == MemoryLayout.size(ofValue: info),
              info.pbi_uid == getuid(), info.pbi_status != SZOMB else { return nil }
        var path = [CChar](repeating: 0, count: 4 * Int(MAXPATHLEN))
        guard proc_pidpath(pid, &path, UInt32(path.count)) > 0 else { return nil }
        let executable = String(decoding: path.prefix { $0 != 0 }.map { UInt8(bitPattern: $0) }, as: UTF8.self)
        return .init(pid: pid, parent: Int32(info.pbi_ppid), uid: info.pbi_uid,
                     startedSeconds: info.pbi_start_tvsec, startedMicroseconds: info.pbi_start_tvusec,
                     executable: executable, command: command)
    }

    static func snapshot() throws -> [GameProcessIdentity] {
        let process = Process(), pipe = Pipe()
        process.executableURL = URL(fileURLWithPath: "/bin/ps")
        process.arguments = ["-axo", "pid=,comm="]
        process.standardOutput = pipe; process.standardError = FileHandle.nullDevice
        try process.run()
        let data = pipe.fileHandleForReading.readDataToEndOfFile(); process.waitUntilExit()
        guard process.terminationStatus == 0 else { throw LauncherError("PROCESS_INSPECTION_FAILED", "Could not inspect the running game.") }
        return String(decoding: data, as: UTF8.self).split(separator: "\n").compactMap { line in
            let parts = line.split(maxSplits: 1, whereSeparator: { $0.isWhitespace })
            guard parts.count == 2, let pid = Int32(parts[0]) else { return nil }
            return identity(pid, command: String(parts[1]))
        }
    }

    static func belongsToRuntime(_ process: GameProcessIdentity, crossOver: CrossOver) -> Bool {
        let app = crossOver.wine.path.components(separatedBy: "/Contents/")[0]
        let binary = URL(fileURLWithPath: process.executable).resolvingSymlinksInPath().path
        return binary.hasPrefix(URL(fileURLWithPath: app).resolvingSymlinksInPath().path + "/Contents/") &&
            !URL(fileURLWithPath: binary).lastPathComponent.lowercased().contains("wineserver")
    }

    static func matches(_ process: GameProcessIdentity, installation: Installation, crossOver: CrossOver) -> Bool {
        guard belongsToRuntime(process, crossOver: crossOver) else { return false }
        let expected = installation.path.appendingPathComponent(installation.game.executable).resolvingSymlinksInPath().path
        let command = process.command
        if command.hasPrefix("/") { return URL(fileURLWithPath: command).resolvingSymlinksInPath().path == expected }
        let chars = Array(command)
        guard chars.count > 3, chars[1] == ":", chars[2] == "\\", chars[0].isASCII, chars[0].isLetter,
              let bottle = try? crossOver.bottleURL(installation.bottle) else { return false }
        let drive = bottle.appendingPathComponent("dosdevices/" + String(chars[0]).lowercased() + ":")
        guard (try? FileManager.default.destinationOfSymbolicLink(atPath: drive.path)) != nil else { return false }
        let relative = String(chars.dropFirst(3)).replacingOccurrences(of: "\\", with: "/")
        let actual = drive.resolvingSymlinksInPath().appendingPathComponent(relative).resolvingSymlinksInPath().path
        return actual == expected
    }

    static func tree(roots: [GameProcessIdentity], snapshot: [GameProcessIdentity], allowed: (GameProcessIdentity) -> Bool) -> [GameProcessIdentity] {
        var selected = roots.filter(allowed)
        var seen = Set(selected.map(\.pid))
        while true {
            let children = snapshot.filter { !seen.contains($0.pid) && seen.contains($0.parent) && allowed($0) }
            guard !children.isEmpty else { break }
            selected += children; seen.formUnion(children.map(\.pid))
        }
        return selected.reversed() // Children first; never signal a shared Wine server.
    }

    public static func runningGames(installations: [Installation], crossOver: CrossOver?) -> Set<GameDefinition> {
        guard let crossOver, let processes = try? snapshot() else { return [] }
        return Set(installations.filter { installation in processes.contains { matches($0, installation: installation, crossOver: crossOver) } }.map(\.game))
    }

    /// The red X explicitly authorizes force quit. No process-group or global Wine kill.
    public static func forceQuit(game: GameDefinition, installations: [Installation], crossOver: CrossOver) throws {
        let processes = try snapshot()
        let copies = installations.filter { $0.game == game }
        let roots = processes.filter { process in copies.contains { matches(process, installation: $0, crossOver: crossOver) } }
        guard !roots.isEmpty else { throw LauncherError("GAME_PROCESS_NOT_FOUND", "This game's process is no longer available.", recovery: "Refresh the game page and try again if it is still running.") }
        let targets = tree(roots: roots, snapshot: processes) { $0.uid == getuid() && belongsToRuntime($0, crossOver: crossOver) }
        for target in targets {
            guard let live = identity(target.pid), target.sameProcess(as: live) else { continue }
            guard kill(target.pid, SIGKILL) == 0 || errno == ESRCH else {
                throw LauncherError("GAME_QUIT_FAILED", "The game process could not be stopped.", recovery: "Quit the game in CrossOver and refresh the launcher.")
            }
        }
    }
}
