// swift-tools-version: 6.0
import PackageDescription
let package = Package(name: "MinecraftMacLauncher", platforms: [.macOS(.v14)], products: [
    .library(name: "LauncherCore", targets: ["LauncherCore"]),
    .executable(name: "MinecraftMacLauncher", targets: ["MinecraftMacLauncher"])
], targets: [
    .target(name: "LauncherCore", resources: [.copy("Resources")]),
    .executableTarget(name: "MinecraftMacLauncher", dependencies: ["LauncherCore"]),
    .testTarget(name: "LauncherCoreTests", dependencies: ["LauncherCore"])
])
