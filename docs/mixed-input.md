# 英単語と日本語の混在入力

2026-10-11。「sex」が「せx」になる報告に対応しました。単語が未登録だったことと、日本語を続けた際に未確定入力全体をローマ字として再解析していたことが原因です。

`english.tsv`にsex、sexy、sexual、sexuality、gender、adult、porn、porno、erotic、dating、relationship、pregnancy、contraceptionの13エントリを手作業で追加しました。内容を理由に入力語を除外する処理はありません。組み込み639エントリ、NUD辞書1,285,626エントリです。

英字と日本語の文字種境界で解析し、登録済み英単語は原文を保持します。3文字以上の組み込み英単語の直後にローマ字の助詞ga/wo/ha/ni/de/to/moがある場合も区切ります。セッション表示、Enter確定、Space変換、予測用の読み生成に同じ処理を使います。大文字を含む英語の区間は変換探索でも原文を維持します。

| 入力 | 表示・変換・確定 |
| --- | --- |
| sex | sex |
| SEX | SEX |
| sexが / sexga | sexが |
| SEXga | SEXが |
| sexyga | sexyが |
| rustde | rustで |

英単語の入力途中（例えばse）はまだ「せ」と表示される場合があります。xまで入力すればsexへ再解析します。未知の英単語は自動認識せず、F10で原文を選ぶかF12で英字モードへ切り替えられます。日本語と英語で同じ綴りになる語の曖昧性、誤字の文脈修正は未解決です。スクリーンショットの「世紀／正規」のような同音語を意味から選ぶモデルは今回追加していません。

変更箇所は辞書・出典JSON、`nextime-core`の解析とセッション、`nextime-convert`の英語経路、Windowsホスト試験と実辞書回帰スクリプト、READMEです。手入力の語彙を加えてNUDを再構築しました。取得元・ライセンス表示を保持しています。入力内容を送信・学習・保存する処理は追加していません。

fmt、clippy（`-D warnings`）、Rustテスト52件（doctest1件含む）、workspace releaseビルド、MSVC `/W4 /WX`、実NUD辞書での混在・前回の候補品質回帰試験が成功しました。WindowsホストでS/E/X/G/A、Space、Enterを送り、sexがの未確定表示・変換・確定を検証しました。既存のローマ字、日本語変換、n'na、nあ、Backspaceも確認しています。CIは同じ試験を実行する設定ですが、GitHub上の実行は未実施です。

`C:\Program Files\NextIME\build-85ee62da4f57b831`に配備し、旧版を保持して登録を更新しました。登録済みサービスの有効化・解除、配備6ファイルのSHA-256一致を確認しました。NUDは47,526,644バイト、10,044ブロックです。

登録済みDLLのホスト実測は100回warmup、1000反復、H/A/S/H/I/Space/Down/Enterの8イベント全体でp50 0.8459ms、p95 1.1226ms、最大1.9081ms。sex自体の速度測定ではなく、UI描画・通常のアプリへのキー配送も除外しています。端末の熱・電力条件は固定せず、iPhone SE・1億エントリ目標は未検証です。

入力先アプリを終了・再起動すると新版を読み込めます。再検証は `scripts/test-japanese-candidates.ps1 -DictionaryPath 'C:/Program Files/NextIME/build-85ee62da4f57b831/base.nud'` と `scripts/test-windows.ps1 -SkipBuild -Registered` です。
