# RustモデルのiPhone実機検証アプリ

SwiftUIからRust製int8モデルをC ABIで呼び出す実験用の親アプリと、Rust共通変換を呼ぶキーボード拡張です。モデルの学習もRustで行います。ダウンロードした教材、ユーザー入力履歴、個人辞書を取り込みません。

再署名時は同梱のLuqnexaKeyboard.appexも保持してください。インストール後、「キーボード設定を開く」から設定へ移動し、「一般 → キーボード → キーボード → 新しいキーボードを追加」でLuqnexaを追加します。入力欄で🌐を押して選びます。フルアクセスは不要です。ローマ字QWERTY、Shift、日英切替、全角半角、候補タップ、取消、削除、確定に対応します。変換中の文字はキーボード内に表示し、候補タップまたは確定で入力先に挿入します。フリック入力と全言語フル辞書は未対応です。キーボードは既存の共通変換・小規模同梱辞書であり、新しいRustニューラルモデルはまだ測定専用です。

`.github/workflows/ios-probe.yml` はGitHubのmacOSランナーでRustのiPhone静的ライブラリとアプリを最終リンクし、`LuqnexaProbe-unsigned.ipa` を生成します。個人の証明書をGitHubへ置かず、署名前のIPAを成果物として保存します。iPhoneでの再署名・インストールは、ユーザーのAltStore Classic／SideStore環境で行います。

GitHub Actionsの「iPhone Rust probe IPA」が成功したrunを開き、`LuqnexaProbe-unsigned-iphone` artifactをダウンロードして展開します。中の`.ipa`をiPhoneへ渡し、AltStore／SideStoreのIPA取り込み機能で追加してください。インストール手順は利用中のツールの公式手順に従ってください。

- AltStore Classic: https://faq.altstore.io/altstore-classic/your-altstore
- SideStore: https://docs.sidestore.io/docs/faq

アプリの「端末内で速度を測る」を押すと、1・8・32候補で100回ウォームアップ後に500回測定します。結果はJSONで表示し、コピーできます。読みと文脈の準備も毎回のC API呼び出しに含みます。固定候補の繰り返しを使う測定なので、IME全体の変換精度・辞書検索・UI・電力・CPU利用率は含みません。低電力モード、熱状態、OS、モデルSHA-256も記録します。

低電力モードを切り替えた測定は別々に保存してください。モデルの常駐データは約162KiBですが、SwiftUIやOSを含むアプリ全体のメモリ量ではありません。推論はボタン操作で一度だけ実行し、バックグラウンドで繰り返しません。実機で90%精度・5msの達成を確認したという主張はありません。

ライセンス: アプリ、Rustコード、合成教材はAGPL-3.0-only。対応するソースは https://github.com/luqnexa/Luqnexa と submodule の https://github.com/luqnexa/Luqnexa-AI です。
