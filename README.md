# Luqnexa

「最小の容量で、最大の語彙を、最速で、最高精度に。」

旧名NextIME。Rustの日本語IME共通コア、ローマ字入力、辞書検索、複数候補、基本・文脈ランキング、CLI、C ABI、Windows x64 TSFプロトタイプです。入力中の文章を自動送信しません。

現在の実装: 日本語と英単語の混在入力、手動確定、Shiftの大文字入力、固有名詞の補助辞書、端末内AIのケース選択・文脈順位づけ、NEXC圧縮辞書の検索。内部分の `nextime-*`・コマンド・DLL名は互換性のため保持します。

未実装または未検証: 全言語辞書、1億エントリでの実機性能、AI自由文章生成、辞書配信API、Windows以外のOSアダプター。Windows版は試作段階です。

## ビルド

Rust 1.85以上。ソースを固定コミットのサブモジュールと共に取得します。

```sh
git clone --recurse-submodules https://github.com/Luqnexa/Luqnexa.git
cd Luqnexa
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
cargo run --release -p nextime-cli -- hashi
```

`hashi` は `橋 / 箸 / 端 / はし` の候補を返します。端末内AIは別途モデルを起動した場合だけ利用します。任意設定・モデル取得: `python modules/Luqnexa-AI/scripts/setup-local-ai.py` と `./modules/Luqnexa-AI/scripts/start-local-ai.ps1`。Windowsネイティブアダプターは `./scripts/build-windows.ps1` と `./scripts/test-windows.ps1` で確認します。インストールは [platforms/windows/README.md](platforms/windows/README.md) を参照してください。

大規模辞書は生成物を同梱せず、`python scripts/import-mozc.py` で固定版を取得できます。詳細と第三者条件は `dictionaries/MOZC-NOTICES.txt`、`mozc-source.json`、`docs/japanese-dictionary.md` に保持します。

## 構成

- [NEXC](https://github.com/Luqnexa/NEXC): 独自符号化、圧縮辞書構築、ディスク検索、追記辞書、共通辞書型。
- [Luqnexa-AI](https://github.com/Luqnexa/Luqnexa-AI): ローカル文章理解、ケース選択、文字統計学習、共通ランキング型。
- [Luqnexa-Server](https://github.com/Luqnexa/Luqnexa-Server): 公開文章収集の初期スクリプトと未実装のサーバー計画。

個人データ・会話ログ・認証情報・取得キャッシュは公開対象に含めません。個人学習データをサーバーへ送信しません。ベンチマークは `benchmarks/`、技術資料は `docs/` にあります。資料中の過去のローカル登録・測定記録は製品全般の精度や性能を保証しません。

## ライセンス

本体ソースはGNU AGPL-3.0-only（[LICENSE](LICENSE)）。対象となる改造版の配布・ネットワーク利用ではAGPLの条件に従って対応ソースを提供してください。第三者辞書・モデル・ライブラリのライセンス表示を保持し、すべてをAGPLへ変更しません。


## 2026-10-13 更新

候補変換・混在入力の回帰テストを更新しました。辞書配信と暗号化Memoryの試作は [Luqnexa-Server](https://github.com/Luqnexa/Luqnexa-Server) を参照してください。Windowsは試作段階で、今回のソース公開は稼働中IMEの更新ではありません。
