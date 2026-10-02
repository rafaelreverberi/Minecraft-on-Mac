import Foundation

public struct LauncherError: Error, LocalizedError, Sendable, Equatable {
    public let code: String
    public let message: String
    public let recovery: String
    public init(_ code: String, _ message: String, recovery: String = "Review Diagnostics and retry.") {
        self.code = code; self.message = message; self.recovery = recovery
    }
    public var errorDescription: String? { "\(code): \(message)\n\(recovery)" }
}
public protocol GameAdapter: Sendable {
    var id: String { get }
    var name: String { get }
    var storeId: String? { get }
    var executable: String? { get }
    var supported: Bool { get }
}
public struct Dungeons2Adapter: GameAdapter {
    public init() {}
    public let id = "dungeons2", name = "Minecraft Dungeons II"
    public let storeId: String? = "9P5786PJB9RP"
    public let executable: String? = "Dungeons/Binaries/WinGDK/Dungeons-WinGDK-Shipping.exe"
    public let supported = true
}
public struct BedrockAdapter: GameAdapter {
    public init() {}
    public let id = "bedrock", name = "Minecraft: Bedrock"
    public let storeId: String? = "9NBLGGH2JHXJ", executable: String? = "Minecraft.Windows.exe"
    public let supported = true
}
public enum InstallState: String, Codable, Sendable {
    case preparing, testing, ready, failed
}
public struct Installation: Codable, Identifiable, Sendable, Equatable {
    public let id: UUID
    public let gameId: String
    public let storeId: String
    public let version: String
    public var path: URL
    public var bottle: String
    public let managed: Bool
    public var state: InstallState
    public var hashes: [String: String]
    public var compatibilityProfile: String
    public var packageRevision: String?
    public var origin: String?
    public var storageBytes: Int64?
    public var displayKind: String { !managed ? "Existing local installation" : (origin == "snapshot" ? "Local snapshot" : (isOfficialManaged ? "Downloaded installation" : "Local snapshot")) }
    public var game: GameDefinition { GameDefinition(rawValue: gameId)! }
    public var isOfficialManaged: Bool { managed && compatibilityProfile.hasPrefix(gameId + "-managed-") }
    public let installedAt: Date
    public var lastSuccessfulTest: Date?
    public var diskBytes: Int64
    public init(id: UUID = UUID(), game: GameDefinition = .dungeons2, version: String, path: URL, bottle: String, managed: Bool,
                state: InstallState = .testing, hashes: [String: String] = [:], compatibilityProfile: String = "", diskBytes: Int64 = 0) {
        self.id = id; self.gameId = game.rawValue; self.storeId = game.storeId; self.version = version
        self.path = path; self.bottle = bottle; self.managed = managed; self.state = state
        self.hashes = hashes; self.compatibilityProfile = compatibilityProfile; self.installedAt = Date()
        self.diskBytes = diskBytes
    }
}
public struct LibraryDatabase: Codable, Sendable {
    public var schema = 2
    public var installations: [Installation] = []
    public var currentByGameId: [String: UUID] = [:]
    public var retainedEnvironmentsByGameId: [String: [String]] = [:]
    // Source compatibility for older clients; serialized storage is always game scoped.
    public var current: UUID? {
        get { currentByGameId["dungeons2"] }
        set { currentByGameId["dungeons2"] = newValue }
    }
    public var retainedEnvironments: [String]? {
        get { retainedEnvironmentsByGameId["dungeons2"] }
        set { retainedEnvironmentsByGameId["dungeons2"] = newValue }
    }
    public func current(for game: GameDefinition) -> Installation? {
        installations.first { $0.gameId == game.rawValue && $0.id == currentByGameId[game.rawValue] }
    }
    public init() {}
    enum CodingKeys: String, CodingKey { case schema, installations, current, retainedEnvironments, currentByGameId, retainedEnvironmentsByGameId }
    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let old = try c.decode(Int.self, forKey: .schema)
        guard [1, 2].contains(old) else { throw LauncherError("DATABASE_VERSION", "Unsupported library schema. Your database is preserved.") }
        installations = try c.decode([Installation].self, forKey: .installations)
        guard Set(installations.map(\.id)).count == installations.count,
              installations.allSatisfy({ GameDefinition(rawValue: $0.gameId)?.storeId == $0.storeId }) else { throw LauncherError("DATABASE_INVALID", "Invalid game identity in library.") }
        if old == 1 {
            guard installations.allSatisfy({ $0.gameId == "dungeons2" }) else { throw LauncherError("DATABASE_INVALID", "Legacy library contains unexpected games.") }
            if let id = try c.decodeIfPresent(UUID.self, forKey: .current) { currentByGameId["dungeons2"] = id }
            retainedEnvironmentsByGameId["dungeons2"] = try c.decodeIfPresent([String].self, forKey: .retainedEnvironments) ?? []
        } else {
            currentByGameId = try c.decode([String: UUID].self, forKey: .currentByGameId)
            retainedEnvironmentsByGameId = try c.decode([String: [String]].self, forKey: .retainedEnvironmentsByGameId)
        }
        try validate()
    }
    public func validate() throws {
        var environments: [String: String] = [:]
        for install in installations {
            guard GameDefinition(rawValue: install.gameId)?.storeId == install.storeId else { throw LauncherError("DATABASE_INVALID", "Unknown game identity.") }
            if let game = environments[install.bottle], game != install.gameId { throw LauncherError("ENVIRONMENT_CROSS_GAME", "Different games cannot share an environment.") }
            environments[install.bottle] = install.gameId
        }
        for (game, names) in retainedEnvironmentsByGameId {
            guard GameDefinition(rawValue: game) != nil else { throw LauncherError("DATABASE_INVALID", "Unknown retained game.") }
            for name in names {
                if let previous = environments[name], previous != game { throw LauncherError("ENVIRONMENT_CROSS_GAME", "Retained environment belongs to another game.") }
                environments[name] = game
            }
        }
        for (game, id) in currentByGameId {
            guard installations.contains(where: { $0.id == id && $0.gameId == game }) else { throw LauncherError("DATABASE_INVALID", "Current selection does not match its game.") }
        }
    }
    public func encode(to encoder: Encoder) throws {
        try validate()
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(2, forKey: .schema); try c.encode(installations, forKey: .installations)
        try c.encode(currentByGameId, forKey: .currentByGameId)
        try c.encode(retainedEnvironmentsByGameId, forKey: .retainedEnvironmentsByGameId)
    }
}
public struct CompatibilityProfile: Codable, Sendable {
    public let id: String
    public let gameId: String
    public let version: String
    public let revision: Int
    public let hashes: [String: String]
    public static func managed(for game: GameDefinition = .dungeons2) throws -> Self {
        if game == .bedrock { return try BedrockRuntime.profile() }
        guard let url = Bundle.module.url(forResource: "compatibility-managed", withExtension: "json", subdirectory: "Resources") else { throw LauncherError("CATALOG_MISSING", "Managed compatibility profile is missing.") }
        return try JSONDecoder().decode(Self.self, from: Data(contentsOf: url))
    }
    public static func bundled() throws -> Self {
        guard let url = Bundle.module.url(forResource: "compatibility", withExtension: "json", subdirectory: "Resources") else {
            throw LauncherError("CATALOG_MISSING", "Bundled compatibility profile is missing.")
        }
        return try JSONDecoder().decode(Self.self, from: Data(contentsOf: url))
    }
}
public struct OperationProgress: Sendable {
    public let phase: String
    public let completed: Int64
    public let total: Int64
    public var fraction: Double? { total > 0 ? Double(completed) / Double(total) : nil }
    public init(_ phase: String, completed: Int64 = 0, total: Int64 = 0) {
        self.phase = phase; self.completed = completed; self.total = total
    }
}
