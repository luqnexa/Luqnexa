// SPDX-License-Identifier: AGPL-3.0-only
//! Offline modern verb forms from explicit dictionary conjugation metadata.
use nextime_dict::Entry;

pub fn productive_forms(entry: &Entry) -> Vec<Entry> {
    let description = crate::grammar::description(&entry.part_of_speech, true);
    let fields: Vec<_> = description.split(',').collect();
    if fields.len() < 7 || fields[0] != "動詞" || fields[1] != "自立" || fields[5] != "基本形"
    {
        return Vec::new();
    }
    let Some(last) = entry.reading.chars().last() else {
        return Vec::new();
    };
    if !entry.surface.ends_with(last) {
        return Vec::new();
    }
    let (negative, polite, te, past) = match (fields[4], last) {
        ("一段", 'る') => ("", "", "て", "た"),
        ("五段・カ行イ音便", 'く') => ("か", "き", "いて", "いた"),
        ("五段・カ行促音便", 'く') => ("か", "き", "って", "った"),
        ("五段・ワ行促音便", 'う') => ("わ", "い", "って", "った"),
        ("五段・ワ行ウ音便", 'う') => ("わ", "い", "うて", "うた"),
        ("五段・ラ行", 'る') if !matches!(entry.reading.as_str(), "ある" | "ござる") => {
            ("ら", "り", "って", "った")
        }
        ("五段動詞", 'う') => ("わ", "い", "って", "った"),
        ("五段動詞", 'く') => ("か", "き", "いて", "いた"),
        ("五段動詞", 'ぐ') => ("が", "ぎ", "いで", "いだ"),
        ("五段動詞", 'す') => ("さ", "し", "して", "した"),
        ("五段動詞", 'つ') => ("た", "ち", "って", "った"),
        ("五段動詞", 'ぬ') => ("な", "に", "んで", "んだ"),
        ("五段動詞", 'ぶ') => ("ば", "び", "んで", "んだ"),
        ("五段動詞", 'む') => ("ま", "み", "んで", "んだ"),
        ("五段動詞", 'る') if !matches!(entry.reading.as_str(), "ある" | "ござる") => {
            ("ら", "り", "って", "った")
        }
        _ => return Vec::new(),
    };
    let reading_stem = &entry.reading[..entry.reading.len() - last.len_utf8()];
    let surface_stem = &entry.surface[..entry.surface.len() - last.len_utf8()];
    let endings = [
        format!("{negative}ない"),
        format!("{negative}なかった"),
        format!("{negative}なくて"),
        format!("{negative}なければ"),
        format!("{polite}ます"),
        format!("{polite}ました"),
        format!("{polite}ません"),
        format!("{polite}たい"),
        format!("{polite}たくない"),
        te.into(),
        past.into(),
        format!("{te}いる"),
        format!("{te}いた"),
        format!("{te}います"),
        format!("{te}ください"),
    ];
    endings
        .into_iter()
        .filter_map(|ending| {
            let result = Entry {
                reading: format!("{reading_stem}{ending}"),
                surface: format!("{surface_stem}{ending}"),
                part_of_speech: format!("{}:derived", entry.part_of_speech),
                frequency: entry.frequency,
            };
            nextime_dict::validate_entry(&result)
                .is_ok()
                .then_some(result)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn word(reading: &str, surface: &str, id: usize) -> Entry {
        Entry {
            reading: reading.into(),
            surface: surface.into(),
            part_of_speech: format!("mozc:{id}:{id}:1000"),
            frequency: 4000,
        }
    }
    #[test]
    fn explicit_classes_generate_regular_and_exceptional_forms_without_guessing() {
        for (entry, expected) in [
            (word("たべる", "食べる", 680), "食べなかった"),
            (word("かく", "書く", 723), "書いて"),
            (word("いく", "行く", 731), "行った"),
            (word("とう", "問う", 799), "問うた"),
            (word("かう", "買う", 813), "買いました"),
            (word("はなす", "話す", 837), "話して"),
        ] {
            let forms = productive_forms(&entry);
            assert!(
                forms.iter().any(|form| form.surface == expected),
                "{}",
                entry.surface
            );
            assert!(forms.iter().all(|form| form.frequency == entry.frequency
                && form.part_of_speech.starts_with(&entry.part_of_speech)));
        }
        assert!(productive_forms(&word("ある", "ある", 763)).is_empty());
        assert!(productive_forms(&word("ゆく", "行く", 739)).is_empty());
        assert!(productive_forms(&word("たべる", "食べ物", 680)).is_empty());
        assert!(productive_forms(&word("かく", "画角", 1851)).is_empty());
    }
}
