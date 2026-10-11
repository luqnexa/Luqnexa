# Windows TSF prototype

共通処理はRust、Windows SDKとの接続はC++17です。Windows x64のデスクトップアプリ向けの初期実装で、32bit・ARM64・ストアアプリの動作は未検証です。Windowsへの実登録、登録済みプロファイルのプロセス内有効化・解除を確認しました。一般アプリによる入力確認は未実施です。

## ビルド・検証

Rust、Visual StudioのC++ Build Tools、Windows SDK、x64 PowerShellを使用します。リポジトリのルートで実行してください。

```powershell
./scripts/build-windows.ps1
./scripts/test-windows.ps1
./scripts/install-windows.ps1 -WhatIf
./scripts/install-windows.ps1 -Action Uninstall -WhatIf
```

成果物はCargoのtargetディレクトリ配下の`windows-x64/`です。`nextime_tsf.dll`、`nextime_ffi.dll`、`LICENSE`を生成します。アダプターは隣接するRust DLLだけを読み込みます。`dictionaries/mozc-ja.tsv`があれば、ビルド時に`base.nud`を生成し、出典・ライセンス表示も同梱します。インストーラーはこの辞書を新版ディレクトリへ配備します。辞書が無ければ639エントリの組み込み辞書を使います。NUDの索引は起動時、各ブロックは検索時に検証します。取得方法は [日本語辞書増量報告](../../docs/japanese-dictionary.md) を参照してください。

テストはCOMファクトリとRust DLLを直接ロードします。Windows実装の文書・context・composition・編集ロックに、メモリ内の`ITextStoreACP`を接続しています。機械登録を避けるため、キーディスパッチャーのsink登録だけをテスト用に代替し、イベントをサービスへ直接渡します。画面レイアウトは`TS_E_NOLAYOUT`を返すため、候補ウィンドウの描画・位置・アクセシビリティはこのテストでは検証しません。

## インストール・アンインストール

ビルド後、**管理者のx64 PowerShell**から実行します。実行中のアプリがDLLを保持する場合は終了してから削除してください。

```powershell
./scripts/install-windows.ps1
./scripts/install-windows.ps1 -Action Uninstall
```

通常のPowerShellからUACを使う場合は `./scripts/elevate-install-windows.ps1`（削除は `-Action Uninstall`）を使用できます。昇格した子プロセスにビルド成果物のパスを渡すため、管理者側のCargo設定やPATHには依存しません。実行ログはtargetの`windows-x64/elevated-install.log`等へ保存します。

登録後の検証は `./scripts/test-windows.ps1 -SkipBuild -Registered` です。配備済みDLLを読み込み、TSFのプロファイル列挙・有効状態、Windowsによるサービスの有効化、foreground key sink、プロセス内解除を確認します。`TF_IPPMF_FORPROCESS`を使い、ユーザーの既定入力プロファイルを変更するフラグは使用しません。このAPIの範囲は [Microsoft ActivateProfile仕様](https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfinputprocessorprofilemgr-activateprofile) に従います。入力操作の既存テストは引き続き直接イベントを渡すホストで実行し、通常のキーボード配送は検証しません。

配備先は`C:\Program Files\NextIME`です。COM CLSID、TSFの日本語プロファイルとキーボードカテゴリを登録します。Windowsの言語設定から`NextIME Japanese (prototype)`を明示的に選択します。既定のIMEを変更する処理はありません。既存の同名ディレクトリやCLSIDがある場合は置き換えません。

更新はビルド後に `./scripts/elevate-install-windows.ps1 -Action Update` を実行します。DLL・同梱辞書・ライセンス等のハッシュを含む別フォルダーへ配備し、旧登録を解除して新登録へ切り替えます。切り替え失敗時は旧登録への復帰を試みます。使用中の旧DLLは削除しません。入力先アプリを再起動して新版を読み込んでください。ユーザーが旧ディレクトリへ置いた独自辞書・設定の移行は未実装です。

アンインストールは登録を解除した後、レシートに記録したSHA-256と一致する現在版のDLL・同梱辞書・ライセンス・出典・レシートを削除します。ユーザーが置いた未記録の辞書、旧版DLL、以前のレシート、その他のファイルは残ります。失敗時はファイルを保持して診断できます。旧版の自動整理・修復・システム停止時まで含む完全なインストーラートランザクションは未実装です。

## 操作

日本語モードでは英字でローマ字入力、Spaceで変換/次候補、上下キーで予測・候補を選択、Shift+Spaceで前候補、Enterで確定、Escapeで取消、Backspaceで削除します。F10で元の英字列を選びEnterで確定できます。F12で日英モードを切り替え、英字モードでは大文字・小文字・数字・記号を保持し、Spaceで単語＋空白を確定します。JISレイアウトでは半角/全角キーも切替に割り当てています。英字モードはセッション内の設定で、アプリ再起動時は日本語になります。

数字・記号は`ToUnicodeEx`で現在のキーボードレイアウトとShift/Caps Lockに従って処理します。Ctrl/Alt/Windowsのショートカットは処理しません。デッドキー・AltGr等は未検証です。未確定入力がない場合のSpaceやEnterもアプリへ渡します。候補選択はキーボードのみです。

日本語モードでもShiftを押しながら入力した大文字はかなに直さず保持します。Shift+A、Shift+Iで`AI`、辞書にない`XYZ`も入力できます。Shiftを離して`ga`を続けると`XYZが`となります。Shiftなしの`ai`は「あい」です。Caps Lockによる大文字も共通コアで保持します。これは指定した大文字を保持する処理で、AIがユーザーのShift操作を代行したり「あい」を勝手に`AI`へ変更する機能ではありません。大文字でローマ字かな入力していた利用者にとっては動作変更です。

読み取り専用状態、キーボード無効/空contextのcompartment、パスワードのInputScopeをチェックして処理を避けます。InputScopeを提供しないアプリを検出できるとは保証しません。文書全文の読み取り・入力の永続保存・ネットワーク送信はありません。

## 設計と制約

`ITfTextInputProcessor`がsinkを登録し、`ITfKeyEventSink`から同期`ITfEditSession`を要求します。編集ロックを取得した後だけRust状態と文書を変更します。取得拒否時はキーを処理しません。Enter以外では確定せず、候補はcompositionを置換します。フォーカス喪失は非同期の取消を要求し、その待機中は新たなIME操作を止めます。描画には非アクティブなWin32 popupを使い、サービス終了時にwindowとclassを解除します。

未対応項目：言語バー/英数切替、マウス候補選択、TSF標準候補UI、スクリーンリーダー、部分確定、周辺文脈、個人学習、コード署名、32bit/ARM64配備、アプリ間互換性、編集途中のアプリ側エラーからの完全な復旧。フォーカス喪失時の非同期取消は実アプリで追加検証が必要です。

Windows仕様の参考： [Text Service Registration](https://learn.microsoft.com/en-us/windows/win32/tsf/text-service-registration)、[RequestEditSession](https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfcontext-requesteditsession)、[StartComposition](https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfcontextcomposition-startcomposition)。

AGPL-3.0-only。配布する場合はライセンスと著作権表示を保持し、対象となる対応ソース提供条件を満たしてください。署名済み配布リリースはまだ作成していません。
