# Phase 3 — Windows TSF追加報告

Windows x64のTSFアダプター、候補popup、登録・削除スクリプトを実装しました。テストホストでは実際のTSF compositionを使った変換が動作します。インストール後の一般アプリでの互換性を確認した製品版ではありません。

## 今回のファイル

- `platforms/windows/src/common.h`: GUID、キー対応、隣接Rust DLLのロード、ABI v1呼び出し。
- `platforms/windows/src/service.cpp`: COMファクトリ、sink、有効化/無効化、編集ロック、composition、確定/取消。
- `platforms/windows/src/candidates.h`: 非アクティブWin32 popup、候補一覧と選択行、DPI/モニター位置処理。
- `platforms/windows/src/registration.cpp`、`guids.cpp`、`nextime.def`: COM/TSF登録、解除、Windows GUIDとDLL exports。
- `platforms/windows/tests/tsf.cpp`、`manager.h`: TSFテストホストとベンチマーク。レジストリは変更しません。
- `scripts/build-windows.ps1`、`test-windows.ps1`、`install-windows.ps1`: MSVC x64ビルド、検証、管理者登録・アンインストール。
- `scripts/elevate-install-windows.ps1`: UACで登録/削除を実行し、子プロセスの終了コードとログを取得。
- `platforms/windows/README.md`: 操作、ビルド、インストール方法、制約、Microsoft仕様リンク。
- `benchmarks/results/windows-tsf-2026-10-11.txt`: 実測記録。
- `.github/workflows/ci.yml`、`README.md`、`docs/phase3-report.md`、`benchmarks/README.md`、この報告: 最新状態とCI・測定手順。

## 検証結果

Windows 11 x64、MSVC、Rust 1.98.1で実行しました。

| 検証 | 結果 |
| --- | --- |
| `cargo fmt --all -- --check` | 成功 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 成功 |
| `cargo test --workspace --locked` | 45件成功（44件＋doctest 1件） |
| `cargo build --workspace --release --locked` | 成功 |
| MSVC `/W4 /WX`、TSF DLLビルド | 成功 |
| TSFホスト: composition、候補移動、Enter確定、Escape取消、Backspace | 成功 |
| 修飾キー対応、読み取り専用、編集ロック拒否、フォーカス取消、再有効化、COM寿命 | 成功 |
| インストール/アンインストール `-WhatIf` | 成功、実変更なし |
| UAC経由の実インストール、HKLM COM登録、TSF日本語プロファイル列挙 | 成功 |
| 配備済みDLL・LICENSEのSHA-256とビルド成果物の比較 | 一致 |
| 登録済みDLLをWindowsが有効化、foreground key sink接続、プロセス内解除 | 成功 |

RustのMSVCリンクではimport library生成の日本語メッセージが`linker stdout`警告として表示されますが、ビルドは成功しています。

ホストの文書・context・composition・ロックはWindows TSFです。キーディスパッチャーのsink登録だけはテスト用代替で、実際のOSによるキー配送やプロファイル登録を検証した結果ではありません。候補UIにはコードがありますが、テストホストはレイアウト未提供のため描画を行いません。InputScopeによるパスワード拒否とcompartment拒否も実アプリで追加検証が必要です。

ユーザーの管理者許可後、UACで昇格した子プロセスから実インストールを行いました。配備先は`C:\Program Files\NextIME`、HKLMのCOM InprocServer32とApartment登録を確認しました。TSF APIで日本語プロファイルの存在・有効状態を確認し、`-Registered`テストでWindowsによるプロセス限定の有効化、foreground key sink接続、プロセス内解除に成功しました。インストールは保持しています。実アンインストール・更新/修復・一般アプリでの入力・描画は未検証です。GitHub上のCI実行結果もまだありません。

手動で生成したclient IDによるサービス有効化や、フォーカスを持たないホストへの通常キー配送は成功しませんでした。登録済みサービスの確認ではWindowsのプロファイル有効化APIを使い、キー配送テストと分離しました。一般アプリ互換性が検証済みとは扱いません。

## 実測

同梱62語、100回warmup後1000回、8キー`H A S H I Space Down Enter`全体の計測です。Rust ABI、辞書検索・ランキング、TSF編集ロックとcomposition更新を含み、popup描画・通常のキーディスパッチ・OS登録を除きます。

| p50 | p95 | 最大 |
| ---: | ---: | ---: |
| 0.1125ms | 0.2017ms | 0.7543ms |

[測定条件と記録](../benchmarks/results/windows-tsf-2026-10-11.txt)。電力・温度・バックグラウンド負荷は固定していません。1億語・iPhone SE/A13・通常変換p95 5msの目標達成を示す測定ではありません。競合IMEとの比較も実施していません。

## 残る課題と次の作業

Phase 3は継続中です。次はWindows言語設定への表示を確認し、メモ帳・ブラウザー・エディターでキー配送、候補位置、フォーカス切替、パスワード欄、読み取り専用欄と実アンインストールを検証します。現在は64bitデスクトップ向けで、32bit・ARM64・ストアアプリは未検証です。

言語バーと英数切替、TSF標準候補UI・アクセシビリティ、マウス操作、再変換、部分確定、文中カーソル、コード署名、インストーラー更新/修復、アプリ側編集エラーの完全な復旧が残っています。Linux/macOS/Android/iOSは未実装です。Phase 4の署名付き辞書クラウドとPhase 5のAI/Memoryへ進む前に、実用IMEとしての互換性検証を追加する必要があります。
