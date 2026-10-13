// SPDX-License-Identifier: AGPL-3.0-only
//! Deterministic romaji parsing and kana normalization; no I/O or network access.

/// Normalize ASCII romaji and full-width katakana to hiragana.
/// Unknown characters are retained. A final `n` is finalized as ん.
/// This is a committed-input parser, not an incremental preedit state machine.
pub fn to_hiragana(input: &str) -> String {
    parse(input, true)
}

/// Render live composition, retaining trailing ambiguous romaji (e.g. n, ky).
/// Reparse the bounded raw input on each edit so n -> na and nn -> nna work.
pub fn to_preedit(input: &str) -> String {
    parse(input, false)
}

/// Bounded conversion alternatives for an omitted n/apostrophe boundary,
/// e.g. shinya -> shin'ya. The ordinary parser and live preedit are unchanged.
/// Only two single-boundary alternatives are returned; no exponential search.
pub fn ambiguous_n_inputs(input: &str) -> Vec<String> {
    let bytes = input.as_bytes();
    let mut result = Vec::new();
    for (i, window) in bytes.windows(3).enumerate() {
        if window[0] == b'n'
            && window[1] == b'y'
            && b"aou".contains(&window[2])
            && (i == 0 || bytes[i - 1] != b'n')
        {
            let mut alternative = input.to_owned();
            // n is ASCII, hence this is always a UTF-8 character boundary.
            alternative.insert(i + 1, '\'');
            result.push(alternative);
            if result.len() == 2 {
                break;
            }
        }
    }
    result
}

fn parse(input: &str, finalize: bool) -> String {
    let input: String = input
        .chars()
        .map(|c| match c {
            'A'..='Z' => c.to_ascii_lowercase(),
            'ァ'..='ヶ' => char::from_u32(c as u32 - 0x60).unwrap_or(c),
            _ => c,
        })
        .collect();
    let mut remaining = input.as_str();
    let mut output = String::with_capacity(input.len());
    while !remaining.is_empty() {
        if !finalize
            && (remaining == "n"
                || ROMAJI.iter().any(|(roman, _)| {
                    roman.len() > remaining.len() && roman.starts_with(remaining)
                })
                || (remaining.len() < 3 && "tch".starts_with(remaining)))
        {
            output.push_str(remaining);
            break;
        }
        let bytes = remaining.as_bytes();
        if (bytes.len() >= 2 && bytes[0] == bytes[1] && b"bcdfghjkmprstvwz".contains(&bytes[0]))
            || remaining.starts_with("tch")
        {
            output.push('っ');
            remaining = &remaining[1..];
            continue;
        }
        if bytes[0] == b'n' {
            if remaining.starts_with("n'") {
                output.push('ん');
                remaining = &remaining[2..];
                continue;
            }
            if remaining.starts_with("nn") {
                output.push('ん');
                // nna -> んな; nn at the end, or before a consonant -> ん.
                let keep_second = bytes.get(2).is_some_and(|b| b"aiueoy".contains(b));
                remaining = &remaining[if keep_second { 1 } else { 2 }..];
                continue;
            }
            if bytes.len() == 1 || !b"aiueoy".contains(&bytes[1]) {
                output.push('ん');
                remaining = &remaining[1..];
                continue;
            }
        }
        let matched = ROMAJI
            .iter()
            .filter(|(roman, _)| remaining.starts_with(roman))
            .max_by_key(|(roman, _)| roman.len());
        if let Some((roman, kana)) = matched {
            output.push_str(kana);
            remaining = &remaining[roman.len()..];
        } else {
            let c = remaining.chars().next().expect("nonempty input");
            output.push(c);
            remaining = &remaining[c.len_utf8()..];
        }
    }
    output
}

const ROMAJI: &[(&str, &str)] = &[
    ("kya", "きゃ"),
    ("kyu", "きゅ"),
    ("kyo", "きょ"),
    ("gya", "ぎゃ"),
    ("gyu", "ぎゅ"),
    ("gyo", "ぎょ"),
    ("sha", "しゃ"),
    ("shu", "しゅ"),
    ("sho", "しょ"),
    ("sya", "しゃ"),
    ("syu", "しゅ"),
    ("syo", "しょ"),
    ("cha", "ちゃ"),
    ("chu", "ちゅ"),
    ("cho", "ちょ"),
    ("tya", "ちゃ"),
    ("tyu", "ちゅ"),
    ("tyo", "ちょ"),
    ("cya", "ちゃ"),
    ("cyu", "ちゅ"),
    ("cyo", "ちょ"),
    ("nya", "にゃ"),
    ("nyu", "にゅ"),
    ("nyo", "にょ"),
    ("hya", "ひゃ"),
    ("hyu", "ひゅ"),
    ("hyo", "ひょ"),
    ("bya", "びゃ"),
    ("byu", "びゅ"),
    ("byo", "びょ"),
    ("pya", "ぴゃ"),
    ("pyu", "ぴゅ"),
    ("pyo", "ぴょ"),
    ("mya", "みゃ"),
    ("myu", "みゅ"),
    ("myo", "みょ"),
    ("rya", "りゃ"),
    ("ryu", "りゅ"),
    ("ryo", "りょ"),
    ("ja", "じゃ"),
    ("ju", "じゅ"),
    ("jo", "じょ"),
    ("jya", "じゃ"),
    ("jyu", "じゅ"),
    ("jyo", "じょ"),
    ("zya", "じゃ"),
    ("zyu", "じゅ"),
    ("zyo", "じょ"),
    ("zye", "じぇ"),
    ("jye", "じぇ"),
    ("sye", "しぇ"),
    ("tye", "ちぇ"),
    ("cye", "ちぇ"),
    ("dya", "ぢゃ"),
    ("dyu", "ぢゅ"),
    ("dyo", "ぢょ"),
    ("shi", "し"),
    ("chi", "ち"),
    ("tsu", "つ"),
    ("xtsu", "っ"),
    ("ltsu", "っ"),
    ("xtu", "っ"),
    ("ltu", "っ"),
    ("she", "しぇ"),
    ("che", "ちぇ"),
    ("je", "じぇ"),
    ("fa", "ふぁ"),
    ("fi", "ふぃ"),
    ("fe", "ふぇ"),
    ("fo", "ふぉ"),
    ("va", "ゔぁ"),
    ("vi", "ゔぃ"),
    ("vu", "ゔ"),
    ("ve", "ゔぇ"),
    ("vo", "ゔぉ"),
    ("thi", "てぃ"),
    ("dhi", "でぃ"),
    ("twu", "とぅ"),
    ("dwu", "どぅ"),
    ("tsa", "つぁ"),
    ("tsi", "つぃ"),
    ("tse", "つぇ"),
    ("tso", "つぉ"),
    ("wi", "うぃ"),
    ("we", "うぇ"),
    ("wo", "を"),
    ("xa", "ぁ"),
    ("xi", "ぃ"),
    ("xu", "ぅ"),
    ("xe", "ぇ"),
    ("xo", "ぉ"),
    ("la", "ぁ"),
    ("li", "ぃ"),
    ("lu", "ぅ"),
    ("le", "ぇ"),
    ("lo", "ぉ"),
    ("xya", "ゃ"),
    ("xyu", "ゅ"),
    ("xyo", "ょ"),
    ("lya", "ゃ"),
    ("lyu", "ゅ"),
    ("lyo", "ょ"),
    ("xwa", "ゎ"),
    ("lwa", "ゎ"),
    ("ka", "か"),
    ("ki", "き"),
    ("ku", "く"),
    ("ke", "け"),
    ("ko", "こ"),
    ("ga", "が"),
    ("gi", "ぎ"),
    ("gu", "ぐ"),
    ("ge", "げ"),
    ("go", "ご"),
    ("sa", "さ"),
    ("si", "し"),
    ("su", "す"),
    ("se", "せ"),
    ("so", "そ"),
    ("za", "ざ"),
    ("zi", "じ"),
    ("ji", "じ"),
    ("zu", "ず"),
    ("ze", "ぜ"),
    ("zo", "ぞ"),
    ("ta", "た"),
    ("ti", "ち"),
    ("tu", "つ"),
    ("te", "て"),
    ("to", "と"),
    ("da", "だ"),
    ("di", "ぢ"),
    ("du", "づ"),
    ("de", "で"),
    ("do", "ど"),
    ("na", "な"),
    ("ni", "に"),
    ("nu", "ぬ"),
    ("ne", "ね"),
    ("no", "の"),
    ("ha", "は"),
    ("hi", "ひ"),
    ("hu", "ふ"),
    ("fu", "ふ"),
    ("he", "へ"),
    ("ho", "ほ"),
    ("ba", "ば"),
    ("bi", "び"),
    ("bu", "ぶ"),
    ("be", "べ"),
    ("bo", "ぼ"),
    ("pa", "ぱ"),
    ("pi", "ぴ"),
    ("pu", "ぷ"),
    ("pe", "ぺ"),
    ("po", "ぽ"),
    ("ma", "ま"),
    ("mi", "み"),
    ("mu", "む"),
    ("me", "め"),
    ("mo", "も"),
    ("ya", "や"),
    ("yu", "ゆ"),
    ("yo", "よ"),
    ("ra", "ら"),
    ("ri", "り"),
    ("ru", "る"),
    ("re", "れ"),
    ("ro", "ろ"),
    ("wa", "わ"),
    ("a", "あ"),
    ("i", "い"),
    ("u", "う"),
    ("e", "え"),
    ("o", "お"),
    ("-", "ー"),
    (",", "、"),
    (".", "。"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_and_finalized_romaji_are_distinct() {
        for (input, preedit, final_text) in [
            ("n", "n", "ん"),
            ("nn", "ん", "ん"),
            ("nna", "んな", "んな"),
            ("kan'", "かん", "かん"),
            ("ky", "ky", "ky"),
            ("gakk", "がっk", "がっk"),
            ("matc", "まtc", "まtc"),
            ("matcha", "まっちゃ", "まっちゃ"),
            ("shi", "し", "し"),
            ("ニホン🙂", "にほん🙂", "にほん🙂"),
        ] {
            assert_eq!(to_preedit(input), preedit, "{input}");
            assert_eq!(to_hiragana(input), final_text, "{input}");
        }
    }

    #[test]
    fn common_spellings() {
        for (input, expected) in [
            ("nihongo", "にほんご"),
            ("SHINBUN", "しんぶん"),
            ("konnichiha", "こんにちは"),
            ("kanpai", "かんぱい"),
            ("gakkou", "がっこう"),
            ("matcha", "まっちゃ"),
            ("kyoushi", "きょうし"),
            ("shashin", "しゃしん"),
            ("n", "ん"),
            ("nn", "ん"),
            ("nna", "んな"),
            ("nnka", "んか"),
            ("kan'i", "かんい"),
            ("nya", "にゃ"),
            ("xtsu", "っ"),
            ("fa-thi", "ふぁーてぃ"),
            ("カタカナ", "かたかな"),
            ("東京🙂", "東京🙂"),
            ("k", "k"),
            ("ny", "ny"),
            ("", ""),
        ] {
            assert_eq!(to_hiragana(input), expected, "{input}");
        }
    }

    #[test]
    fn every_table_entry_roundtrips() {
        for &(roman, kana) in ROMAJI {
            assert_eq!(to_hiragana(roman), kana, "{roman}");
        }
    }
}
