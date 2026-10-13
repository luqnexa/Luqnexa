// SPDX-License-Identifier: AGPL-3.0-only
use nextime_core::{Engine, Key, Session};

#[test]
fn reviewed_rewrites_preserve_faithful_reading_and_f9_choice() {
    let engine = Engine::bundled();
    for (input, faithful, suggestion) in [
        ("kikainiatta", "機会にあった", "機会に恵まれた"),
        (
            "kono mondaiwakaiketsusuru",
            "この問題は解決する",
            "この問題を解決する",
        ),
    ] {
        let candidates = engine
            .convert_with_context(input, "", 8)
            .unwrap()
            .candidates;
        assert!(candidates.iter().any(|c| c.surface == faithful));
        assert!(candidates.iter().any(|c| c.surface == suggestion));
    }
    let mut session = Session::new(Engine::bundled(), 8).unwrap();
    session.event(Key::ToggleAi).unwrap();
    session.event(Key::Text("kikainiatta")).unwrap();
    session.event(Key::Space).unwrap();
    assert_eq!(session.display(), "機会にあった");
    assert!(
        engine
            .convert_with_context(&format!("{}konitiha", " ".repeat(130)), "", 8)
            .is_err()
    );
    assert_eq!(engine.normalize_input("hello world", false), "hello world");
}

#[test]
fn user_sentence_suite_keeps_original_and_requires_explicit_commit() {
    for line in include_str!("../../../tests/sentence-suite.tsv")
        .lines()
        .filter(|s| !s.starts_with('#'))
    {
        let row: Vec<_> = line.split('\t').collect();
        let engine = Engine::bundled();
        let original = engine.normalize_input(row[0], false);
        let result = engine.convert_with_context(row[0], "", 8).unwrap();
        assert_eq!(
            result.candidates[0].surface, row[1],
            "{}: {:?}",
            row[0], result.candidates
        );
        assert!(
            result.candidates.iter().any(|c| c.surface == original),
            "literal lost: {}",
            row[0]
        );
        let mut session = Session::new(engine, 8).unwrap();
        for c in row[0].chars() {
            assert!(
                session
                    .event(Key::Text(&c.to_string()))
                    .unwrap()
                    .committed
                    .is_none()
            );
        }
        assert!(session.event(Key::Space).unwrap().committed.is_none());
        assert_eq!(session.display(), row[1]);
    }
}

#[test]
fn reported_sentences_convert_without_qwen_and_commit_only_on_enter() {
    for (reading, expected) in [
        ("あいをあいようしている", "AIを愛用している"),
        ("あいをたいせつにする", "愛を大切にする"),
        ("きかいをつかう", "機械を使う"),
        ("きかいにめぐまれた", "機会に恵まれた"),
        ("はしでごはんをたべる", "箸でご飯を食べる"),
        ("はしをわたる", "橋を渡る"),
        ("pythonでかくほうがいい", "Pythonで書くほうがいい"),
        ("きょうはくもりです", "今日は曇りです"),
    ] {
        let mut session = Session::new(Engine::bundled(), 8).unwrap();
        for c in reading.chars() {
            assert!(
                session
                    .event(Key::Text(&c.to_string()))
                    .unwrap()
                    .committed
                    .is_none()
            );
        }
        assert!(session.event(Key::Space).unwrap().committed.is_none());
        assert_eq!(session.display(), expected, "reading={reading}");
        assert_eq!(
            session.event(Key::Enter).unwrap().committed.as_deref(),
            Some(expected)
        );
    }
}

#[test]
fn keyboard_errors_keep_literal_and_preserve_recognized_words() {
    let engine = Engine::bundled();
    for input in ["pythpn", "pyhton", "pythonn"] {
        let result = engine
            .convert_with_context(input, "コードを書きます。", 8)
            .unwrap();
        assert!(
            result
                .candidates
                .iter()
                .any(|c| c.surface.eq_ignore_ascii_case("Python")),
            "{input}: {:?}",
            result.candidates
        );
        assert!(
            result
                .candidates
                .iter()
                .any(|c| c.surface == result.reading)
        );
    }
    for input in ["rust", "hello", "python", "nihongo", "PYTHON"] {
        let plain = engine.convert(input, 32).unwrap();
        let ai = engine.convert_with_context(input, "", 32).unwrap();
        assert!(
            ai.candidates
                .iter()
                .all(|c| plain.candidates.iter().any(|p| p.surface == c.surface)),
            "unrequested correction: {input}"
        );
    }
}

#[test]
fn trained_native_model_selects_intrinsic_sentence_meaning() {
    for (good, bad) in [
        ("箸でご飯を食べる", "橋出ご飯を食べる"),
        ("橋を渡る", "箸を渡る"),
        ("機械を使う", "機会を使う"),
        ("機会に恵まれた", "機械に恵まれた"),
    ] {
        let mut candidates = [bad, good]
            .map(|s| nextime_core::Candidate {
                surface: s.into(),
                score: 0,
                segments: 1,
            })
            .to_vec();
        nextime_ai::Model::bundled().rerank("", &mut candidates, 8);
        assert_eq!(candidates[0].surface, good);
    }
}

#[test]
fn romaji_symbols_emoji_and_korean_require_explicit_commit() {
    for line in include_str!("../../../tests/romaji-symbols.tsv")
        .lines()
        .filter(|s| !s.starts_with('#') && !s.is_empty())
    {
        let (input, expected) = line.split_once('\t').unwrap();
        let mut session = Session::new(Engine::bundled(), 8).unwrap();
        for c in input.chars() {
            assert!(
                session
                    .event(Key::Text(&c.to_string()))
                    .unwrap()
                    .committed
                    .is_none()
            );
        }
        assert!(session.event(Key::Space).unwrap().committed.is_none());
        assert_eq!(session.display(), expected, "romaji: {input}");
    }
}
