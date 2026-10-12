# 未学習50例の現状評価（2026-10-12）

インストール済み `build-c8cf49275a61362e` の独自AIをテストした。今回の50例は辞書・補正表・モデルの学習へ追加していない。参考表記はユーザー指定の正解ではなく、評価前に仮定した表記。厳密一致数を一般的な正答率と呼ばない。

## 結果

| 条件 | 参考表記との先頭一致 |
| --- | ---: |
| Luqnexa・入力原文 | 10/50 |
| Luqnexa・比較用の空白除去 | 10/50 |
| Meltype 1.2.1・比較用の空白除去 | 21/50 |

Luqnexaで参考表記が8候補内に存在したのは16/50。各入力を一文字ずつ渡してSpaceで候補を表示し、Enterは押さずキャンセルした。全例で自動確定なしを検査し、実行エラーなく処理できた。セッションの確定履歴は空で、直前の文章を与えていない。

#8の「あついお茶」、#38の「韓国語のあいさつ」は参考表記との表記差であり、一律に意味の誤りとは扱わない。#10の「洗濯の選択肢」、#14の「このAIは本物だ」も成立する解釈がある。#1・4・5・6・7・9・10・14・15・16は同音語の参考解釈を仮定しており、ユーザーによる期待表記の確認が必要。

## 時間と比較条件

開発PCの約128万語NUD、Text+Space、200回・warmup20：p50=43.9838ms、p95=76.0585ms、最大=136.5130ms。描画・Qwen・ネットワーク・辞書オープン・初回モデル読み込みを含まない。p95は5ms目標を超えた。空白除去条件のp95=43.0645ms。iPhone SE・1億語条件ではなく、並行処理負荷も隔離していない。

Meltypeは稼働中プロセスの独立した名前付きパイプセッションを使い、空白を除去した50入力を文字キーとSpaceで処理した。返された確定編集数は0。Enterや実アプリへの編集適用を行わず、Escape後にセッションを閉じた。既存の設定・学習状態は各製品で異なる。Meltypeの時間はIPCとUIスレッドを含むため、LuqnexaのFFI時間と相対速度比較しない。

## 観測された課題

- 同文内の役割に応じた同音語の選択が弱い。箸・端、機械・機会、AI・愛を誤る。
- 英語トークンやアクセント付き外国語をローマ字として解析してしまう。
- jを含む通常ローマ字や文中のOS/AIなどの扱いが崩れる例がある。
- 複数の入力ミスや語尾の欠落に対応できていない。
- 未知の長い読みを分割する探索が遅い。今回の時間だけでは原因別のCPU内訳は未計測。

前回の30/30は専用辞書・補正と、その例への学習を含む回帰結果だった。この50例はその改善が一般化できていないことを示す。今回は評価だけを行い、IMEの再インストール・モデル更新・学習への追加は行っていない。

## 全50例

参考表記は暫定値。候補一覧はJSONに保存している。

| # | 入力 | 仮の参考表記 | Luqnexa先頭候補 | 厳密一致 |
| --- | --- | --- | --- | --- |
| 1 | kishagakishadekishashita | 記者が汽車で帰社した | 記者が記者で帰社した | 不一致（参考値） |
| 2 | konohashihawataremasen | この橋は渡れません | この橋は渡れません | 一致 |
| 3 | hashidesaranohashiwotsutsuku | 箸で皿の端をつつく | 橋出皿の橋をつつく | 不一致（参考値） |
| 4 | koushounoyochigaaru | 交渉の余地がある | 交渉の余地がある | 一致 |
| 5 | koushounagijutsuwomanabu | 高尚な技術を学ぶ | 交渉な技術を学ぶ | 不一致（参考値） |
| 6 | seikakunaseikakuwoshiru | 正確な性格を知る | 正確な性格を知る | 一致 |
| 7 | kanjinokanjiwooboeru | 漢字の感じを覚える | 感jいのかんjいを覚える | 不一致（参考値） |
| 8 | atsuiochawoatsuihininomu | 熱いお茶を暑い日に飲む | あついお茶を暑い日に飲む | 不一致（参考値） |
| 9 | kaitouwokaitousuru | 解答を回答する | 会頭を回答する | 不一致（参考値） |
| 10 | sentakunosentakushiwofuyasu | 選択の選択肢を増やす | 洗濯の選択肢を増やす | 不一致（参考値） |
| 11 | karenoaihafukai | 彼の愛は深い | 枯野AIは深い | 不一致（参考値） |
| 12 | aigagazouwoseiseisuru | AIが画像を生成する | AIが画像を生成する | 一致 |
| 13 | aiwodounyuushitegyoumuwokaizen | AIを導入して業務を改善 | 愛を導入して業務を改善 | 不一致（参考値） |
| 14 | konoaihahonmonoda | この愛は本物だ | このAIは本物だ | 不一致（参考値） |
| 15 | kikaigakikaiwoseigyosuru | 機械が機械を制御する | 機会が機会を制御する | 不一致（参考値） |
| 16 | kikaigaarebakikaiwomitai | 機会があれば機械を見たい | 機会があれば機会をみたい | 不一致（参考値） |
| 17 | konomoderuhagakushuugahitsuyou | このモデルは学習が必要 | このモデルは学習が必要 | 一致 |
| 18 | deetawomotonikekkawoyosokusuru | データをもとに結果を予測する | でエタをもとに結果を予測する | 不一致（参考値） |
| 19 | shouraihajibundeOSwotsukuritai | 将来は自分でOSを作りたい | 将来はjいぶんでおsをつくりたい | 不一致（参考値） |
| 20 | atarashiiAIhakotobanoimiworikaisuru | 新しいAIは言葉の意味を理解する | 新しい愛は言葉の意味を理解する | 不一致（参考値） |
| 21 | rustde memorywokanrisuru | Rustでmemoryを管理する | Rustでメモryを管理する | 不一致（参考値） |
| 22 | githubni pullrequestwookuru | GitHubにpullrequestを送る | GitHubに府llれqうえstを送る | 不一致（参考値） |
| 23 | dockerde containerwokidousuru | Dockerでcontainerを起動する | dockerでc御大ねrを起動する | 不一致（参考値） |
| 24 | linuxno kernelwokoushinsuru | Linuxのkernelを更新する | linuxのけrねlを更新する | 不一致（参考値） |
| 25 | pythonde jsonwoyomikomu | Pythonでjsonを読み込む | Pythonでj村を読み込む | 不一致（参考値） |
| 26 | gpuno vramgatarinai | GPUのvramが足りない | gぷのvらm語りない | 不一致（参考値） |
| 27 | apino responsegakaettekonai | APIのresponseが返ってこない | api乗れsポンセがかえって小内 | 不一致（参考値） |
| 28 | typescriptde frontendwokaihatsusuru | TypeScriptでfrontendを開発する | typescriptでf論点dを開発する | 不一致（参考値） |
| 29 | sshde servernisetsuzokusuru | SSHでserverに接続する | っshで serverに接続する | 不一致（参考値） |
| 30 | aino inferencewokousokukasuru | AIのinferenceを高速化する | AIのいんフェれんcえを高速化する | 不一致（参考値） |
| 31 | kyouhabonjourtoaisatsushita | 今日はbonjourと挨拶した | 今日は盆城rと挨拶した | 不一致（参考値） |
| 32 | 한국어nohatsuonwobenkyousuru | 한국어の発音を勉強する | 한국어の発音を勉強する | 一致 |
| 33 | graciasnoimiwooshiete | graciasの意味を教えて | gらcイアsの意味を教えて | 不一致（参考値） |
| 34 | mañanahasupeingonotango | mañanaはスペイン語の単語 | まñアナはスペイン語の単語 | 不一致（参考値） |
| 35 | 中文tonihongowokirikaeru | 中文と日本語を切り替える | 中文と日本語を切り替える | 一致 |
| 36 | hellotokonnichihawotsukaiwakeru | helloとこんにちはを使い分ける | helloとこんにちはを使い分ける | 一致 |
| 37 | mercitoiwaretanodeoreiwokaeshita | merciと言われたのでお礼を返した | 目rcいといわれたのでお礼を変えした | 不一致（参考値） |
| 38 | 안녕하세요hakankokugonoaisatsu | 안녕하세요は韓国語の挨拶 | 안녕하세요は韓国語のあいさつ | 不一致（参考値） |
| 39 | cafédepuroguramingusuru | caféでプログラミングする | cあféでプログラミングする | 不一致（参考値） |
| 40 | open source nosofutowosagasu | open sourceのソフトを探す | おペンそうrcえのそ太を探す | 不一致（参考値） |
| 41 | konnbannha | こんばんは | こんばんは | 一致 |
| 42 | ohayougozaimau | おはようございます | おはようございまう | 不一致（参考値） |
| 43 | watasiha gakuseidesu | 私は学生です | ワタシハ学生です | 不一致（参考値） |
| 44 | kyouha tenkiga iides | 今日は天気がいいです | 教派天気がいいでs | 不一致（参考値） |
| 45 | pytohn de codewokaku | Pythonでcodeを書く | pyとhんでcお出を書く | 不一致（参考値） |
| 46 | aiwobenkyousiteimas | AIを勉強しています | 愛を勉強していまs | 不一致（参考値） |
| 47 | ashitaha kaigigaarimau | 明日は会議があります | 明日破壊ギガありまう | 不一致（参考値） |
| 48 | arigatougozaimashia | ありがとうございました | ありがとうございますございマシア | 不一致（参考値） |
| 49 | nihonngono henkanwotamesu | 日本語の変換を試す | 日本語の変換を試す | 一致 |
| 50 | rustde puroguramuwotukru | Rustでプログラムを作る | Rustでプログラムをつkる | 不一致（参考値） |

機械可読結果：`benchmarks/results/sentence-suite-50.json`（全候補）、`sentence-suite-50-compact.json`、`meltype-sentence-suite-50-verified.json`、`sentence-suite-50-metadata.json`。入力と仮の参考表記：`tests/sentence-suite-50.tsv`。

## MelType集計の訂正

初回の18/50はテスト側がパススルー文字を取り落としていた。#32・35・38を正しく含めた再測定は21/50。これら3件はMelTypeの誤変換ではない。[不一致29件の内訳](meltype-50-details.md)を参照。初回JSONは監査用に残し、訂正版を別ファイルへ保存した。
