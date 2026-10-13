# RustモデルのiPhone実機検証アプリ

SwiftUIからRust製int8モデルをC ABIで呼び出す、実験用の親アプリです。キーボード拡張ではありません。モデルの学習もRustで行います。ダウンロードした教材、ユーザー入力履歴、個人辞書を取り込みません。

`.github/workflows/ios-probe.yml` はGitHubのmacOSランナーでRustのiPhone静的ライブラリとアプリを最終リンクし、`LuqnexaProbe-unsigned.ipa` を生成します。個人の証明書をGitHubへ置かず、署名前のIPAを成果物として保存します。iPhoneでの再署名・インストールは、ユーザーのAltStore Classic／SideStore環境で行います。

GitHub Actionsの「iPhone Rust probe IPA」が成功したrunを開き、`LuqnexaProbe-unsigned-iphone` artifactをダウンロードして展開します。中の`.ipa`をiPhoneへ渡し、AltStore／SideStoreのIPA取り込み機能で追加してください。インストール手順は利用中のツールの公式手順に従ってください。

- AltStore Classic: https://faq.altstore.io/altstore-classic/your-altstore
- SideStore: https://docs.sidestore.io/docs/faq

アプリの「端末内で速度を測る」を押すと、1・8・32候補で100回ウォームアップ後に500回測定します。結果はJSONで表示し、コピーできます。読みと文脈の準備も毎回のC API呼び出しに含みます。固定候補の繰り返しを使う測定なので、IME全体の変換精度・辞書検索・UI・電力・CPU利用率は含みません。低電力モード、熱状態、OS、モデルSHA-256も記録します。

低電力モードを切り替えた測定は別々に保存してください。モデルの常駐データは約162KiBですが、SwiftUIやOSを含むアプリ全体のメモリ量ではありません。推論はボタン操作で一度だけ実行し、バックグラウンドで繰り返しません。実機で90%精度・5msの達成を確認したという主張はありません。

ライセンス: アプリ、Rustコード、合成教材はAGPL-3.0-only。対応するソースは https://github.com/luqnexa/Luqnexa と submodule の https://github.com/luqnexa/Luqnexa-AI です。
