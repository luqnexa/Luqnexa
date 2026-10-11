# Phase 3 — 未確定入力とFFIの実装報告

これは共通基盤を作成した時点の記録です。その後のWindows TSF実装・最新の制約は [Windows追加報告](phase3-windows.md) を参照してください。

今回の範囲はPhase 3の最初の共通基盤です。実用OS IMEの完成ではありません。

## 実装

- rawからの逐次preedit生成、曖昧な末尾n・未完子音の保持。
- SessionでText・Backspace・Convert・候補移動・Enter・Cancelを処理。
- 候補表示と確定を分離し、Enter以外でcommitを発生させない設計。
- 入力上限・文字検証・辞書エラー時の状態維持、空入力時のキー通過。
- C ABI v1、整数handle検証、明示長付きUTF-8、呼び出し側所有バッファ、panic境界。
- サンプル辞書・`.nud`を開くFFI、DLL/static library出力。
- CLIの行コマンド式セッションデモ、C11呼び出しデモ、検証スクリプト、CI更新。
- Session・パーサー・ABI・CLIのテストと8イベントのベンチマーク。

通常入力の保存・送信は追加していません。文章の実際の挿入はホスト側が明示的な確定操作に応じて行います。送信機能はありません。

## 作成・変更ファイル

新規:

- `crates/nextime-core/src/session.rs`
- `crates/nextime-ffi/Cargo.toml`、`src/lib.rs`、`include/nextime.h`
- `crates/nextime-cli/src/session_demo.rs`
- `platforms/common/ffi-demo.c`
- `scripts/test-ffi.ps1`、`scripts/test-ffi.sh`
- `benchmarks/session.rs`、`benchmarks/results/2026-10-11-session-windows.log`
- `docs/session-ffi.md`、この報告

変更:

- `Cargo.toml / Cargo.lock`: FFI crateと依存関係。
- `crates/nextime-core/Cargo.toml / src/lib.rs`: Session公開・ベンチマーク登録。
- `crates/nextime-parser/src/lib.rs`: preedit解析とテスト。
- `crates/nextime-cli/src/main.rs / tests/cli.rs`: sessionコマンド・テスト。
- `.github/workflows/ci.yml`: Cデモとsessionベンチマーク。
- `README.md / benchmarks/README.md / docs/phase2-report.md`: 最新範囲と過去記録の明示。

既存の変換・NEXC機能と過去の測定結果を保っています。

## 検証

Windows x86_64 / Rust 1.98.1でfmt、clippy、workspace test、release buildが成功しています。テストは単体・結合44件とdoctest 1件の計45件です。C11デモをMSVCでコンパイルし、Rust DLLへの動的リンク・実行を確認しました。候補4件を取得し、次候補を選択して `commit: 箸` を返しました。

最低対応版Rust 1.85.0でも `cargo +1.85.0 test --workspace --locked` の45件が通過し、`cargo +1.85.0 check --workspace --all-targets --locked` でベンチマークを含めて確認しました。

MSVCのDLLリンクではimport library生成の日本語メッセージがRustから `linker stdout` 警告として表示されます。ビルドは成功しています。clippyの `-D warnings` は通過しました。

CI設定は更新しましたが、GitHub上の実行とLinux/macOSでの動作確認は未実施です。初期のC検証ではMinGWの日本語パス処理が失敗したため、WindowsのスクリプトはMSVCとtarget内のtempを使用しています。

## ベンチマーク

```sh
cargo bench -p nextime-core --bench session --locked
```

Windows 11 / Ryzen 5 5600H / Rust 1.98.1、同梱62エントリ、候補8、100回warmup後2000反復、release/thin LTOでの単一測定です。

測定は `h,a,s,h,i` のText5回、Convert、Next、Enterの8イベントをまとめたworkflowです。Sessionの解析・候補生成・選択・commit・状態クリア・返却値破棄・assertを含み、C ABI・UI・OS入力経路は含みません。

| p50 | p95 | p99 | 最大 |
| ---: | ---: | ---: | ---: |
| 0.0139ms | 0.0172ms | 0.0278ms | 0.1051ms |

生ログ: [session測定](../benchmarks/results/2026-10-11-session-windows.log)。電源・温度・OS負荷は固定していません。1億語・A13・低電力・発熱・電力・実際のOS候補UIは未検証です。これまでの5ms目標の達成を示す測定ではありません。

## 未実装と次の作業

Windows TSF COMサービス、Linux IBus/Fcitx5、macOS InputMethodKit、Android/iOSアダプター、ネイティブ候補UI、インストール・アンインストールは未実装です。現時点でOSのIME一覧には追加されません。

文中カーソル、属性範囲、再変換、選択範囲編集、フォーカス切替のOS連携、修飾キー・直接入力モード、文脈/学習、FFIからの個人追記辞書利用も未実装です。NEXCの1億語構築・暗号化・署名等は継続課題です。

次はWindows TSFの登録・キーイベント・compositionとcommitの接続、候補UI、インストール/アンインストールの実装を進めます。今回のC ABIはその接続点になります。仕様・メモリとスレッドの契約は [Session/FFI仕様](session-ffi.md) を参照してください。
