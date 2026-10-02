import XCTest
import CryptoKit
@testable import LauncherCore

final class CoreTests: XCTestCase {
    func temp() throws -> URL {
        let base = FileManager.default.temporaryDirectory.path
        let url = URL(fileURLWithPath: base.hasPrefix("/var/") ? "/private" + base : base).appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: url) }
        return url
    }
    func testPathTraversalAndAbsolutePathsRejected() throws {
        let root = try temp()
        for bad in ["../save", "/etc/passwd", "Game/../../save", "Game//file", "Game\\file", ".", "Game/./file"] {
            XCTAssertThrowsError(try FileSafety.child(bad, of: root), bad)
        }
        XCTAssertEqual(try FileSafety.child("Game/file", of: root), root.appendingPathComponent("Game/file"))
    }
    func testSymlinkEscapeRejectedIncludingRoot() throws {
        let root = try temp(), outside = try temp()
        try FileManager.default.createSymbolicLink(at: root.appendingPathComponent("Game"), withDestinationURL: outside)
        XCTAssertThrowsError(try FileSafety.child("Game/save", of: root))
        XCTAssertThrowsError(try FileSafety.child("save", of: root.appendingPathComponent("Game")))
        XCTAssertThrowsError(try FileSafety.validateTree(root))
    }
    func testHashStreamingAndPEArchitecture() throws {
        let root = try temp(), file = root.appendingPathComponent("test")
        try Data("abc".utf8).write(to: file)
        XCTAssertEqual(try FileSafety.hash(file), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        XCTAssertFalse(try FileSafety.isX64PE(file))
        var pe = Data(repeating: 0, count: 70); pe[0] = 0x4d; pe[1] = 0x5a; pe[60] = 64
        pe[64] = 0x50; pe[65] = 0x45; pe[68] = 0x64; pe[69] = 0x86
        try pe.write(to: file); XCTAssertTrue(try FileSafety.isX64PE(file))
        pe[69] = 0x01; try pe.write(to: file); XCTAssertFalse(try FileSafety.isX64PE(file))
    }
    func testLaunchPreservesHostedFlagsAndShippingExecutable() {
        let install = Installation(version: "1.1.1.0", path: URL(fileURLWithPath: "/tmp/game"), bottle: "User Bottle", managed: false)
        let crossover = CrossOver(wine: URL(fileURLWithPath: "/tmp/wine"), bottlesRoot: URL(fileURLWithPath: "/tmp/bottles"))
        XCTAssertEqual(crossover.launchArguments(install), ["--debugmsg", "-all", "--bottle", "User Bottle", "--dll", "xgameruntime=n;XCurl=n", "--workdir", "/tmp/game/Dungeons/Binaries/WinGDK", "/tmp/game/Dungeons/Binaries/WinGDK/Dungeons-WinGDK-Shipping.exe"])
        let env = CrossOver.environment()
        for key in ["WINE_DLL_FILE_MAP", "XGR_NO_DELEGATE", "RUST_LOG", "XODUS_LOG", "AUTHORIZATION", "COOKIE"] { XCTAssertNil(env[key]) }
        XCTAssertEqual(env["XCURL_COMPAT_IDENTITY"], "1")
    }
    func testBottlePathInjectionRejected() throws {
        let root = try temp()
        let crossover = CrossOver(wine: root, bottlesRoot: root)
        for name in ["../Bottle", "a/b", "a\\b", "", ".private", "a\nb"] { XCTAssertThrowsError(try crossover.bottleURL(name)) }
    }
    func testManifestRoundTripAndCorruptDatabasePreserved() async throws {
        let root = try temp()
        let db = LibraryDatabase()
        try FileSafety.write(db, to: root.appendingPathComponent("library.json"))
        do {
            let store = try LibraryStore(root: root, isGameRunning: { false })
            let result = await store.snapshot(); XCTAssertNil(result.current); XCTAssertTrue(result.installations.isEmpty)
            let invalid = Installation(version: "2.0.0.0", path: root.appendingPathComponent("missing"), bottle: "missing", managed: false)
            do { _ = try await store.verify(invalid); XCTFail("Unknown or missing build accepted") } catch {}
        }
        try Data("malformed".utf8).write(to: root.appendingPathComponent("library.json"))
        XCTAssertThrowsError(try LibraryStore(root: root, isGameRunning: { false }))
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("library.json"), encoding: .utf8), "malformed")
    }
    func testLibraryExclusiveWriterLock() throws {
        let root = try temp()
        let first = try LibraryStore(root: root, isGameRunning: { false })
        try withExtendedLifetime(first) { XCTAssertThrowsError(try LibraryStore(root: root, isGameRunning: { false })) }
    }
    func testUntrustedStagingCleanupAndCacheSymlinkRejected() async throws {
        let root = try temp(); let store = try LibraryStore(root: root, isGameRunning: { false })
        do { try await store.removeStaging(".staging-../../game"); XCTFail("Traversal accepted") } catch {}
        let staging = root.appendingPathComponent("Games/dungeons2/Versions/.staging-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: staging, withIntermediateDirectories: true)
        do { try await store.removeStaging(staging.lastPathComponent); XCTFail("Unmanifested cleanup accepted") } catch {}
        XCTAssertTrue(FileManager.default.fileExists(atPath: staging.path))
        try FileManager.default.createSymbolicLink(at: root.appendingPathComponent("Cache/escape"), withDestinationURL: root)
        do { try await store.clearCache(); XCTFail("Cache symlink accepted") } catch {}
        XCTAssertTrue(FileManager.default.fileExists(atPath: root.appendingPathComponent("Cache/escape").path))
    }
    func testDiagnosticsPositiveSchemaHasNoPathsOrAccountFields() async throws {
        let root = try temp(); try FileManager.default.createDirectory(at: root.appendingPathComponent("Logs"), withIntermediateDirectories: true)
        let diagnostics = Diagnostics(root: root)
        try await diagnostics.record(.launchExited, exitCode: 7)
        let data = try await diagnostics.report(); let text = String(decoding: data, as: UTF8.self)
        XCTAssertTrue(text.contains("launchExited")); XCTAssertTrue(text.contains("7"))
        for forbidden in [root.path, "username", "gamertag", "token", "Authorization", "XUID", "cookie"] { XCTAssertFalse(text.contains(forbidden)) }
    }
    func testAdapterPlaceholderAndPinnedCatalog() throws {
        XCTAssertFalse(BedrockAdapter().supported); XCTAssertNil(BedrockAdapter().executable)
        XCTAssertEqual(Dungeons2Adapter().storeId, "9P5786PJB9RP")
        let profile = try CompatibilityProfile.bundled()
        XCTAssertEqual(profile.hashes.count, 6); XCTAssertEqual(profile.version, "1.1.1.0")
    }
    func fakeReference() throws -> (URL, URL, CrossOver, CompatibilityProfile) {
        let root = try temp(), game = try temp(), bottleRoot = try temp()
        let bin = game.appendingPathComponent("Dungeons/Binaries/WinGDK")
        try FileManager.default.createDirectory(at: bin, withIntermediateDirectories: true)
        var pe = Data(repeating: 0, count: 70); pe[0] = 0x4d; pe[1] = 0x5a; pe[60] = 64
        pe[64] = 0x50; pe[65] = 0x45; pe[68] = 0x64; pe[69] = 0x86
        var hashes: [String:String] = [:]
        for name in ["Dungeons-WinGDK-Shipping.exe", "XCurl.dll", "XCurl2504.dll", "xgameruntime.dll", "xgameruntime.gdk.dll", "libHttpClient.GDK.dll"] {
            let file = bin.appendingPathComponent(name); try pe.write(to: file)
            hashes["Dungeons/Binaries/WinGDK/" + name] = try FileSafety.hash(file)
        }
        try pe.write(to: game.appendingPathComponent("xgameruntime.dll"))
        try Data(#"<Game><Identity Name="Microsoft.MinecraftDungeons2" Version="1.1.1.0" /></Game>"#.utf8).write(to: game.appendingPathComponent("MicrosoftGame.config"))
        let bottle = bottleRoot.appendingPathComponent("TestBottle")
        try FileManager.default.createDirectory(at: bottle.appendingPathComponent("drive_c/windows/system32"), withIntermediateDirectories: true)
        try pe.write(to: bottle.appendingPathComponent("drive_c/windows/system32/xgameruntime.dll"))
        try Data().write(to: bottle.appendingPathComponent("cxbottle.conf"))
        let crossover = CrossOver(wine: URL(fileURLWithPath: "/nonexistent-wine"), bottlesRoot: bottleRoot)
        let profile = CompatibilityProfile(id: "fixture", gameId: "dungeons2", version: "1.1.1.0", revision: 1, hashes: hashes)
        return (root, game, crossover, profile)
    }
    func testManagedSnapshotRollbackRepairAndRemovalPreserveReference() async throws {
        let (root, game, crossover, profile) = try fakeReference()
        let store = try LibraryStore(root: root, profile: profile, crossOver: crossover, runtimeTests: false, isGameRunning: { false })
        let original = try await store.registerExternal(game, bottle: "TestBottle")
        let copy = try await store.clone(original.id)
        var db = await store.snapshot(); XCTAssertEqual(db.current, original.id); XCTAssertEqual(db.installations.count, 2)
        let brokenDLL = copy.path.appendingPathComponent("Dungeons/Binaries/WinGDK/XCurl.dll")
        try Data("damage".utf8).write(to: brokenDLL)
        do { _ = try await store.reverify(copy.id); XCTFail("Damaged DLL accepted") } catch {}
        db = await store.snapshot(); XCTAssertEqual(db.current, original.id); XCTAssertEqual(db.installations.first(where: { $0.id == copy.id })?.state, .failed)
        try await store.repairCompatibility(copy.id, referenceID: original.id)
        XCTAssertEqual(try FileSafety.hash(brokenDLL), profile.hashes["Dungeons/Binaries/WinGDK/XCurl.dll"])
        try await store.select(copy.id)
        do { try await store.remove(copy.id); XCTFail("Current version removed") } catch {}
        try await store.select(original.id); try await store.remove(copy.id)
        XCTAssertFalse(FileManager.default.fileExists(atPath: copy.path.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: game.appendingPathComponent("MicrosoftGame.config").path))
        try await store.clearSelection(); try await store.remove(original.id)
        XCTAssertTrue(FileManager.default.fileExists(atPath: game.path))
        db = await store.snapshot(); XCTAssertNil(db.current); XCTAssertTrue(db.installations.isEmpty)
    }
    func testUnknownVersionCannotReplaceCurrent() async throws {
        let (root, game, crossover, profile) = try fakeReference()
        let store = try LibraryStore(root: root, profile: profile, crossOver: crossover, runtimeTests: false, isGameRunning: { false })
        let original = try await store.registerExternal(game, bottle: "TestBottle")
        let config = game.appendingPathComponent("MicrosoftGame.config")
        let valid = try Data(contentsOf: config)
        try Data(#"<Game><Identity Name="Microsoft.MinecraftDungeons2" Version="2.0.0.0" /></Game>"#.utf8).write(to: config)
        do { _ = try await store.clone(original.id); XCTFail("Unknown build cloned") } catch {}
        let db = await store.snapshot(); XCTAssertEqual(db.current, original.id); XCTAssertEqual(db.installations.count, 1)
        try valid.write(to: config)
    }
    func testAccountSchemaRejectsInconsistentEntitlementAndUnknownError() throws {
        let valid = Data(#"{"schema":1,"status":"ok","signedIn":true,"gamertag":"SyntheticTag","entitlement":"verified","availableVersion":"1.1.1.0"}"#.utf8)
        XCTAssertEqual(try AccountStatus.decode(valid).entitlement, "verified")
        for bad in [
            #"{"schema":1,"status":"ok","signedIn":false,"entitlement":"verified"}"#,
            #"{"schema":1,"status":"error","signedIn":false,"entitlement":"unknown","errorCode":"secret-value"}"#,
            #"{"schema":2,"status":"ok","signedIn":false,"entitlement":"unknown"}"#] {
            XCTAssertThrowsError(try AccountStatus.decode(Data(bad.utf8)))
        }
    }
    func testLiveReferenceWhenExplicitlyEnabled() async throws {
        guard ProcessInfo.processInfo.environment["MINECRAFT_LIVE_REFERENCE_TEST"] == "1" else { throw XCTSkip("Live CrossOver verification is opt-in.") }
        let game = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Games/MinecraftDungeons2-FullyDecrypted")
        let store = try LibraryStore(root: try temp())
        let install = try await store.registerExternal(game, bottle: "Dungeons2Xbox")
        XCTAssertEqual(install.state, .ready)
        XCTAssertNotNil(install.lastSuccessfulTest)
    }

    func testSignedCatalogTamperWrongKeyAndRollbackAreRejected() throws {
        let key = Curve25519.Signing.PrivateKey()
        let profile = try CompatibilityProfile.bundled()
        let catalog = SignedCatalog(revision: 3, profiles: [profile])
        let payload = try JSONEncoder().encode(catalog)
        let signature = try key.signature(for: payload)
        let verified = try SignedCatalog.verify(payload: payload, signature: signature, publicKey: key.publicKey.rawRepresentation, minimumRevision: 2)
        XCTAssertEqual(verified.revision, 3)
        var damaged = payload; damaged[0] ^= 1
        XCTAssertThrowsError(try SignedCatalog.verify(payload: damaged, signature: signature, publicKey: key.publicKey.rawRepresentation, minimumRevision: 2))
        XCTAssertThrowsError(try SignedCatalog.verify(payload: payload, signature: signature, publicKey: Curve25519.Signing.PrivateKey().publicKey.rawRepresentation, minimumRevision: 2))
        XCTAssertThrowsError(try SignedCatalog.verify(payload: payload, signature: signature, publicKey: key.publicKey.rawRepresentation, minimumRevision: 4))
    }

    func testCacheCleanupDeletesOnlyUnchangedReceiptedFiles() async throws {
        let root = try temp(); let store = try LibraryStore(root: root, isGameRunning: { false })
        let cache = root.appendingPathComponent("Cache"), file = cache.appendingPathComponent("download.bin")
        try Data("owned package fixture".utf8).write(to: file)
        do { try await store.clearCache(); XCTFail("Untracked file deleted") } catch {}
        XCTAssertTrue(FileManager.default.fileExists(atPath: file.path))
        try FileSafety.write(["download.bin": FileSafety.hash(file)], to: root.appendingPathComponent("cache-manifest.json"))
        try await store.clearCache(); XCTAssertFalse(FileManager.default.fileExists(atPath: file.path))
    }

    func testRunningGameBlocksMutationBeforeTouchingReference() async throws {
        let (root, game, crossover, profile) = try fakeReference()
        let store = try LibraryStore(root: root, profile: profile, crossOver: crossover, runtimeTests: false, isGameRunning: { true })
        let before = try FileSafety.hash(game.appendingPathComponent("MicrosoftGame.config"))
        do { _ = try await store.registerExternal(game, bottle: "TestBottle"); XCTFail("Running-game guard bypassed") } catch {
            XCTAssertEqual((error as? LauncherError)?.code, "GAME_RUNNING")
        }
        XCTAssertEqual(try FileSafety.hash(game.appendingPathComponent("MicrosoftGame.config")), before)
        let db = await store.snapshot(); XCTAssertTrue(db.installations.isEmpty)
    }

}
