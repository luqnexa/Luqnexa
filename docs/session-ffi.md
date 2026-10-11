# Composition Session / C ABI v1

この文書はPhase 3の共通基盤を説明します。Windows TSF等への登録や他アプリの入力欄への挿入はまだ実装していません。

## 入力状態

`Session::new(engine, limit)` は入力欄1つの未確定状態を所有します。Engineは既存のメモリ辞書、NEXCのFileDictionary、LiteDictionaryを利用できます。Sessionはraw入力、ひらがなpreedit、候補、選択位置を保持します。

| イベント | 挙動 |
| --- | --- |
| Text | rawの末尾へ追加し再解析。候補を解除。入力に空文字・制御文字は禁止 |
| Backspace | raw末尾のUnicodeスカラー値を1つ削除し再解析。候補を解除 |
| Convert | 初回は候補生成。候補表示中は次候補へ |
| Next / Previous | 候補の前後移動。端では折り返す |
| Enter | 選択候補または確定解析した読みを返し、compositionを空にする |
| Cancel | 入力・候補をすべて破棄。確定文字列は返さない |

入力が空の編集キー、候補がないNext/Previousはhandled=falseです。ホストは元のキーをアプリへ通します。空のcompositionでのEnterはアプリのキーとして扱われ得るため、OSアダプター側でチャット送信等のアプリ動作を適切に扱う必要があります。Luqnexa自身は送信機能を持ちません。

`to_preedit` は末尾の未完ローマ字を保持します。例えば `n → に`（iを追加）、`ky → きゃ`（aを追加）。EnterとConvertでは既存の確定入力パーサーを使い、末尾nをんへ確定します。rawを毎回再解析するのでnnからnnaへの変化も扱えます。Backspaceは表示上の1文字でなくrawの1文字を戻します。文中カーソル・範囲選択・再変換・grapheme単位の編集は未対応です。

入力の上限はraw512 UTF-8バイトかつpreedit128 Unicodeスカラー値。候補は1〜32件です。Text・Convertでの検証や辞書検索に失敗した場合、以前のcompositionを保持します。Cancelはフォーカス移動時にもホストから呼んでください。途中の入力から別入力への自動確定や自動学習はありません。

## CLI操作デモ

```sh
cargo run --release -p nextime-cli -- session
```

標準入力へ次の各行を入力します。

```text
text h
text a
text s
text h
text i
space
next
enter
quit
```

候補選択後に `commit: 箸` が1回表示されます。CLIは実際のキーをフックせず、行単位のコマンドで状態遷移を実演します。`session --nud FILE.nud` と `--journal FILE.nxl`、既存の `--dictionary`、`--limit` も利用できます。EOF/quitで未確定入力は破棄されます。未知コマンドはエラーです。

## C ABI

ヘッダーは `crates/nextime-ffi/include/nextime.h`、実装は `crates/nextime-ffi/src/lib.rs` です。ABI versionは1。32/64bitのsize_t、uint32_t、uint64_t、int32_tとポインターだけを境界で使用します。RustのVec・String・enumのメモリ配置を公開しません。

```text
nextime_abi_version
nextime_session_create
nextime_session_event
nextime_session_candidates
nextime_session_text
nextime_session_destroy
```

createには明示長付きUTF-8パスと候補数を渡します。パス長0はサンプル辞書、非0は `.nud` を開きます。パス上限4096バイト、NULは禁止です。失敗時は出力handleを0にします。FFIでの `.nxl` 利用はまだ対応していません。

handleはプロセス内registryに登録されたu64で、Rustのメモリポインターではありません。0、未登録、破棄済みhandleは拒否します。成功したcreateごとにdestroyが必要です。破棄しなければセッションと辞書が保持されます。DLLのアンロード前にすべてを破棄してください。

eventはText(1)、Backspace(2)、Convert(3)、Next(4)、Previous(5)、Enter(6)、Cancel(7)。Text以外のtext_lengthは0です。出力flagsのbit0はhandled、bit1はcommitあり。確定文字列は次の成功eventまで保持します。kind5を取得するだけでは消費しません。HAS_COMMITが立ったイベントに対して一度だけ取得・挿入してください。

textのkindはraw(1)、preedit(2)、display(3)、candidate(4)、committed(5)、mode(6)。modeは`日本語`または`English`です。displayは明示的に選択した候補があればその表記、それ以外はpreeditです。予測表示中は候補が存在してもselectedがUINT32_MAXになります。candidateだけindexを使用します。出力はUTF-8のNUL終端で、requiredは終端を含むバイト数です。NULL/容量0でサイズを問い合わせるとBUFFER_TOO_SMALLが返ります。十分なバッファで再度取得します。不足したバッファは部分書き込みせず、そのまま返します。

追加イベントはLiteral(8)、ToggleMode(9)、Space(10)です。Literalはrawを候補として選択し、Enterでのみ確定します。ToggleModeは日本語/英字を切り替えます。Spaceは日本語ではConvert、英字では現在の単語＋空白を確定します。TextとBackspaceは予測を更新し、未選択状態に戻します。予測辞書の検索失敗は予測のみを空にして入力を継続します。通常変換の辞書エラーは従来通り返します。

| status | 意味 |
| ---: | --- |
| 0 | 成功 |
| 1 | 引数・index・候補数・入力文字が不正 |
| 2 | handleが無効 |
| 3 | UTF-8が不正 |
| 4 | 出力バッファ不足（サイズ問い合わせを含む） |
| 5 | 入力・パスの長さ上限超過 |
| 6 | 辞書のopen/検索に失敗 |
| 7 | panic/lock等の内部エラー |

OS側は文字列を必要に応じUTF-16等へ変換します。文字カーソル位置や属性範囲は現在ABIで公開しません。status6は細かいI/Oエラー文字列を公開しないため、診断拡張は今後必要です。

## 安全性とスレッド

handle registryとセッションはMutexで保護します。ただしホストは同一handleへのevent・読取・destroyを直列化してください。サイズ問い合わせと読取の間に他スレッドが状態を変更すると再問い合わせが必要です。destroyと実行中の操作を競合させた場合は、開始済み操作がArcで状態を保持するため、操作完了がdestroyより後になる可能性があります。ホスト側でこの競合を避けます。

非NULLのポインターが実際に有効かをRust側で検証することはできません。呼び出し側は指定長の入力を読み取り可能にし、出力を十分な領域に配置し、output同士・inputとoutputを重ねない責任があります。不正ポインター・不正な長さによるメモリ違反はcatch_unwindで救済できません。

Rustのunwind panicはFFI境界で捕捉します。panic=abortやOOM、OSシグナルまで回復できる保証はありません。panicしたセッションのMutexはpoisonされ得るため、内部エラー後はセッションを破棄し作り直します。

## ビルドとCでの検証

```sh
cargo build -p nextime-ffi --release
```

Windowsでは `nextime_ffi.dll` とimport library・static library、Linuxでは `.so` / `.a`、macOSでは `.dylib` / `.a` が対象です。実際の出力先は `cargo metadata` のtarget_directoryに従います。Rust targetとCプログラムのCPUアーキテクチャを一致させます。

Windows（Visual StudioのC++ Build Toolsが必要）:

```powershell
powershell -NoProfile -File scripts/test-ffi.ps1
```

Linux/macOS（ccとpython3が必要）:

```sh
sh scripts/test-ffi.sh
```

両スクリプトはC11デモをコンパイルし、動的ライブラリへリンクします。候補4件・選択・Enter・UTF-8確定・handle破棄を確認します。Windowsスクリプトは一時出力をtargetへ置き、MSVCで日本語ユーザーフォルダーのtempパスに起因する失敗を回避します。TEMP/TMPは終了時に戻します。

CIに全3OSでのCデモを追加しましたが、ローカル実測・動作確認はWindows x86_64のみです。Windows TSFプロトタイプの最新状態は [Windows手順](../platforms/windows/README.md) を参照してください。
