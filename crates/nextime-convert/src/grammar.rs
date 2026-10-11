// SPDX-License-Identifier: AGPL-3.0-only
//! Conservative local grammar checks, not Mozc's statistical connection model.
use std::sync::OnceLock;

fn description(pos: &str, right: bool) -> &str {
    static IDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    let mut fields = pos.split(':');
    if fields.next() != Some("mozc") {
        return pos;
    }
    let left = fields.next();
    let id = if right { fields.next() } else { left }.and_then(|s| s.parse::<usize>().ok());
    let ids = IDS.get_or_init(|| {
        include_str!("../../../dictionaries/mozc-id.def")
            .lines()
            .map(|s| s.split_once(' ').map_or("", |(_, s)| s))
            .collect()
    });
    id.and_then(|i| ids.get(i).copied()).unwrap_or("")
}

pub fn can_follow(previous: &str, current: &str) -> bool {
    let previous = description(previous, true);
    let current = description(current, false);
    if previous.is_empty() || current.is_empty() {
        return true;
    }
    if current == "connective" {
        return previous.starts_with("名詞")
            || matches!(previous, "noun" | "english" | "slang-en" | "sahen");
    }
    // Unregistered noun compounds are ambiguous; preserve kana instead of
    // manufacturing arbitrary names/one-character kanji compounds.
    if (previous.starts_with("名詞") || matches!(previous, "english" | "slang-en"))
        && current.starts_with("名詞")
    {
        return previous.contains("接頭") || current.contains("接尾");
    }
    if current.contains("サ変・スル") && previous.starts_with("名詞") {
        return previous.contains("サ変接続");
    }
    if current == "suru-suffix" {
        return previous.contains("サ変接続") || previous == "sahen";
    }
    if previous.starts_with("名詞") && current.contains("特殊・マス") {
        return false;
    }
    if previous.starts_with("動詞") && !previous.contains("基本形") && current.starts_with("名詞")
    {
        return previous.contains("連用") && current.contains("接尾");
    }
    // Verb/adjective suffixes must not be attached to arbitrary names/nouns.
    if previous.starts_with("名詞")
        && (current.starts_with("動詞") || current.starts_with("形容詞,非自立"))
    {
        return false;
    }
    true
}

/// Common grammatical phrases stay in kana; these are not standalone noun
/// aliases such as 仕手. Longer strings get an edge of their own.
pub const SUFFIXES: &[&str] = &[
    "して",
    "する",
    "した",
    "します",
    "しました",
    "しています",
    "してください",
    "してほしい",
    "してほしいです",
    "していただきたい",
];

/// Noun-connected grammatical expressions must not split into 夜/寄る etc.
pub const CONNECTIVES: &[&str] = &[
    "によって",
    "について",
    "に対して",
    "による",
    "により",
    "として",
];

pub fn is_function_reading(reading: &str) -> bool {
    matches!(reading, "です" | "ます" | "でした" | "ました" | "ほしい")
        || SUFFIXES.contains(&reading)
}

pub fn is_particle(pos: &str) -> bool {
    description(pos, false).starts_with("助詞")
}

pub fn is_particle_reading(reading: &str) -> bool {
    matches!(
        reading,
        "が" | "を" | "は" | "に" | "で" | "と" | "も" | "の" | "へ" | "や" | "て"
    )
}

pub fn is_predicate(pos: &str) -> bool {
    let pos = description(pos, false);
    pos.starts_with("動詞")
        || pos.starts_with("形容詞")
        || pos.starts_with("助動詞")
        || matches!(pos, "verb" | "adjective")
}

pub fn can_end(pos: &str) -> bool {
    let pos = description(pos, true);
    if pos.starts_with("接頭詞") {
        return false;
    }
    if pos.starts_with("動詞") || pos.starts_with("形容詞") || pos.starts_with("助動詞") {
        return pos.contains("基本形") || pos.contains("命令") || pos.contains("体言接続");
    }
    true
}
