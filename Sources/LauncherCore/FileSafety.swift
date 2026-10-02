import Foundation
import CryptoKit

public enum FileSafety {
    public static func hash(_ url: URL) throws -> String {
        let h = try FileHandle(forReadingFrom: url); defer { try? h.close() }
        var sha = SHA256()
        while let data = try h.read(upToCount: 1024 * 1024), !data.isEmpty {
            try Task.checkCancellation(); sha.update(data: data)
        }
        return sha.finalize().map { String(format: "%02x", $0) }.joined()
    }
    /// Reject path traversal and every symlink component, including an existing root.
    public static func child(_ relative: String, of root: URL) throws -> URL {
        let parts = relative.split(separator: "/", omittingEmptySubsequences: false)
        guard !parts.isEmpty, !relative.hasPrefix("/"), parts.allSatisfy({ !$0.isEmpty && $0 != "." && $0 != ".." && !$0.contains("\\") }) else {
            throw LauncherError("UNSAFE_PATH", "A manifest path is invalid.")
        }
        guard root.isFileURL, !root.pathComponents.contains(".."), !root.pathComponents.contains(".") else { throw LauncherError("UNSAFE_PATH", "Installation root is invalid.") }
        let target = root.appendingPathComponent(relative)
        guard target.path.hasPrefix(root.path + "/") else { throw LauncherError("UNSAFE_PATH", "Path escaped installation.") }
        var cursor = URL(fileURLWithPath: "/")
        for part in target.pathComponents.dropFirst() {
            cursor.appendPathComponent(part)
            if let attrs = try? FileManager.default.attributesOfItem(atPath: cursor.path), attrs[.type] as? FileAttributeType == .typeSymbolicLink {
                throw LauncherError("SYMLINK_REJECTED", "Installation contains a symbolic link.")
            }
        }
        return target
    }
    public static func validateTree(_ root: URL) throws {
        _ = try child("validation-probe", of: root)
        guard let walk = FileManager.default.enumerator(at: root, includingPropertiesForKeys: [.isSymbolicLinkKey]) else {
            throw LauncherError("INSTALL_MISSING", "Cannot read the installation directory.")
        }
        for case let path as URL in walk {
            try Task.checkCancellation()
            if try path.resourceValues(forKeys: [.isSymbolicLinkKey]).isSymbolicLink == true {
                throw LauncherError("SYMLINK_REJECTED", "Refusing to manage an installation containing symbolic links.")
            }
        }
    }
    public static func write<T: Encodable>(_ value: T, to url: URL) throws {
        let encoder = JSONEncoder(); encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        try encoder.encode(value).write(to: url, options: [.atomic])
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
    }
    public static func isX64PE(_ url: URL) throws -> Bool {
        let file = try FileHandle(forReadingFrom: url); defer { try? file.close() }
        guard let header = try file.read(upToCount: 64), header.count == 64, header[0] == 0x4d, header[1] == 0x5a else { return false }
        let offset = (0..<4).reduce(UInt32(0)) { $0 | UInt32(header[60 + $1]) << ($1 * 8) }
        try file.seek(toOffset: UInt64(offset))
        guard let pe = try file.read(upToCount: 6), pe.count == 6 else { return false }
        return Array(pe.prefix(4)) == [0x50,0x45,0,0] && pe[4] == 0x64 && pe[5] == 0x86
    }
    public static func version(_ root: URL, game: GameDefinition = .dungeons2) throws -> String {
        let config = try child("MicrosoftGame.config", of: root)
        let data = try Data(contentsOf: config)
        guard data.count < 1024 * 1024 else { throw LauncherError("GAME_IDENTITY_INVALID", "Game configuration is too large.") }
        let delegate = GameIdentityParser()
        let parser = XMLParser(data: data); parser.shouldResolveExternalEntities = false; parser.delegate = delegate
        guard parser.parse(), delegate.count == 1, delegate.name == game.packageIdentity, let version = delegate.version else { throw LauncherError("GAME_IDENTITY_INVALID", "Select the matching game installation folder.") }
        _ = try PackageVersion(version)
        return version
    }
    public static func diskUsage(_ root: URL) throws -> Int64 {
        var bytes: Int64 = 0
        let items = FileManager.default.enumerator(at: root, includingPropertiesForKeys: [.fileSizeKey, .isRegularFileKey])
        while let url = items?.nextObject() as? URL {
            try Task.checkCancellation()
            let v = try url.resourceValues(forKeys: [.fileSizeKey, .isRegularFileKey])
            if v.isRegularFile == true { bytes += Int64(v.fileSize ?? 0) }
        }
        return bytes
    }
}

private final class GameIdentityParser: NSObject, XMLParserDelegate {
    var count = 0
    var name: String?
    var version: String?
    func parser(_ parser: XMLParser, didStartElement elementName: String, namespaceURI: String?, qualifiedName qName: String?, attributes attributeDict: [String:String]) {
        if elementName == "Identity" { count += 1; name = attributeDict["Name"]; version = attributeDict["Version"] }
    }
    func parser(_ parser: XMLParser, foundExternalEntityDeclarationWithName name: String, publicID: String?, systemID: String?) { parser.abortParsing() }
}
