// swift-tools-version: 6.2
import Foundation
import PackageDescription

let runtime = ProcessInfo.processInfo.environment["FLUID_AUDIO_PATH"]!
let package = Package(
    name: "GretchenASRBench",
    platforms: [.macOS(.v14)],
    dependencies: [.package(name: "FluidAudio", path: runtime, traits: [])],
    targets: [.executableTarget(name: "ASRBench", dependencies: [.product(name: "FluidAudio", package: "FluidAudio")])]
)
