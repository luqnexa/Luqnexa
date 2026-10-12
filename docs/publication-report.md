# Luqnexa GitHub公開記録

公開日: 2026-10-11。既存の作業ディレクトリを保持し、公開用チェックアウトへ分割して初回の `main` を公開した。

| リポジトリ | 公開内容 | 初回コミット |
| --- | --- | --- |
| [Luqnexa](https://github.com/luqnexa/Luqnexa) | IME共通コア、変換、CLI、FFI、OSアダプター、テスト | `dc5a17d770b1c36188d9f417adee20bb1c306760` |
| [NEXC](https://github.com/luqnexa/NEXC) | 独自辞書圧縮、検索、追記保存、辞書型、テスト | `2ac1a18e5e98333a660cc53f9e3b2ef3e91f6119` |
| [Luqnexa-AI](https://github.com/luqnexa/Luqnexa-AI) | 文脈ランキング、大文字小文字候補、学習・ローカル推論連携 | `8818d1cdf174dd79a5919c8694bcc2d287ecd53f` |
| [Luqnexa-Server](https://github.com/luqnexa/Luqnexa-Server) | 青空文庫収集・再開管理スクリプト、テスト、開発計画 | `7d8a6780f0e4b83f2114abb683b74c10bb145514` |
| [.github](https://github.com/luqnexa/.github) | 組織プロフィール、貢献・脆弱性報告案内 | `06858f5f5fc9fee33612d409fae3cd534424b8de` |

全5件の匿名アクセス、公開設定、既定ブランチ `main` を確認した。ソースの基本ライセンスはAGPL-3.0-only。第三者データの元のライセンス・帰属表示も保持している。全5件でGitHubの非公開脆弱性報告を有効にした。

## 公開しなかったデータ

認証情報、秘密鍵、個人辞書、入力・会話履歴、端末設定、ログ、SQLite進捗データ、取得キャッシュ、未検証の学習成果物はアップロードしていない。これらを別の非公開リポジトリへ転送する処理も実施していない。

ダウンロード済みの大きな推論モデル・推論実行ファイルと生成済みの大規模Mozc TSVは除外し、取得・生成スクリプトとライセンス情報を残した。公開情報から作成した同梱の小規模統計モデルは、出典・ライセンス・帰属を保持し、学習レポートのSHA-256と一致することを確認した。公開コミットのファイル内容を認証トークン・秘密鍵パターン等で検査し、検出はなかった。

## 検証

分割後のRustワークスペースでfmt、警告をエラーにしたclippy、単体・統合テスト、releaseビルドを実施した。Python学習・収集テスト、Windowsネイティブテスト、ベンチマークの動作も確認した。

GitHub Actionsの初回実行は以下の全ジョブが成功した。

- [IME CI](https://github.com/luqnexa/Luqnexa/actions/runs/38145081826): Windows、Linux、macOS、Rust 1.85.0。C ABIはLinux/macOS、Windows TSFはWindowsジョブで検証。
- [NEXC CI](https://github.com/luqnexa/NEXC/actions/runs/38145052656): Windows、Linux、macOS、Rust 1.85.0。
- [AI CI](https://github.com/luqnexa/Luqnexa-AI/actions/runs/38145063493): Windows、Linux、macOS、Rust 1.85.0。
- [収集スクリプト CI](https://github.com/luqnexa/Luqnexa-Server/actions/runs/38145071796): Pythonテスト。

`.github` は文書専用でCIワークフローは設置していない。これらの成功は全OSの実機IME動作、1億語での性能、iPhone SEでの5ms達成やAIの一般的な精度を保証しない。

## 今後の変更場所

公開用の独立したGitチェックアウトは、この作業ディレクトリの `.cache/publication/repos-v2/` 内の各リポジトリにある。元の作業ディレクトリと自動同期する構成ではない。`.cache` は追跡対象外なので、継続開発では各公開リポジトリを別の永続ディレクトリへcloneして使うことを推奨する。

IMEは `modules/NEXC` と `modules/Luqnexa-AI`、Serverは `modules/Luqnexa-AI` を固定コミットのGit submoduleとして参照する。取得には `git clone --recurse-submodules` を使う。依存側を更新する場合は、その変更を公開・検証した後、利用側のsubmoduleコミットも更新する必要がある。

## 未実装と今回の範囲

ServerのRust/Axum API、認証、PostgreSQL、Redis Streams、R2、Ed25519署名更新は未実装。AIの自由な文章生成・会話理解全体も完成していない。各READMEに現状と未実装事項を記載した。

今回行ったのはソース公開であり、稼働中IMEへのインストール、管理者操作、モデル適用、実行中の青空文庫学習の変更は行っていない。
