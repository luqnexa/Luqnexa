// SPDX-License-Identifier: AGPL-3.0-only
import SwiftUI
@main
struct LuqnexaProbeApp: App {
    var body: some Scene { WindowGroup { NavigationStack { VStack(spacing: 16) {
        Text("Luqnexaキーボード").font(.title2)
        Button("キーボード設定を開く") {
            if let url = URL(string: UIApplication.openSettingsURLString) { UIApplication.shared.open(url) }
        }
        Text("設定 → 一般 → キーボードからLuqnexaを追加してください。フルアクセスは不要です。")
    }.padding() } } }
}
