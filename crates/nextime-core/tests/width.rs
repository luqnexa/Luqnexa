// SPDX-License-Identifier: AGPL-3.0-only
use nextime_core::{Engine, Key, Session};
#[test]
fn explicit_width_change_never_commits_or_destroys_raw_input() {
    let mut session = Session::new(Engine::bundled(), 8).unwrap();
    session.event(Key::Text("Python123!")).unwrap();
    assert!(session.event(Key::FullWidth).unwrap().committed.is_none());
    assert_eq!(session.display(), "Ｐｙｔｈｏｎ１２３！");
    assert_eq!(session.raw(), "Python123!");
    session.event(Key::HalfWidth).unwrap();
    assert_eq!(session.display(), "Python123!");
    assert_eq!(
        session.event(Key::Enter).unwrap().committed.as_deref(),
        Some("Python123!")
    );
    session.event(Key::Text("ガッツポーズ")).unwrap();
    session.event(Key::HalfWidth).unwrap();
    assert_eq!(session.display(), "ｶﾞｯﾂﾎﾟｰｽﾞ");
    session.event(Key::FullWidth).unwrap();
    assert_eq!(session.display(), "ガッツポーズ");
    session.event(Key::Cancel).unwrap();
    assert!(session.display().is_empty());
}
