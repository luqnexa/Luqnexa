# Phase 1 実装報告

この文書はPhase 1完了時点の記録です。最新のNEXC実装と検証結果は [Phase 2報告](phase2-report.md) を参照してください。

## 調査結果と作成ファイル

初期リポジトリは `.git` のみで、コミット・既存ソースはありませんでした。既存コードの変更・削除は行っていません。ローカル環境にはRustとCargoがあり、Windows上で実装と検証を行いました。

新規作成は合計28ファイルです。主要ファイル:

- ルート: `Cargo.toml`、`Cargo.lock`、`.gitignore`、`LICENSE`、`README.md`。
- 各crateの `Cargo.toml` とソース: parser / dict / ranker / convert / core / CLIの6crate。
- 結合テスト: `crates/nextime-core/tests/end_to_end.rs`、`crates/nextime-cli/tests/cli.rs`。単体テストは各ライブラリソース内。
- 辞書: `dictionaries/sample-ja.tsv`、`dictionaries/README.md`。
- ベンチマーク: `benchmarks/conversion.rs`、`benchmarks/README.md`、`benchmarks/results/2026-10-11-windows.md`、同名の `.log`（生ログ）。
- 設計・報告: `docs/design.md`、このファイル。
- CI: `.github/workflows/ci.yml`。

## 実装済み機能

コンパイル可能なRust共通コア、ローマ字からかな、基本日本語辞書検索、複数候補、基本ランキング、CLI、単体・結合テスト、ベンチマーク基盤、CI設定、README・設計書を実装しました。

変換例:

```text
input: hashi
reading: はし
1. 橋
2. 箸
3. 端
4. はし

input: nihongonyuuryoku
reading: にほんごにゅうりょく
1. 日本語入力
```

## 検証結果

ローカルのRust 1.98.1 / Windows x86_64で以下を実行しました。

| 検証 | 結果 |
| --- | --- |
| `cargo fmt --all -- --check` | 成功 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 成功、警告なし |
| `cargo test --workspace` | 成功、単体・結合16件とdoctest 1件 |
| `cargo +1.85.0 test --workspace --locked` | 成功、同じ17件で最低対応Rustを確認 |
| `cargo build --workspace --release` | 成功 |
| CLI `hashi` / `nihongonyuuryoku` / 外部TSV指定 | 正常出力を確認 |
| releaseベンチマーク、合成10万件 | 完走 |

CIはLinux・Windows・macOSとMSRVを対象に定義しています。GitHub上の実行結果と他OSの動作は未確認です。Rust 1.85はWindowsでローカル検証しました。

## ベンチマーク結果

最終コードのWindows / Ryzen 5 5600H、候補8、各2000回（長文200回）の単発測定で、同梱62件の通常反復変換p95は **0.0166ms**、合成10万件の異なる読み検索p95は **0.0658ms**、128文字入力p95は **0.4833ms** でした。詳細・他ワークロードは [実測記録](../benchmarks/results/2026-10-11-windows.md) に記載しています。

本来の基準端末・辞書規模ではないため、p95 5msという最重要目標は**達成判定をしていません**。既存IME比較も未実施です。

## 未実装・技術的課題

- NEXC圧縮、`.nud`、RAMに全展開しない検索、追記構造、差分辞書は未実装。現行の文字列Vecは1億語には不適切です。
- 辞書は自作62件のサンプルのみ。全言語の語彙や実用的な日本語辞書・頻度・活用・品詞接続モデルはありません。
- 候補生成はbeamによる近似探索です。高曖昧性で候補を取りこぼす可能性があり、ランキングの精度評価用コーパスが必要です。
- ローマ字入力の全方式、半角カナ・Unicode正規化、未確定入力管理、OSイベント処理は未対応です。
- 個人学習・辞書3層・暗号化・AI・Memory・Adaptive・新語収集・API・署名更新は未実装です。
- 全OSアダプター、候補UI、インストーラー、iOSのメモリ制約検証は未実装です。
- 電力、RSS、CPU負荷、cold cache、実機A13、熱状態、実語彙1億件の計測は未実施です。

## 次のPhase

次はPhase 2としてNEXCの可逆形式と検索APIを段階的に実装します。まず小規模辞書で復元の完全一致・破損データ検証・圧縮状態での検索をテストし、追記用Liteと構築用Maxを分離します。その後、代表的な多言語データのライセンスを確認して測定セットを用意し、語数を増やして容量・RAM・遅延を実測します。1億件・iPhone SE実機の測定ができるまでは性能目標を達成済みにしません。
