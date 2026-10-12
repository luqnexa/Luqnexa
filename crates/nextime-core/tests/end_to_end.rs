// SPDX-License-Identifier: AGPL-3.0-only
use nextime_core::{ConvertError, Dictionary, Engine};

#[test]
fn love_and_ai_remain_available_with_contextual_choices() {
    let engine = Engine::bundled();
    let result = engine.convert("あいをつかめ", 24).unwrap();
    assert_eq!(result.candidates[0].surface, "愛を掴め");
    assert!(result.candidates.iter().any(|c| c.surface == "AIを掴め"));
    for (context, expected) in [
        ("恋人への愛情について話しています。", "愛を確かめたい"),
        ("人工知能の動作について話しています。", "AIを確かめたい"),
    ] {
        let result = engine
            .convert_with_context("あいをたしかめたい", context, 24)
            .unwrap();
        assert_eq!(result.candidates[0].surface, expected, "{context}");
        assert!(
            result
                .candidates
                .iter()
                .any(|c| c.surface == "愛を確かめたい")
        );
        assert!(
            result
                .candidates
                .iter()
                .any(|c| c.surface == "AIを確かめたい")
        );
    }
    let result = engine.convert("あいをたしかめたい", 24).unwrap();
    assert!(
        result
            .candidates
            .iter()
            .any(|c| c.surface == "愛を確かめたい")
    );
    assert!(
        result
            .candidates
            .iter()
            .any(|c| c.surface == "AIを確かめたい")
    );
}

#[test]
fn desire_phrases_keep_their_conjugation_and_only_commit_explicitly() {
    use nextime_core::{Key, Session};
    for (input, expected) in [
        ("あなるセックスしたい", "アナルセックスしたい"),
        ("anarusekkusushitai", "アナルセックスしたい"),
        ("べんきょうしたい", "勉強したい"),
        ("けんさくしたいです", "検索したいです"),
        ("せっくすしたくない", "セックスしたくない"),
    ] {
        for ai in [true, false] {
            let mut session = Session::new(Engine::bundled(), 24).unwrap();
            if !ai {
                session.event(Key::ToggleAi).unwrap();
            }
            for ch in input.chars() {
                assert!(
                    session
                        .event(Key::Text(&ch.to_string()))
                        .unwrap()
                        .committed
                        .is_none()
                );
            }
            assert!(session.event(Key::Space).unwrap().committed.is_none());
            assert_eq!(session.display(), expected, "input={input}, ai={ai}");
            assert!(
                session
                    .candidates()
                    .iter()
                    .all(|c| !c.surface.contains("した位")
                        && !c.surface.contains("した医")
                        && !c.surface.contains("した胃"))
            );
            assert_eq!(
                session.event(Key::Enter).unwrap().committed.as_deref(),
                Some(expected)
            );
        }
    }
}

#[test]
fn local_ai_configuration_refreshes_after_session_creation() {
    use nextime_core::{Key, Session};
    const CHILD: &str = "LUQNEXA_CONFIG_REFRESH_TEST";
    if std::env::var_os(CHILD).is_some() {
        let directory =
            std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Luqnexa");
        let engine = Engine::bundled();
        assert_eq!(
            engine.convert("あいをたしかめたい", 8).unwrap().candidates[0].surface,
            "AIを確かめたい"
        );
        let mut session = Session::new(engine, 8).unwrap();
        assert!(!session.local_ai_enabled());
        assert_eq!(session.local_ai_status(), "local:config-missing");
        std::fs::create_dir(&directory).unwrap();
        let config = directory.join("local-ai.json");
        let enabled = format!(r#"{{"enabled":true,"token":"{}"}}"#, "0".repeat(64));
        std::fs::write(&config, &enabled).unwrap();
        session.event(Key::Text("あいをたしかめたい")).unwrap();
        assert!(
            session.local_ai_enabled(),
            "refresh before conversion UI is shown"
        );
        session.event(Key::Space).unwrap();
        assert!(session.local_ai_enabled());
        assert_eq!(session.local_ai_status(), "local:on");
        assert!(
            session
                .candidates()
                .iter()
                .any(|c| c.surface == "AIを確かめたい")
        );
        assert!(
            session
                .candidates()
                .iter()
                .any(|c| c.surface == "愛を確かめたい")
        );
        session.event(Key::Cancel).unwrap();
        std::fs::write(&config, r#"{"enabled":false}"#).unwrap();
        session.event(Key::Text("あいをたしかめたい")).unwrap();
        session.event(Key::Space).unwrap();
        assert!(!session.local_ai_enabled());
        assert_eq!(session.local_ai_status(), "local:config-disabled");
        session.event(Key::Cancel).unwrap();
        std::fs::write(&config, "broken JSON").unwrap();
        session.event(Key::Text("a")).unwrap();
        assert_eq!(session.local_ai_status(), "local:config-invalid");
        session.event(Key::Cancel).unwrap();
        std::fs::write(&config, r#"{"enabled":true,"token":"short"}"#).unwrap();
        session.event(Key::Text("a")).unwrap();
        assert_eq!(session.local_ai_status(), "local:token-invalid");
        session.event(Key::Cancel).unwrap();
        std::fs::write(&config, &enabled).unwrap();
        session.event(Key::ToggleAi).unwrap();
        session.event(Key::Text("あいをたしかめたい")).unwrap();
        session.event(Key::Space).unwrap();
        assert!(
            !session.local_ai_enabled(),
            "user's F9 choice must remain disabled"
        );
        assert_eq!(session.local_ai_status(), "local:off");
        std::fs::remove_file(config).unwrap();
        std::fs::remove_dir(directory).unwrap();
        return;
    }
    // Separate process: never mutate this process's real user configuration.
    let directory = std::env::temp_dir().join(format!(
        "luqnexa-config-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "local_ai_configuration_refreshes_after_session_creation",
        ])
        .env(CHILD, "1")
        .env("LUQNEXA_AI_BACKEND", "qwen")
        .env("LOCALAPPDATA", &directory)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::remove_dir(directory).unwrap();
}

#[test]
#[ignore = "Requires a running, explicitly configured local Qwen backend"]
fn real_local_llm_changes_contextual_choice_without_commit_or_stale_selection() {
    use nextime_core::{Key, Session};
    assert!(nextime_ai::LocalLlm::configured().is_some());
    let dictionary =
        Dictionary::from_tsv("はし\t橋\tnoun\t10000\nはし\t箸\tnoun\t9000\nはし\t端\tnoun\t8000\n")
            .unwrap();
    let mut session = Session::new(Engine::new(dictionary), 8).unwrap();
    session
        .event(Key::Text("テーブルの一番外側です。"))
        .unwrap();
    session.event(Key::Enter).unwrap();
    session.event(Key::Text("hashi")).unwrap();
    session.event(Key::Space).unwrap();
    assert_eq!(
        session.display(),
        "橋",
        "Lightweight baseline should differ from the LLM choice"
    );
    let started = std::time::Instant::now();
    while session.display() != "端" && started.elapsed().as_secs() < 20 {
        assert!(session.event(Key::PollAi).unwrap().committed.is_none());
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert_eq!(session.display(), "端");
    session.event(Key::Cancel).unwrap();
    session.event(Key::Text("hashi")).unwrap();
    session.event(Key::Space).unwrap();
    session.event(Key::PollAi).unwrap();
    session.event(Key::Next).unwrap();
    let explicit = session.display().to_owned();
    for _ in 0..20 {
        let outcome = session.event(Key::PollAi).unwrap();
        assert!(!outcome.handled && outcome.committed.is_none());
        assert_eq!(session.display(), explicit);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    session.clear_context();
    assert!(!session.event(Key::PollAi).unwrap().handled);
}

#[test]
#[ignore = "Requires a running, explicitly configured local Qwen backend"]
fn real_local_llm_preserves_english_name_while_ranking_japanese() {
    use nextime_core::{Key, Session};
    assert!(nextime_ai::LocalLlm::configured().is_some());
    let dictionary = Dictionary::from_tsv(
        "ChatGPTでかく\tChatGPTで書く\tphrase\t10000\nChatGPTでかく\tChatGPTで描く\tphrase\t9000\n",
    )
    .unwrap();
    let mut session = Session::new(Engine::new(dictionary), 8).unwrap();
    session
        .event(Key::Text("文章ではなく、猫の絵を作る話です。"))
        .unwrap();
    session.event(Key::Enter).unwrap();
    for ch in "ChatGPTでかく".chars() {
        assert!(
            session
                .event(Key::Text(&ch.to_string()))
                .unwrap()
                .committed
                .is_none()
        );
    }
    session.event(Key::Space).unwrap();
    let started = std::time::Instant::now();
    while session.local_ai_pending() && started.elapsed().as_secs() < 30 {
        assert!(session.event(Key::PollAi).unwrap().committed.is_none());
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert_eq!(session.display(), "ChatGPTで描く");
    assert_eq!(
        session.event(Key::Enter).unwrap().committed.as_deref(),
        Some("ChatGPTで描く")
    );
}

#[test]
#[ignore = "Requires a running, explicitly configured local Qwen backend"]
fn real_local_llm_selects_casing_without_automatic_commit() {
    use nextime_core::{Key, Session};
    assert!(nextime_ai::LocalLlm::configured().is_some());
    for (context, reading, expected) in [
        (
            "人工知能のチャットサービスを使います。",
            "chatgpt",
            "ChatGPT",
        ),
        ("コードを書くエディターの話です。", "vscode", "VSCode"),
        ("ソースコードを共有するサービスです。", "github", "GitHub"),
        (
            "メモリ安全性を重視するプログラミング言語です。",
            "rust",
            "Rust",
        ),
        (
            "The exact variable name is chatgpt. Preserve its lowercase spelling.",
            "chatgpt",
            "chatgpt",
        ),
    ] {
        let mut session = Session::new(Engine::bundled(), 8).unwrap();
        if context.is_ascii() {
            session.event(Key::ToggleMode).unwrap();
        }
        session.event(Key::Text(context)).unwrap();
        session.event(Key::Enter).unwrap();
        if context.is_ascii() {
            session.event(Key::ToggleMode).unwrap();
        }
        for ch in reading.chars() {
            assert!(
                session
                    .event(Key::Text(&ch.to_string()))
                    .unwrap()
                    .committed
                    .is_none()
            );
        }
        assert_eq!(session.display(), reading);
        assert!(session.event(Key::Space).unwrap().committed.is_none());
        assert!(session.candidates().iter().any(|c| c.surface == reading));
        let started = std::time::Instant::now();
        while session.local_ai_pending() && started.elapsed().as_secs() < 30 {
            assert!(session.event(Key::PollAi).unwrap().committed.is_none());
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert_eq!(session.display(), expected, "{context}");
        assert_eq!(
            session.event(Key::Enter).unwrap().committed.as_deref(),
            Some(expected)
        );
    }
}

#[test]
fn proper_names_and_ai_phrases_survive_character_input_and_ai_ranking() {
    let cases = [
        ("chatgpt", "chatgpt"),
        ("ChatGPT", "ChatGPT"),
        ("ChatGPTde", "ChatGPTで"),
        ("chatgptniyoru", "chatgptによる"),
        ("ちゃtgpt", "ChatGPT"),
        ("ちゃっとGPT", "ChatGPT"),
        ("ちゃっとじーぴーてぃー", "ChatGPT"),
        ("GitHubwo", "GitHubを"),
        ("OpenAIga", "OpenAIが"),
        ("YouTubede", "YouTubeで"),
        ("TypeScript", "TypeScript"),
        ("VSCode", "VSCode"),
        ("あおぞらぶんこ", "青空文庫"),
        ("なつめそうせき", "夏目漱石"),
        ("とうきょう", "東京"),
        ("にんてんどう", "任天堂"),
        ("あいによる", "AIによる"),
        ("ainiyoru", "AIによる"),
        ("あいによって", "AIによって"),
        ("あい", "AI"),
        ("ai", "AI"),
        ("あいが", "AIが"),
        ("あいを", "AIを"),
        ("aiga", "AIが"),
        ("えーあい", "AI"),
        ("あいをつかい", "AIを使い"),
        ("aiwotsukai", "AIを使い"),
        ("あいをつかって", "AIを使って"),
        ("あいをつかいたい", "AIを使いたい"),
        ("あいをつかいます", "AIを使います"),
        ("あいを使い", "AIを使い"),
        ("あいをりよう", "AIを利用"),
    ];
    for ai in [false, true] {
        for (raw, expected) in cases {
            // New entries must work without rebuilding an old compressed dictionary.
            let old = Dictionary::from_tsv(
                "で\tで\tparticle\t9000\nを\tを\tparticle\t9000\nが\tが\tparticle\t9000\n",
            )
            .unwrap();
            let mut session = nextime_core::Session::new(
                Engine::new(nextime_core::WithCommonPredictions::new(old)),
                8,
            )
            .unwrap();
            if !ai {
                session.event(nextime_core::Key::ToggleAi).unwrap();
            }
            for ch in raw.chars() {
                assert!(
                    session
                        .event(nextime_core::Key::Text(&ch.to_string()))
                        .unwrap()
                        .committed
                        .is_none()
                );
            }
            assert!(
                session
                    .event(nextime_core::Key::Space)
                    .unwrap()
                    .committed
                    .is_none()
            );
            assert_eq!(session.display(), expected, "raw={raw}, ai={ai}");
            assert_eq!(
                session
                    .event(nextime_core::Key::Enter)
                    .unwrap()
                    .committed
                    .as_deref(),
                Some(expected)
            );
        }
    }
    let engine = Engine::bundled();
    assert_eq!(engine.normalize_input("ai", false), "あい");
    assert!(
        engine
            .predict("ai", 8, false)
            .unwrap()
            .iter()
            .all(|c| c.surface != "AIによる")
    );
    for input in ["nihongo", "arigatou", "kyou"] {
        assert!(!engine.is_english_word(input));
    }
    for input in ["あいをつたえる", "あいする", "あいをこめて"] {
        assert!(
            engine
                .convert(input, 8)
                .unwrap()
                .candidates
                .iter()
                .all(|c| !c.surface.contains("AI")),
            "{input}"
        );
        assert!(
            engine
                .predict(input, 8, false)
                .unwrap()
                .iter()
                .all(|c| !c.surface.contains("AI")),
            "{input}"
        );
    }
}

#[test]
fn new_english_vocabulary_works_with_old_dictionary_and_japanese_particles() {
    let old = Dictionary::from_tsv("で\tで\tparticle\t9000\nを\tを\tparticle\t9000\n").unwrap();
    let engine = Engine::new(nextime_core::WithCommonPredictions::new(old));
    for (raw, expected) in [
        ("vscode", "vscode"),
        ("VSCode", "VSCode"),
        ("vscodede", "vscodeで"),
        ("vscodeを", "vscodeを"),
        ("luqnexa", "luqnexa"),
    ] {
        assert_eq!(engine.normalize_input(raw, false), expected);
        assert_eq!(
            engine.convert(raw, 8).unwrap().candidates[0].surface,
            expected
        );
    }
}

#[test]
#[ignore = "Explicit million-case audit; run in release mode with --ignored"]
fn million_normalization_checks() {
    let engine = Engine::bundled();
    let cases = [
        ("ai", "あい"),
        ("AI", "AI"),
        ("XYZga", "XYZが"),
        ("sexga", "sexが"),
        ("vscode", "vscode"),
        ("vscodede", "vscodeで"),
        ("ぷろzyえくと", "ぷろじぇくと"),
        ("purozyekuto", "ぷろじぇくと"),
        ("nあ", "んあ"),
        ("AIによる", "AIによる"),
    ];
    for i in 0..1_000_000 {
        let (raw, expected) = cases[i % cases.len()];
        assert_eq!(engine.normalize_input(raw, i % 2 == 0), expected);
    }
}

#[test]
fn project_reading_supports_zye_and_mixed_kana_vowels() {
    let engine =
        Engine::new(Dictionary::from_tsv("ぷろじぇくと\tプロジェクト\tnoun\t6000\n").unwrap());
    for input in [
        "purozyekuto",
        "purojyekuto",
        "ぷろzyえくと",
        "ぷろzyエクト",
        "ぷろじぇくと",
    ] {
        for live in [true, false] {
            assert_eq!(engine.normalize_input(input, live), "ぷろじぇくと");
        }
        assert_eq!(
            engine.convert(input, 8).unwrap().candidates[0].surface,
            "プロジェクト"
        );
        let mut session = nextime_core::Session::new(
            Engine::new(Dictionary::from_tsv("ぷろじぇくと\tプロジェクト\tnoun\t6000\n").unwrap()),
            8,
        )
        .unwrap();
        for ch in input.chars() {
            session
                .event(nextime_core::Key::Text(&ch.to_string()))
                .unwrap();
        }
        session.event(nextime_core::Key::Space).unwrap();
        assert_eq!(session.display(), "プロジェクト");
    }
    assert_eq!(engine.normalize_input("nあ", false), "んあ");
    assert_eq!(engine.normalize_input("ZYえ", false), "ZYえ");
    assert_eq!(engine.normalize_input("sexえ", false), "sexえ");
}

#[test]
fn predicts_nearby_words_without_unrequested_sentence_or_ai_expansion() {
    let engine = Engine::bundled();
    for input in ["yoroshi", "おせ", "あい", "ai"] {
        let candidates = engine.predict(input, 32, false).unwrap();
        assert!(
            candidates
                .iter()
                .all(|c| c.surface != "よろしくお願いします"
                    && c.surface != "お世話になっております"
                    && c.surface != "AIによる")
        );
    }
    assert!(
        engine
            .predict("nihon", 8, false)
            .unwrap()
            .iter()
            .any(|c| c.surface == "日本語")
    );
    assert!(
        engine
            .predict("よろしくおねがいします", 8, false)
            .unwrap()
            .iter()
            .any(|c| c.surface == "よろしくお願いします")
    );
    assert!(
        engine
            .predict("omg", 8, false)
            .unwrap()
            .iter()
            .all(|c| !c.surface.contains(' '))
    );
    let dictionary =
        Dictionary::from_tsv("ai\tai\tenglish\t5500\nによる\tによる\tparticle\t9000\n").unwrap();
    let engine = Engine::new(dictionary);
    for input in ["AIによる", "AIniyoru"] {
        let result = engine.convert(input, 8).unwrap();
        assert_eq!(result.reading, "AIによる");
        assert_eq!(result.candidates[0].surface, "AIによる");
    }
    assert_eq!(engine.normalize_input("ai", false), "あい");
}

#[test]
fn public_api_converts_romaji_kana_and_katakana() {
    let engine = Engine::bundled();
    for input in ["nihongo", "にほんご", "ニホンゴ"] {
        let result = engine.convert(input, 8).unwrap();
        assert_eq!(result.reading, "にほんご");
        assert_eq!(result.candidates[0].surface, "日本語");
    }
    assert_eq!(
        engine.convert("nihongonyuuryoku", 8).unwrap().candidates[0].surface,
        "日本語入力"
    );
}

#[test]
fn repeated_calls_are_deterministic_and_bounded() {
    let engine = Engine::bundled();
    let expected = engine.convert("hashi", 4).unwrap();
    for _ in 0..100 {
        assert_eq!(engine.convert("hashi", 4).unwrap(), expected);
    }
    assert!(expected.candidates.len() <= 4);
    assert!(engine.convert("", 8).unwrap().candidates.is_empty());
    assert_eq!(
        engine.convert(&"a".repeat(513), 8),
        Err(ConvertError::InputTooLong)
    );
}

#[test]
fn empty_dictionary_preserves_unknown_input() {
    let engine = Engine::new(Dictionary::from_tsv("").unwrap());
    assert_eq!(
        engine.convert("nihongo🙂", 8).unwrap().candidates[0].surface,
        "にほんご🙂"
    );
}

#[test]
fn unicode_and_ambiguous_input_obey_candidate_invariants() {
    let engine = Engine::bundled();
    for input in [
        "hashihashihashi",
        "🙂ニホンゴ。",
        "未知のことば",
        "にほんごをべんきょうする",
        "kyouあした",
        "は",
    ] {
        for limit in [1, 2, 8, 32] {
            let result = engine.convert(input, limit).unwrap();
            assert!(!result.candidates.is_empty());
            assert!(result.candidates.len() <= limit);
            let mut seen = std::collections::HashSet::new();
            for candidate in &result.candidates {
                assert!(seen.insert(&candidate.surface));
            }
            assert!(
                result
                    .candidates
                    .windows(2)
                    .all(|p| p[0].score >= p[1].score)
            );
            if limit > 1 {
                assert!(
                    result
                        .candidates
                        .iter()
                        .any(|c| c.surface == result.reading)
                );
            }
        }
    }
    assert!(engine.convert(&"🙂".repeat(128), 1).is_ok());
    assert_eq!(
        engine.convert(&"🙂".repeat(129), 1),
        Err(ConvertError::InputTooLong)
    );
}
