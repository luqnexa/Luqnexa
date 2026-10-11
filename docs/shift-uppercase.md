# Shiftで指定した大文字を保持

2026-10-11、WindowsでShiftとアルファベットを同時に押したとき、日本語モードでも大文字として表示・確定するよう修正しました。

Windowsアダプターは既存の `ToUnicodeEx` によって現在のレイアウトとShift/Caps Lockの状態から文字を得ています。その後の共通コアが辞書未登録の大文字をかなへ戻していたため、共通コアの入力解析で連続する先頭大文字を保持するよう変更しました。混在文字列内の英字のまとまりについても同じ処理を行います。未知の全大文字を変換候補生成でも1まとまりとして保持するため、Spaceで変換しても勝手にかなへ戻りません。

| 入力 | 日本語モードの表示・確定 |
| --- | --- |
| Shift+A | A |
| Shift+A、Shift+I | AI |
| Shift+X、Shift+Y、Shift+Z | XYZ |
| XYZの後にShiftを離してga | XYZが |
| AIの後に日本語の「による」 | AIによる |
| Shiftなしでai | あい |

AIオン・オフ、逐次入力、Space変換、Enter確定、Backspace、取消後の日本語入力をRustテストで確認しました。WindowsのネイティブテストではA〜ZすべてのShiftキーマッピングと、キーマッピング→RustSession→表示・確定の経路を確認しました。実際の物理キーボードによるShift/Caps Lock配送のテストや、全キーボードレイアウトの確認はまだです。

Rustテスト65件、fmt、clippy、releaseビルド、Rust 1.85のcore/convertテスト、実辞書のAI・混在入力・日本語の回帰テスト、Windows TSFホストテストが成功しました。128万エントリのNUDを配置したWindows x64 TSFホストの8イベント測定は、ウォームアップ100回・1000回でp50 0.9663ms、p95 1.3230ms、最大2.2097ms。UI描画・実アプリへの配送・A13・1億エントリ・電力・熱は含みません。

変更箇所は `crates/nextime-core/src/lib.rs`、`crates/nextime-core/src/session.rs`、`crates/nextime-convert/src/lib.rs`、`platforms/windows/src/common.h`、`platforms/windows/tests/tsf.cpp` です。新DLLは `C:/opencode-rust/target/windows-x64` に生成済みですが、インストールは未適用です。AIが大文字を自動判断したりShift操作を代行する機能は追加していません。前の要望どおり「あい」を無条件にAIへ変換することはしません。
