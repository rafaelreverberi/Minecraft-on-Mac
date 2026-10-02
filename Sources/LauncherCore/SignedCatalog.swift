import Foundation
import CryptoKit

public struct SignedCatalog: Codable, Sendable {
    public let revision: Int
    public let profiles: [CompatibilityProfile]
    /// The caller must provide the release-pinned public key and persisted minimum revision.
    /// No network endpoint or trust-on-first-use key exists in the development preview.
    public static func verify(payload: Data, signature: Data, publicKey: Data, minimumRevision: Int) throws -> Self {
        guard payload.count <= 1024 * 1024, signature.count == 64, publicKey.count == 32,
              let key = try? Curve25519.Signing.PublicKey(rawRepresentation: publicKey),
              key.isValidSignature(signature, for: payload) else {
            throw LauncherError("CATALOG_SIGNATURE_INVALID", "Compatibility catalog signature could not be verified.")
        }
        let catalog = try JSONDecoder().decode(Self.self, from: payload)
        guard catalog.revision >= minimumRevision, catalog.revision > 0, catalog.profiles.count <= 1000 else {
            throw LauncherError("CATALOG_ROLLBACK_REJECTED", "Catalog revision is stale or invalid.")
        }
        var ids = Set<String>()
        for profile in catalog.profiles {
            guard ids.insert(profile.id).inserted, profile.gameId == "dungeons2", profile.revision > 0,
                  !profile.hashes.isEmpty, profile.hashes.count <= 100,
                  profile.hashes.values.allSatisfy({ $0.count == 64 && $0.allSatisfy { $0.isHexDigit && $0.isASCII } }) else {
                throw LauncherError("CATALOG_INVALID", "Compatibility profile schema is invalid.")
            }
            for relative in profile.hashes.keys {
                _ = try FileSafety.child(relative, of: URL(fileURLWithPath: "/catalog-validation-root"))
            }
        }
        return catalog
    }
}
