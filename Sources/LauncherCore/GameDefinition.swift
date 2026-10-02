import Foundation

/// The registry owns package identity and launch policy, not views or the database.
public enum GameDefinition: String, Codable, CaseIterable, Sendable {
    case dungeons2, bedrock
    public var name: String { self == .dungeons2 ? "Minecraft Dungeons II" : "Minecraft: Bedrock" }
    public var storeId: String { self == .dungeons2 ? "9P5786PJB9RP" : "9NBLGGH2JHXJ" }
    public var packageIdentity: String { self == .dungeons2 ? "Microsoft.MinecraftDungeons2" : "Microsoft.MinecraftUWP" }
    public var baseline: String { self == .dungeons2 ? "1.1.1.0" : "1.26.5203.0" }
    public var executable: String { self == .dungeons2 ? "Dungeons/Binaries/WinGDK/Dungeons-WinGDK-Shipping.exe" : "Minecraft.Windows.exe" }
    public var workingDirectory: String? { self == .dungeons2 ? "Dungeons/Binaries/WinGDK" : nil }
    public var overrides: String { self == .dungeons2 ? "xgameruntime=n;XCurl=n" : BedrockRuntime.dlls.map { $0 + "=n,b" }.joined(separator: ";") }
    public var localSavePaths: [String] { self == .dungeons2 ? ["Dungeons/Binaries/WinGDK/savedata", "Dungeons/Saved/SaveGames"] : ["minecraftWorlds", "games/com.mojang/minecraftWorlds"] }
    public func validateVersion(_ version: String) throws {
        let value = try PackageVersion(version)
        let minimum = try PackageVersion(baseline)
        if self == .bedrock && value < minimum { throw LauncherError("BEDROCK_VERSION_TOO_OLD", "Bedrock requires version \(baseline) or newer.", recovery: "Your current working installation is unchanged.") }
        guard value == minimum else { throw LauncherError(self == .bedrock ? "BEDROCK_COMPATIBILITY_UNVERIFIED" : "COMPATIBILITY_UNVERIFIED", "Compatibility not verified yet for \(version).", recovery: "Your current working version is unchanged. A newer launcher compatibility profile is required.") }
    }
    public func versionRoot(in root: URL) throws -> URL { try FileSafety.child("Games/\(rawValue)/Versions", of: root) }
}
public struct PackageVersion: Comparable, Sendable {
    public let parts: [UInt16]
    public init(_ value: String) throws {
        let pieces = value.split(separator: ".", omittingEmptySubsequences: false)
        guard pieces.count == 4, pieces.allSatisfy({ !$0.isEmpty && $0.allSatisfy({ $0.isASCII && $0.isNumber }) }), pieces.allSatisfy({ UInt16($0) != nil }) else { throw LauncherError("PACKAGE_VERSION_INVALID", "Game version must contain four valid numbers.") }
        parts = pieces.map { UInt16($0)! }
    }
    public static func < (lhs: Self, rhs: Self) -> Bool { lhs.parts.lexicographicallyPrecedes(rhs.parts) }
}
