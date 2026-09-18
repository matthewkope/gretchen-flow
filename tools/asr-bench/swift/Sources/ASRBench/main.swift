import FluidAudio
import Foundation

struct Clip: Decodable {
    let id: String
    let path: String
    let seconds: Double
}

func emit(_ row: [String: Any]) throws {
    let data = try JSONSerialization.data(withJSONObject: row, options: [.sortedKeys])
    FileHandle.standardOutput.write(data)
    FileHandle.standardOutput.write(Data([10]))
}

let args = CommandLine.arguments
guard args.count == 4, let repeats = Int(args[3]), repeats > 0 else {
    fatalError("Usage: ASRBench MODEL_DIRECTORY MANIFEST REPEATS")
}
let clips = try JSONDecoder().decode([Clip].self, from: Data(contentsOf: URL(fileURLWithPath: args[2])))
let audio: [[Float]] = try clips.map { clip in
    let data = try Data(contentsOf: URL(fileURLWithPath: clip.path))
    guard data.count % 4 == 0 else { throw NSError(domain: "Invalid PCM", code: 1) }
    return data.withUnsafeBytes { bytes in
        stride(from: 0, to: bytes.count, by: 4).map { offset in
            Float(bitPattern: UInt32(littleEndian: bytes.loadUnaligned(fromByteOffset: offset, as: UInt32.self)))
        }
    }
}
let loadStart = DispatchTime.now().uptimeNanoseconds
let models = try await AsrModels.load(from: URL(fileURLWithPath: args[1]), version: .v2)
let manager = AsrManager()
try await manager.loadModels(models)
try emit(["event": "load", "model": "parakeet-v2-coreml", "seconds": Double(DispatchTime.now().uptimeNanoseconds - loadStart) / 1e9])
for pass in 0...repeats {
    // Pass zero records a cold first call and warms the model; excluded from warm aggregates.
    let indices = pass == 0 ? [0] : Array(clips.indices)
    for i in indices {
        let start = DispatchTime.now().uptimeNanoseconds
        var state = try TdtDecoderState()
        let result = try await manager.transcribe(audio[i], decoderState: &state)
        let elapsed = Double(DispatchTime.now().uptimeNanoseconds - start) / 1e9
        try emit(["event": "transcribe", "model": "parakeet-v2-coreml", "id": clips[i].id,
                  "pass": pass, "seconds": elapsed, "audio_seconds": clips[i].seconds, "text": result.text])
    }
}
