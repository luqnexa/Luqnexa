// SPDX-License-Identifier: AGPL-3.0-only
import Foundation

final class NeuralModel {
    enum Failure: Error { case missingModel, rust(Int32) }
    private var handle: UInt64 = 0
    init() throws {
        guard let url = Bundle.main.url(forResource: "model", withExtension: "lqml") else {
            throw Failure.missingModel
        }
        let bytes = try Data(contentsOf: url)
        let result = bytes.withUnsafeBytes { raw in
            luqnexa_model_create(raw.bindMemory(to: UInt8.self).baseAddress, raw.count, &handle)
        }
        guard result == Int32(NEXTIME_OK.rawValue) else { throw Failure.rust(result) }
    }
    deinit { if handle != 0 { _ = luqnexa_model_destroy(handle) } }
    func score(reading: [UInt8], context: [UInt8], surface: [UInt8]) throws -> Float {
        var value: Float = 0
        let result = reading.withUnsafeBufferPointer { r in
            context.withUnsafeBufferPointer { c in
                surface.withUnsafeBufferPointer { s in
                    luqnexa_model_score(handle, r.baseAddress, r.count, c.baseAddress, c.count,
                                       s.baseAddress, s.count, &value)
                }
            }
        }
        guard result == Int32(NEXTIME_OK.rawValue) else { throw Failure.rust(result) }
        return value
    }
    func probe() throws -> [String: Any] {
        let reading = Array("pythondeaiwotsukuru".utf8)
        let context = Array("プログラミングの話をしている".utf8)
        let candidates = ["PythonでAIを作る", "Pythonで愛を作る", "PythonでAIをつくる", "pythonでAIを作る"]
            .map { Array($0.utf8) }
        var observations: [[String: Any]] = []
        for count in [1, 8, 32] {
            var samples: [Double] = []
            var checksum: Float = 0
            for iteration in 0..<600 {
                let start = DispatchTime.now().uptimeNanoseconds
                for n in 0..<count {
                    checksum += try score(reading: reading, context: context,
                                          surface: candidates[n % candidates.count])
                }
                let elapsed = Double(DispatchTime.now().uptimeNanoseconds - start) / 1000
                if iteration >= 100 { samples.append(elapsed) }
            }
            samples.sort()
            observations.append(["candidates": count, "iterations": 500, "warmup": 100,
                                 "p50_us": samples[250], "p95_us": samples[475],
                                 "p99_us": samples[495], "checksum": checksum])
        }
        var metadata: [String: Any] = [:]
        if let url = Bundle.main.url(forResource: "training-report", withExtension: "json"),
           let data = try? Data(contentsOf: url),
           let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
            metadata["model_sha256"] = json["model_sha256"]
            metadata["synthetic_pair_evaluation"] = json["after"]
        }
        return ["scope": "C ABI supplied candidate scores; excludes dictionary/UI; repeated candidates",
                "os": ProcessInfo.processInfo.operatingSystemVersionString,
                "model_bytes": Int(LUQNEXA_MODEL_BYTES),
                "training_metadata": metadata,
                "low_power": ProcessInfo.processInfo.isLowPowerModeEnabled,
                "thermal_state": ProcessInfo.processInfo.thermalState.rawValue,
                "measurements": observations]
    }
}
