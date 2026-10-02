import XCTest
@testable import LauncherCore

final class BedrockTests: XCTestCase {
    func temporary() throws -> URL {
        let root = URL(fileURLWithPath: FileManager.default.temporaryDirectory.path.hasPrefix("/var/") ? "/private" + FileManager.default.temporaryDirectory.path : FileManager.default.temporaryDirectory.path).appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: root) }; return root
    }
    func testVersionPolicyDistinguishesOldMalformedAndFuture() throws {
        try GameDefinition.bedrock.validateVersion("1.26.5203.0")
        for version in ["1.26.5202.0", "1.1.1.0", "0.26.5203.0"] {
            XCTAssertThrowsError(try GameDefinition.bedrock.validateVersion(version)) { XCTAssertEqual(($0 as? LauncherError)?.code, "BEDROCK_VERSION_TOO_OLD") }
        }
        for version in ["1.26.5204.0", "1.27.0.0", "2.0.0.0"] {
            XCTAssertThrowsError(try GameDefinition.bedrock.validateVersion(version)) { XCTAssertEqual(($0 as? LauncherError)?.code, "BEDROCK_COMPATIBILITY_UNVERIFIED") }
        }
        for version in ["", "1.26.5203", "1.26.5203.0.extra", "1.+26.5203.0", "1.26.65536.0", "1.٢٦.5203.0", "1.26..0"] { XCTAssertThrowsError(try PackageVersion(version)) }
    }
    func testIndependentSelectionsAndRetainedEnvironmentsRoundTrip() throws {
        var db = LibraryDatabase()
        let d = Installation(version: "1.1.1.0", path: URL(fileURLWithPath: "/game/dungeons"), bottle: "Dungeons", managed: false)
        let b = Installation(game: .bedrock, version: "1.26.5203.0", path: URL(fileURLWithPath: "/game/bedrock"), bottle: "Bedrock", managed: true)
        db.installations = [d,b]; db.currentByGameId = [d.gameId:d.id,b.gameId:b.id]
        db.retainedEnvironmentsByGameId = ["dungeons2":["D-saves"],"bedrock":["B-worlds"]]
        let result = try JSONDecoder().decode(LibraryDatabase.self, from: JSONEncoder().encode(db))
        XCTAssertEqual(result.current(for: .dungeons2)?.id, d.id); XCTAssertEqual(result.current(for: .bedrock)?.id, b.id)
        XCTAssertEqual(result.retainedEnvironmentsByGameId["bedrock"], ["B-worlds"])
        db.currentByGameId["bedrock"] = nil; XCTAssertEqual(db.current(for: .dungeons2)?.id,d.id)
    }
    func testSchemaOneMigrationMakesBackupAndPreservesDungeons() async throws {
        let root = try temporary()
        let install = Installation(version: "1.1.1.0", path: root.appendingPathComponent("external"), bottle: "Dungeons", managed: false)
        let object: [String:Any] = ["schema":1,"installations":[try JSONSerialization.jsonObject(with: JSONEncoder().encode(install))],"current":install.id.uuidString,"retainedEnvironments":["old-saves"]]
        let old = try JSONSerialization.data(withJSONObject: object)
        try old.write(to: root.appendingPathComponent("library.json"))
        let store = try LibraryStore(root: root, runtimeTests: false, isGameRunning: { false })
        let result = await store.snapshot()
        XCTAssertEqual(result.schema,2);XCTAssertEqual(result.current(for: .dungeons2)?.id,install.id)
        XCTAssertNil(result.current(for: .bedrock));XCTAssertEqual(result.retainedEnvironmentsByGameId["dungeons2"],["old-saves"])
        XCTAssertEqual(try Data(contentsOf: root.appendingPathComponent("library.schema1.backup.json")),old)
        let saved = try JSONSerialization.jsonObject(with: Data(contentsOf: root.appendingPathComponent("library.json"))) as! [String:Any]
        XCTAssertNil(saved["current"]);XCTAssertEqual(saved["schema"] as? Int,2)
    }
    func testCrossGameEnvironmentAndSelectionAreRejected() throws {
        var db = LibraryDatabase()
        let d = Installation(version: "1.1.1.0", path: URL(fileURLWithPath: "/d"), bottle: "same", managed: true)
        let b = Installation(game: .bedrock, version: "1.26.5203.0", path: URL(fileURLWithPath: "/b"), bottle: "same", managed: true)
        db.installations = [d,b]; XCTAssertThrowsError(try db.validate())
        db.installations = [d]; db.retainedEnvironmentsByGameId = ["bedrock":["same"]];XCTAssertThrowsError(try db.validate())
        db.retainedEnvironmentsByGameId = [:];db.currentByGameId = ["bedrock":d.id];XCTAssertThrowsError(try db.validate())
    }
    func testUnsupportedSchemaNeverResetsLibrary() throws {
        let root = try temporary();let data = Data("{\"schema\":99,\"installations\":[]}".utf8)
        try data.write(to: root.appendingPathComponent("library.json"));XCTAssertThrowsError(try LibraryStore(root: root))
        XCTAssertEqual(try Data(contentsOf: root.appendingPathComponent("library.json")),data)
    }
    func testBedrockLaunchUsesPersistentExecutableAndNoMaterializationEnvironment() throws {
        let root = try temporary()
        let b = Installation(game: .bedrock, version: "1.26.5203.0", path: root.appendingPathComponent("Games/bedrock/Versions/1.26.5203.0-id/Game"), bottle: "owned", managed: true)
        let cx = CrossOver(wine: root, bottlesRoot: root)
        let args = cx.launchArguments(b)
        XCTAssertEqual(args.last,b.path.appendingPathComponent("Minecraft.Windows.exe").path)
        XCTAssertTrue(args.contains(b.path.path));XCTAssertTrue(args.contains(b.game.overrides))
        for bad in ["ram://", "xodus", "crossover-launch.py", "hdiutil"] { XCTAssertFalse(args.joined(separator: " ").contains(bad)) }
        for bad in ["WINE_DLL_FILE_MAP", "MTL_HUD_ENABLED", "MML_BRIDGE_KEY", "XODUS_LOG"] { XCTAssertNil(CrossOver.environment(for: .bedrock)[bad]) }
    }
    func testCrossOverVersionGateIsExact() {
        for v in ["26.3", "26.3.0", "26.3.12"] { XCTAssertTrue(CrossOver.supportsBedrock(v)) }
        for v in ["26.30", "26.4", "26.2.9", "26.3beta", "26.3.", "27.3", "26.3.0\n"] { XCTAssertFalse(CrossOver.supportsBedrock(v)) }
    }
    func testIdentityValidationIsPerGame() throws {
        let root = try temporary()
        try Data("<Game><Identity Name=\"Microsoft.MinecraftUWP\" Version=\"1.26.5203.0\" /></Game>".utf8).write(to: root.appendingPathComponent("MicrosoftGame.config"))
        XCTAssertEqual(try FileSafety.version(root, game: .bedrock), "1.26.5203.0")
        XCTAssertThrowsError(try FileSafety.version(root, game: .dungeons2))
    }
    func testBedrockCannotDeleteCrossGamePathOrForeignEnvironment() async throws {
        let root = try temporary();let outside = try temporary()
        let b = Installation(game: .bedrock, version: "1.26.5203.0", path: outside, bottle: "foreign", managed: true, compatibilityProfile: "bedrock-managed-1.26.5203-r1")
        var db = LibraryDatabase();db.installations = [b];try FileSafety.write(db,to: root.appendingPathComponent("library.json"))
        let store = try LibraryStore(root:root,runtimeTests:false,isGameRunning:{false})
        do { try await store.remove(b.id);XCTFail("Unsafe deletion allowed") } catch {}
        XCTAssertTrue(FileManager.default.fileExists(atPath:outside.path));let snapshot = await store.snapshot();XCTAssertEqual(snapshot.installations.count,1)
    }
    func testStagingReceiptMustMatchItsGame() async throws {
        let root = try temporary();let store = try LibraryStore(root:root,runtimeTests:false,isGameRunning:{false})
        let id = UUID();let path = try FileSafety.child(".staging-\(id.uuidString)",of: GameDefinition.bedrock.versionRoot(in:root))
        try FileManager.default.createDirectory(at:path,withIntermediateDirectories:false)
        let wrong = Installation(id:id,version:"1.1.1.0",path:path.appendingPathComponent("Game"),bottle:"dungeons",managed:true)
        try FileSafety.write(wrong,to:path.appendingPathComponent("metadata.json"))
        do { try await store.removeStaging("bedrock/"+path.lastPathComponent);XCTFail("Cross-game staging removed") } catch {}
        XCTAssertTrue(FileManager.default.fileExists(atPath:path.path))
    }
    func testOwnershipReceiptCannotCrossGames() throws {
        let root = try temporary(),id = UUID()
        try FileSafety.write(["schema":"1","owner":id.uuidString],to:root.appendingPathComponent("minecraftmac-owner.json"))
        try ManagedInstaller.validateOwner(bottle:root,name:"MinecraftMac-\(id.uuidString)",game:.dungeons2)
        XCTAssertThrowsError(try ManagedInstaller.validateOwner(bottle:root,name:"MinecraftMac-\(id.uuidString)",game:.bedrock))
        try FileSafety.write(["schema":"2","owner":id.uuidString,"gameId":"bedrock"],to:root.appendingPathComponent("minecraftmac-owner.json"))
        try ManagedInstaller.validateOwner(bottle:root,name:"MinecraftMac-bedrock-\(id.uuidString)",game:.bedrock)
        XCTAssertThrowsError(try ManagedInstaller.validateOwner(bottle:root,name:"BedrockMacOS",game:.bedrock))
    }
    func testRegistrationUsesOwnedDllInsteadOfCrossOverBuiltin() throws {
        let root = try temporary()
        let runtime = root.appendingPathComponent("Owned runtime")
        try FileManager.default.createDirectory(at: runtime, withIntermediateDirectories: true)
        try FileManager.default.copyItem(at: BedrockRuntime.assets().appendingPathComponent("classes.json"), to: runtime.appendingPathComponent("classes.json"))
        let text = try XCTUnwrap(String(data: BedrockRuntime.registrationData(runtime: runtime), encoding: .utf16))
        XCTAssertTrue(text.contains("ActivatableClassId\\Windows.Foundation.Metadata.ApiInformation]"))
        XCTAssertTrue(text.contains("x86_64-windows\\\\wintypes.dll"))
        XCTAssertFalse(text.contains("system32"))
        let classes = try JSONDecoder().decode([String:String].self, from: Data(contentsOf: runtime.appendingPathComponent("classes.json")))
        XCTAssertEqual(text.components(separatedBy: "[HKEY_LOCAL_MACHINE").count - 1, classes.count)
        try Data("{\"Windows.Foundation.Metadata.ApiInformation\":\"untrusted.dll\"}".utf8).write(to: runtime.appendingPathComponent("classes.json"))
        XCTAssertThrowsError(try BedrockRuntime.registrationData(runtime: runtime))
    }
    func testBundledRuntimeProvenanceAndHashes() throws {
        guard let assets = try? BedrockRuntime.assets(), FileManager.default.fileExists(atPath: assets.appendingPathComponent("gameinput.dll").path) else { throw XCTSkip("Build release Bedrock resources with scripts/build_bedrock.py to test artifact integrity") }
        let profile = try BedrockRuntime.profile();XCTAssertEqual(profile.gameId,"bedrock")
        let root = try temporary();let runtime = try BedrockRuntime.prepare(root:root)
        for (name,hash) in profile.hashes { XCTAssertEqual(try FileSafety.hash(runtime.appendingPathComponent("x86_64-windows/"+name)),hash) }
        try Data("changed".utf8).write(to:runtime.appendingPathComponent("x86_64-unix/xgameruntime.so"))
        XCTAssertThrowsError(try BedrockRuntime.prepare(root:root))
    }
}
