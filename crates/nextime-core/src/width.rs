// SPDX-License-Identifier: AGPL-3.0-only
//! Explicit width conversion; unrelated Unicode is preserved.
pub fn full_width(text: &str) -> String {
    let mut result = String::new();
    for ch in text.chars() {
        if let Some(index) = HALF.chars().position(|c| c == ch) {
            result.push(FULL.chars().nth(index).unwrap());
        } else {
            result.push(match ch {
                ' ' => '\u{3000}',
                '!'..='~' => char::from_u32(ch as u32 + 0xfee0).unwrap(),
                _ => ch,
            });
        }
    }
    // Compose only standard voiced/semi-voiced kana, keeping invalid marks intact.
    for (plain, voiced) in [
        (
            "カキクケコサシスセソタチツテトハヒフヘホウ",
            "ガギグゲゴザジズゼゾダヂヅデドバビブベボヴ",
        ),
        ("ハヒフヘホ", "パピプペポ"),
    ] {
        let mark = if voiced.starts_with('ガ') {
            '゛'
        } else {
            '゜'
        };
        for (a, b) in plain.chars().zip(voiced.chars()) {
            result = result.replace(&format!("{a}{mark}"), &b.to_string());
        }
    }
    result
}

pub fn half_width(text: &str) -> String {
    let mut result = String::new();
    for ch in text.chars() {
        if let Some(index) = "ガギグゲゴザジズゼゾダヂヅデドバビブベボヴ"
            .chars()
            .position(|c| c == ch)
        {
            result.push("ｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾊﾋﾌﾍﾎｳ".chars().nth(index).unwrap());
            result.push('ﾞ');
        } else if let Some(index) = "パピプペポ".chars().position(|c| c == ch) {
            result.push("ﾊﾋﾌﾍﾎ".chars().nth(index).unwrap());
            result.push('ﾟ');
        } else if let Some(index) = FULL.chars().position(|c| c == ch) {
            result.push(HALF.chars().nth(index).unwrap());
        } else {
            result.push(match ch {
                '\u{3000}' => ' ',
                '！'..='～' => char::from_u32(ch as u32 - 0xfee0).unwrap(),
                _ => ch,
            });
        }
    }
    result
}
const HALF: &str = "｡｢｣､･ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝﾞﾟ";
const FULL: &str = "。「」、・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_ascii_and_voiced_kana_preserves_other_scripts() {
        let text = "Python 123! ガッツポーズ 中文🙂한국어";
        assert_eq!(
            full_width(&half_width(text)),
            "Ｐｙｔｈｏｎ　１２３！　ガッツポーズ　中文🙂한국어"
        );
        assert_eq!(half_width(&full_width("Hello 123!")), "Hello 123!");
        assert_eq!(full_width("ｳﾞｶﾞﾊﾟ"), "ヴガパ");
        assert_eq!(full_width("ｱﾞ"), "ア゛");
    }
}
