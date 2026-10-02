import Foundation

/// Positive schema: raw child output, arbitrary error descriptions and account identifiers are never accepted.
public actor Diagnostics {
    public struct Event: Codable, Sendable {
        public let date: Date
        public let code: Code
        public let exitCode: Int32?
    }
    public enum Code: String, Codable, Sendable {
        case launcherStarted, installationVerified, operationFailed, launchStarted, launchExited, snapshotCreated, versionRemoved, cacheCleared
    }
    private var events: [Event] = []
    private let root: URL
    public init(root: URL) {
        self.root = root
        if let url = try? FileSafety.child("Logs/events.json", of: root), let data = try? Data(contentsOf: url),
           data.count <= 1024 * 1024, let saved = try? JSONDecoder().decode([Event].self, from: data) {
            events = Array(saved.suffix(200))
        }
    }
    public func record(_ code: Code, exitCode: Int32? = nil) throws {
        events.append(Event(date: Date(), code: code, exitCode: exitCode)); events = Array(events.suffix(200))
        let url = try FileSafety.child("Logs/events.json", of: root)
        try FileSafety.write(events, to: url)
    }
    public func report() throws -> Data {
        struct Report: Encodable {
            let schema = 1
            let application = "Minecraft on Mac development preview"
            let architecture = "arm64"
            let crossOverDetected: Bool
            let events: [Event]
        }
        return try JSONEncoder().encode(Report(crossOverDetected: CrossOver.detect() != nil, events: events))
    }
}
