# 学習モデルの変換全体での検証

候補比較に `--model` を渡すだけでは、Rustの変換エンジンの重みは変わりません。通常ビルドは `models/native-model.bin` を使います。CLIの `--model-info` は、実際にコンパイルしたモデル・アンカーのSHA-256と研究ビルドかどうかを返します。

採用判定済みの学習runを明示して、別のtargetディレクトリへ研究用CLIをビルドできます。

```powershell
$env:LUQNEXA_RESEARCH_MODEL_DIR = '学習runの絶対パス'
cargo build -p nextime-cli --release --features nextime-ai/research-model --target-dir C:/opencode-rust/target/research-ranker
```

ビルド時にモデル形式、モデル・訓練スナップショットのハッシュ、再現性確認、候補比較の退行判定、アンカーのサイズ・形式を確認し、成果物をビルドディレクトリに保存します。候補比較の判定通過は実際の変換精度や配布の承認ではありません。文脈がない場合、新しいアンカーへ補正範囲を広げず、従来の範囲だけを使います。

`evaluate-mixed.py --session-cli --split validation` で固定の調整用データを先に測定します。研究用CLIと報告対象モデルのハッシュが違えば評価を中止します。固定testは学習や個別解答への調整に使わず、変換全体の検証に残します。実際のWindows IMEへのインストール・モデル適用は行いません。

ジャーナルには排他ロックがあるため、同じジャーナルの比較実行は順番に行ってください。評価用のセッション学習は無効化し、文脈は合成された評価用文章だけを使います。

訓練専用の実誤候補の調査には `diagnose-runtime.py --lessons TRAIN.jsonl --cli CLI --nud BASE --journal EXPERIMENT --output REPORT` を使います。最大24文までとし、固定評価を混ぜた場合は停止します。最大32候補内に正答がないものは `missing_candidate`、正答が下位にあるものは `ranking_error` に分けます。出力された `proposed_pair` は要点検の候補で、学習への自動採用はしません。同じ入力の新しい誤候補を採用しても、新しい文章として多様性を水増ししないでください。

2026-10-13の活用辞書v2検証では、訓練24文の第一候補正解が13→14、候補不足が8→6となりましたが、固定test24文は8/24のままでした。未知文で90%超を達成した証拠はなく、研究辞書は稼働IMEへ適用していません。
