// SPDX-License-Identifier: AGPL-3.0-only
import SwiftUI

@main
struct LuqnexaProbeApp: App {
    var body: some Scene { WindowGroup { ProbeView() } }
}
struct ProbeView: View {
    @State private var report = "測定はボタンを押した時だけ実行します。"
    @State private var running = false
    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Text("Rust製AI・実験用") .font(.title2)
                    Text("約162KiBの候補順位モデル。辞書を含む変換精度・5ms目標の達成を証明するアプリではありません。入力履歴の取得、通信、自動学習は行いません。")
                    Button(running ? "測定中…" : "端末内で速度を測る") { measure() }
                        .buttonStyle(.borderedProminent).disabled(running)
                    Text(report).font(.system(.caption, design: .monospaced)).textSelection(.enabled)
                }.padding()
            }.navigationTitle("Luqnexa検証")
        }
    }
    private func measure() {
        running = true
        report = "測定中…"
        // A bounded, user-triggered task; never repeat automatically in the background.
        DispatchQueue.global(qos: .userInitiated).async {
            let result: String
            do {
                let model = try NeuralModel()
                let data = try JSONSerialization.data(withJSONObject: model.probe(), options: [.prettyPrinted, .sortedKeys])
                result = String(decoding: data, as: UTF8.self)
            } catch { result = "測定失敗: \(error)" }
            DispatchQueue.main.async { report = result; running = false }
        }
    }
}
