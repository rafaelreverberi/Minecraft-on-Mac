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
    public let storeId: String? = nil, executable: String? = nil
    public let supported = false
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
    public let installedAt: Date
    public var lastSuccessfulTest: Date?
    public var diskBytes: Int64
    public init(id: UUID = UUID(), version: String, path: URL, bottle: String, managed: Bool,
                state: InstallState = .testing, hashes: [String: String] = [:], compatibilityProfile: String = "", diskBytes: Int64 = 0) {
        self.id = id; self.gameId = "dungeons2"; self.storeId = "9P5786PJB9RP"; self.version = version
        self.path = path; self.bottle = bottle; self.managed = managed; self.state = state
        self.hashes = hashes; self.compatibilityProfile = compatibilityProfile; self.installedAt = Date()
        self.diskBytes = diskBytes
    }
}
public struct LibraryDatabase: Codable, Sendable {
    public var schema = 1
    public var installations: [Installation] = []
    public var current: UUID?
    public init() {}
}
public struct CompatibilityProfile: Codable, Sendable {
    public let id: String
    public let gameId: String
    public let version: String
    public let revision: Int
    public let hashes: [String: String]
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
