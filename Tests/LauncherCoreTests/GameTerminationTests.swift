import XCTest
import Foundation
import Darwin
@testable import LauncherCore

final class GameTerminationTests: XCTestCase {
    func record(_ pid: Int32, parent: Int32 = 1, started: UInt64 = 1, path: String = "/fixture/CrossOver.app/Contents/wine", command: String = "") -> GameProcessIdentity {
        .init(pid: pid, parent: parent, uid: getuid(), startedSeconds: started, startedMicroseconds: 10, executable: path, command: command)
    }
    func testPIDReuseAndExecutableChangeAreDifferentIdentities() {
        let original = record(100)
        XCTAssertFalse(original.sameProcess(as: record(100, started: 2)))
        XCTAssertFalse(original.sameProcess(as: record(100, path: "/bin/sleep")))
        XCTAssertTrue(original.sameProcess(as: record(100, parent: 42)))
    }
    func testTreeKeepsOtherGameAndSharedWineServer() {
        let cx = CrossOver(wine: URL(fileURLWithPath: "/fixture/CrossOver.app/Contents/wine"), bottlesRoot: URL(fileURLWithPath: "/fixture/bottles"))
        let root = record(100), child = record(101, parent: 100), grandchild = record(102, parent: 101)
        let siblingGame = record(200), server = record(103, parent: 100, path: "/fixture/CrossOver.app/Contents/wineserver")
        let unrelated = record(104, parent: 100, path: "/bin/sleep")
        let targets = GameTermination.tree(roots: [root], snapshot: [root, child, grandchild, siblingGame, server, unrelated]) { GameTermination.belongsToRuntime($0, crossOver: cx) }
        XCTAssertEqual(targets.map(\.pid), [102, 101, 100])
    }
    func testExactInstallationPathAndRuntimeRequired() {
        let cx = CrossOver(wine: URL(fileURLWithPath: "/fixture/CrossOver.app/Contents/wine"), bottlesRoot: URL(fileURLWithPath: "/fixture/bottles"))
        let install = Installation(game: .bedrock, version: "1.26.5203.0", path: URL(fileURLWithPath: "/fixture/Game"), bottle: "b", managed: true)
        XCTAssertTrue(GameTermination.matches(record(100, command: "/fixture/Game/Minecraft.Windows.exe"), installation: install, crossOver: cx))
        XCTAssertFalse(GameTermination.matches(record(100, command: "/other/Game/Minecraft.Windows.exe"), installation: install, crossOver: cx))
        XCTAssertFalse(GameTermination.matches(record(100, path: "/bin/sleep", command: "/fixture/Game/Minecraft.Windows.exe"), installation: install, crossOver: cx))
    }
    func testForceQuitOnlyDisposableGameTree() throws {
        let root = FileManager.default.temporaryDirectory.resolvingSymlinksInPath().appendingPathComponent("GameQuit-" + UUID().uuidString)
        let app = root.appendingPathComponent("Fixture.app/Contents")
        let game = app.appendingPathComponent("Game")
        try FileManager.default.createDirectory(at: game, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let executable = game.appendingPathComponent("Minecraft.Windows.exe"), child = app.appendingPathComponent("sleep")
        let source = root.appendingPathComponent("fixture.c")
        try """
        #include <unistd.h>
        #include <stdlib.h>
        int main(int argc, char **argv) {
            if (argc > 1) {
                pid_t child = fork();
                if (child == 0) { execl(argv[1], argv[1], (char *)0); _exit(127); }
                if (child < 0) return 1;
            }
            sleep(30);
            return 0;
        }
        """.write(to: source, atomically: true, encoding: .utf8)
        for binary in [executable, child] {
            let compiler = Process()
            compiler.executableURL = URL(fileURLWithPath: "/usr/bin/cc")
            compiler.arguments = [source.path, "-o", binary.path]
            compiler.standardOutput = FileHandle.nullDevice; compiler.standardError = FileHandle.nullDevice
            try compiler.run(); compiler.waitUntilExit()
            XCTAssertEqual(compiler.terminationStatus, 0)
        }
        let process = Process(), unrelated = Process()
        process.executableURL = executable
        process.arguments = [child.path]
        process.standardOutput = FileHandle.nullDevice; process.standardError = FileHandle.nullDevice
        unrelated.executableURL = child; unrelated.arguments = []
        try unrelated.run(); try process.run()
        defer {
            if process.isRunning { process.terminate() }; process.waitUntilExit()
            if unrelated.isRunning { unrelated.terminate() }; unrelated.waitUntilExit()
        }
        let cx = CrossOver(wine: app.appendingPathComponent("wine"), bottlesRoot: root)
        let install = Installation(game: .bedrock, version: "1.26.5203.0", path: game, bottle: "fixture", managed: true)
        var descendants: [GameProcessIdentity] = []
        for _ in 0..<50 {
            descendants = try GameTermination.snapshot().filter { $0.parent == process.processIdentifier }
            if !descendants.isEmpty { break }
            Thread.sleep(forTimeInterval: 0.02)
        }
        XCTAssertFalse(descendants.isEmpty)
        XCTAssertEqual(GameTermination.runningGames(installations: [install], crossOver: cx), [.bedrock])
        try GameTermination.forceQuit(game: .bedrock, installations: [install], crossOver: cx)
        process.waitUntilExit()
        XCTAssertEqual(process.terminationReason, .uncaughtSignal)
        XCTAssertTrue(unrelated.isRunning, "An unrelated process in the same runtime must survive.")
        for target in descendants {
            for _ in 0..<50 {
                if GameTermination.identity(target.pid) == nil { break }
                Thread.sleep(forTimeInterval: 0.02)
            }
            XCTAssertNil(GameTermination.identity(target.pid), "The game's child should exit as well.")
        }
    }
}
