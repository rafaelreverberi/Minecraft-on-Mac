import Foundation

public struct DependencyStatus: Identifiable, Sendable {
    public let id: String
    public let name: String
    public let requiredForPlay: Bool
    public let available: Bool
    public let detail: String
}
public enum DependencyManager {
    /// On-demand discovery only; no installer, Homebrew mutation, or idle polling.
    public static func inspect(developerMode: Bool) -> [DependencyStatus] {
        var items = [DependencyStatus(id: "crossover", name: "CrossOver", requiredForPlay: true,
                                      available: CrossOver.detect() != nil, detail: "User-installed licensed CrossOver")]
        let probes = ["token-abi.exe", "xcurl_exports.exe", "runtime_exports.exe"]
        let bundled = probes.allSatisfy { Bundle.module.url(forResource: $0, withExtension: nil, subdirectory: "Resources/Probes") != nil }
        items.append(.init(id: "probes", name: "Compatibility self-tests", requiredForPlay: true, available: bundled, detail: "Built during packaging; no compiler needed to play"))
        guard developerMode else { return items }
        let home = FileManager.default.homeDirectoryForCurrentUser
        for (id, name, candidates) in [
            ("rust", "Rust / Cargo", [home.appendingPathComponent(".cargo/bin/cargo"), URL(fileURLWithPath: "/opt/homebrew/bin/cargo")]),
            ("mingw", "MinGW x86_64 cross-compiler", [URL(fileURLWithPath: "/opt/homebrew/bin/x86_64-w64-mingw32-gcc"), URL(fileURLWithPath: "/usr/local/bin/x86_64-w64-mingw32-gcc")]),
            ("brew", "Homebrew", [URL(fileURLWithPath: "/opt/homebrew/bin/brew"), URL(fileURLWithPath: "/usr/local/bin/brew")]),
            ("xcode", "Apple developer tools", [URL(fileURLWithPath: "/Applications/Xcode.app/Contents/Developer/usr/bin/xcodebuild")])
        ] {
            let present = candidates.contains { FileManager.default.isExecutableFile(atPath: $0.path) }
            items.append(.init(id: id, name: name, requiredForPlay: false, available: present, detail: "Development builds only. Never installed automatically."))
        }
        return items
    }
}
