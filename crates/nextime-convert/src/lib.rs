// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded beam search over UTF-8 character boundaries.
use nextime_dict::{DictionarySource, LookupError};
use nextime_ranker::{Candidate, UNKNOWN_SCORE, rank, word_score};
mod grammar;

#[derive(Clone)]
struct Path {
    candidate: Candidate,
    pos: String,
}

fn rank_paths(paths: &mut Vec<Path>, limit: usize) {
    paths.sort_unstable_by(|a, b| {
        b.candidate
            .score
            .cmp(&a.candidate.score)
            .then_with(|| a.candidate.segments.cmp(&b.candidate.segments))
            .then_with(|| a.candidate.surface.cmp(&b.candidate.surface))
            .then_with(|| a.pos.cmp(&b.pos))
    });
    let mut seen = std::collections::HashSet::new();
    paths.retain(|p| seen.insert((p.candidate.surface.clone(), p.pos.clone())));
    paths.truncate(limit);
}

pub const MAX_INPUT_CHARS: usize = 128;
pub const MAX_CANDIDATES: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConvertError {
    InputTooLong,
    InvalidLimit,
    Dictionary(LookupError),
}

impl std::fmt::Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Self::Dictionary(error) = self {
            return write!(f, "dictionary lookup failed: {error}");
        }
        f.write_str(match self {
            Self::InputTooLong => {
                "input exceeds the Phase 1 limit (512 raw UTF-8 bytes or 128 normalized characters)"
            }
            Self::InvalidLimit => "candidate limit must be between 1 and 32",
            Self::Dictionary(_) => unreachable!(),
        })
    }
}
impl std::error::Error for ConvertError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Dictionary(error) = self {
            Some(error)
        } else {
            None
        }
    }
}

/// Input must already be normalized. Unknown characters stay available through
/// literal edges. Each position retains at most `limit * 4` paths with POS, so this is
/// an approximate N-best search, not an exhaustive enumeration.
pub fn convert(
    dictionary: &(impl DictionarySource + ?Sized),
    reading: &str,
    limit: usize,
) -> Result<Vec<Candidate>, ConvertError> {
    if !(1..=MAX_CANDIDATES).contains(&limit) {
        return Err(ConvertError::InvalidLimit);
    }
    let mut boundaries: Vec<_> = reading
        .char_indices()
        .take(MAX_INPUT_CHARS + 1)
        .map(|(i, _)| i)
        .collect();
    if boundaries.len() > MAX_INPUT_CHARS {
        return Err(ConvertError::InputTooLong);
    }
    if boundaries.is_empty() {
        return Ok(Vec::new());
    }
    boundaries.push(reading.len());
    let count = boundaries.len() - 1;
    // A whole dictionary word should not spawn speculative split compounds.
    let exact = dictionary
        .lookup(reading, MAX_CANDIDATES)
        .map_err(ConvertError::Dictionary)?;
    if !exact.is_empty() {
        let mut result: Vec<_> = exact
            .iter()
            .map(|e| Candidate {
                surface: e.surface.clone(),
                score: word_score(e.frequency),
                segments: 1,
            })
            .collect();
        finish(&mut result, reading, count, limit);
        return Ok(result);
    }
    let beam_limit = limit * 4;
    let mut beams = vec![Vec::<Path>::new(); count + 1];
    beams[0].push(Path {
        candidate: Candidate {
            surface: String::new(),
            score: 0,
            segments: 0,
        },
        pos: String::new(),
    });
    for start in 0..count {
        rank_paths(&mut beams[start], beam_limit);
        let prefixes = std::mem::take(&mut beams[start]);
        if start > 0 {
            for particle in ["が", "は"] {
                let tail = &reading[boundaries[start]..];
                if let Some(predicate_tail) = tail.strip_prefix(particle) {
                    for predicate in grammar::EVALUATIONS {
                        if !predicate_tail.starts_with(predicate) {
                            continue;
                        }
                        let entries = dictionary
                            .lookup(predicate, MAX_CANDIDATES)
                            .map_err(ConvertError::Dictionary)?;
                        for entry in entries.iter().filter(|e| {
                            grammar::is_predicate(&e.part_of_speech)
                                && grammar::can_end(&e.part_of_speech)
                        }) {
                            let end = start + 1 + predicate.chars().count();
                            extend(
                                &mut beams[end],
                                &prefixes,
                                &format!("{particle}{}", entry.surface),
                                word_score(entry.frequency) + 1024,
                                "predicate-phrase",
                            );
                        }
                    }
                }
            }
        }
        if start > 0 {
            if let Some(connective) = grammar::CONNECTIVES.iter().find(|text| {
                reading[boundaries[start]..].starts_with(**text)
                    && prefixes
                        .iter()
                        .any(|p| grammar::can_follow(&p.pos, "connective"))
            }) {
                extend(
                    &mut beams[start + connective.chars().count()],
                    &prefixes,
                    connective,
                    word_score(9000),
                    "connective",
                );
                continue;
            }
        }
        // Recognized English runs keep their original casing in mixed input.
        if reading[boundaries[start]..].starts_with(|c: char| c.is_ascii_alphabetic()) {
            let length = reading[boundaries[start]..]
                .chars()
                .take_while(|c| c.is_ascii_alphabetic())
                .count();
            let end = start + length;
            let word = &reading[boundaries[start]..boundaries[end]];
            let entries = dictionary
                .lookup(&word.to_ascii_lowercase(), MAX_CANDIDATES)
                .map_err(ConvertError::Dictionary)?;
            if word.chars().all(|c| c.is_ascii_uppercase())
                || entries
                    .iter()
                    .any(|e| matches!(e.part_of_speech.as_str(), "english" | "slang-en"))
            {
                extend(
                    &mut beams[end],
                    &prefixes,
                    word,
                    word_score(5500),
                    "english",
                );
            }
        }
        // Keep a recognized suru conjugation intact. Shorter edges such as
        // した + 位 must not compete with したい after a sahen noun.
        if let Some(suffix) = grammar::SUFFIXES
            .iter()
            .filter(|suffix| reading[boundaries[start]..].starts_with(**suffix))
            .max_by_key(|suffix| suffix.len())
            .filter(|_| {
                start > 0
                    && prefixes
                        .iter()
                        .any(|p| grammar::can_follow(&p.pos, "suru-suffix"))
            })
        {
            let end = start + suffix.chars().count();
            extend(
                &mut beams[end],
                &prefixes,
                suffix,
                word_score(9000),
                "suru-suffix",
            );
            continue;
        }
        let ends = start + 1..=(start + dictionary.max_reading_chars()).min(count);
        let slices: Vec<_> = ends
            .clone()
            .map(|end| &reading[boundaries[start]..boundaries[end]])
            .collect();
        let batches = dictionary
            .lookup_batch(&slices, MAX_CANDIDATES)
            .map_err(ConvertError::Dictionary)?;
        if batches.len() != slices.len() {
            return Err(ConvertError::Dictionary(LookupError(
                "invalid batch result length".into(),
            )));
        }
        for (end, matches) in ends.zip(batches) {
            let slice = &reading[boundaries[start]..boundaries[end]];
            let negative_predicate = start > 0
                && slice.ends_with("ない")
                && matches
                    .iter()
                    .any(|e| grammar::is_predicate(&e.part_of_speech));
            for entry in matches.iter() {
                if start > 0
                    && slice == "ない"
                    && !matches!(entry.surface.as_str(), "ない" | "無い")
                {
                    continue;
                }
                if negative_predicate && !grammar::is_predicate(&entry.part_of_speech) {
                    continue;
                }
                if start > 0
                    && (grammar::is_function_reading(slice)
                        || grammar::is_particle_reading(slice)
                        || grammar::is_particle(&entry.part_of_speech))
                    && entry.surface != slice
                {
                    continue;
                }
                extend(
                    &mut beams[end],
                    &prefixes,
                    &entry.surface,
                    word_score(entry.frequency),
                    &entry.part_of_speech,
                );
            }
            // Bound intermediate memory even for highly ambiguous dictionaries.
            rank_paths(&mut beams[end], beam_limit);
        }
        let literal = &reading[boundaries[start]..boundaries[start + 1]];
        // A one-character literal after a converted token often manufactures
        // a broken word (e.g. 穴 + る). Keep all-kana fallback separately.
        let literal_prefixes: Vec<_> = prefixes
            .iter()
            .filter(|p| {
                p.candidate.surface == reading[..boundaries[start]]
                    || !literal.chars().any(|c| ('ぁ'..='ん').contains(&c))
            })
            .cloned()
            .collect();
        extend(
            &mut beams[start + 1],
            &literal_prefixes,
            literal,
            UNKNOWN_SCORE,
            "",
        );
        rank_paths(&mut beams[start + 1], beam_limit);
    }
    let mut result: Vec<_> = std::mem::take(&mut beams[count])
        .into_iter()
        .filter(|p| grammar::can_end(&p.pos))
        .map(|p| p.candidate)
        .collect();
    finish(&mut result, reading, count, limit);
    Ok(result)
}

fn finish(result: &mut Vec<Candidate>, reading: &str, count: usize, limit: usize) {
    rank(result, limit);
    // Candidate count is a ceiling, not a quota. A speculative extra word
    // costs roughly 5,000 points; do not fill the UI with those weak paths.
    if let Some(best) = result.first().map(|c| c.score) {
        result.retain(|c| c.score >= best - 3072);
    }
    // Reserve a literal fallback when multiple candidates were requested.
    // For limit=1 the best conversion must take precedence.
    let fallback = Candidate {
        surface: reading.into(),
        score: UNKNOWN_SCORE * count as i64,
        segments: count,
    };
    if !result.iter().any(|c| c.surface == reading) && (limit > 1 || result.is_empty()) {
        if result.len() >= limit {
            rank(result, limit - 1);
        }
        result.push(fallback);
    }
    rank(result, limit);
}

fn extend(target: &mut Vec<Path>, prefixes: &[Path], surface: &str, score: i64, pos: &str) {
    for prefix in prefixes {
        if !grammar::can_follow(&prefix.pos, pos) {
            continue;
        }
        let mut combined = String::with_capacity(prefix.candidate.surface.len() + surface.len());
        combined.push_str(&prefix.candidate.surface);
        combined.push_str(surface);
        target.push(Path {
            candidate: Candidate {
                surface: combined,
                score: prefix.candidate.score
                    + score
                    + grammar::transition_score(&prefix.candidate.surface, surface),
                segments: prefix.candidate.segments + 1,
            },
            pos: pos.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn particle_chains_are_not_replaced_with_homophone_nouns_after_a_word() {
        let dictionary = nextime_dict::Dictionary::from_tsv(
            "ねこ\t猫\tnoun\t10000\nには\t二羽\tnoun\t20000\nには\tには\tparticle\t1000\nでは\t出羽\tnoun\t20000\nでは\tでは\tparticle\t1000\nのは\t野派\tnoun\t20000\nのは\tのは\tparticle\t1000\n"
        ).unwrap();
        for (reading, expected, wrong) in [
            ("ねこには", "猫には", "猫二羽"),
            ("ねこでは", "猫では", "猫出羽"),
            ("ねこのは", "猫のは", "猫野派"),
        ] {
            let result = convert(&dictionary, reading, 32).unwrap();
            assert!(result.iter().any(|c| c.surface == expected));
            assert!(!result.iter().any(|c| c.surface == wrong));
        }
        // Whole dictionary words still keep their registered noun alternatives.
        assert!(
            convert(&dictionary, "には", 32)
                .unwrap()
                .iter()
                .any(|c| c.surface == "二羽")
        );
    }
    #[test]
    fn curated_nouns_do_not_bypass_imported_word_constraints() {
        // Curated noun compounds remain supported. Imported arbitrary noun
        // combinations retain their restriction, including after curated nouns.
        assert!(grammar::can_follow("noun", "noun"));
        assert!(!grammar::can_follow("noun", "mozc:1851:1851:0"));
        assert!(grammar::can_follow("mozc:1851:1851:0", "noun"));
        assert!(!grammar::can_follow("noun", "suru-suffix"));
        assert!(grammar::can_follow("sahen", "suru-suffix"));
        assert!(grammar::can_follow("noun", "particle"));
        // Registered compounds still take the whole-word path.
        let dictionary = Dictionary::from_tsv(
            "かさ\t傘\tnoun\t9000\nやま\t山\tnoun\t9000\nかさやま\t傘山\tnoun\t9000\n",
        )
        .unwrap();
        assert_eq!(
            convert(&dictionary, "かさやま", 4).unwrap()[0].surface,
            "傘山"
        );
        let unregistered =
            Dictionary::from_tsv("かさ\t傘\tnoun\t9000\nやま\t山\tmozc:1851:1851:0\t9000\n")
                .unwrap();
        assert!(
            convert(&unregistered, "かさやま", 4)
                .unwrap()
                .iter()
                .all(|c| c.surface != "傘山")
        );
    }
    #[test]
    fn desire_conjugations_do_not_split_into_homophone_nouns() {
        let dict = Dictionary::from_tsv(
            "べんきょう\t勉強\tsahen\t6000\nけんさく\t検索\tmozc:1841:1841:100\t6000\nした\tした\tphrase\t9000\nしたい\t死体\tnoun\t9000\nい\t位\tnoun\t10000\nい\t医\tnoun\t10000\nい\t胃\tnoun\t10000"
        ).unwrap();
        for noun in ["べんきょう", "けんさく"] {
            let surface = if noun == "べんきょう" {
                "勉強"
            } else {
                "検索"
            };
            for suffix in [
                "したい",
                "したいです",
                "したかった",
                "したくない",
                "したくないです",
                "したくなかった",
                "したくて",
            ] {
                let reading = format!("{noun}{suffix}");
                let candidates = convert(&dict, &reading, 24).unwrap();
                assert_eq!(candidates[0].surface, format!("{surface}{suffix}"));
                assert!(
                    candidates
                        .iter()
                        .all(|c| c.surface == format!("{surface}{suffix}") || c.surface == reading),
                    "{candidates:?}"
                );
            }
        }
        assert_eq!(convert(&dict, "したい", 8).unwrap()[0].surface, "死体");
        assert!(
            convert(&dict, "い", 8)
                .unwrap()
                .iter()
                .any(|c| c.surface == "胃")
        );
    }
    #[test]
    fn evaluation_particles_and_sentence_endings_remain_grammatical() {
        let dict = Dictionary::from_tsv(
            "せいど\t精度\tnoun\t6500\nおと\t音\tnoun\t6000\nがいい\t害意\tnoun\t9000\nがわるい\t側類\tnoun\t9000\nいい\tいい\tadjective\t5000\nいい\t良い\tadjective\t4900\nわるい\t悪い\tadjective\t5000\nよ\tよ\tparticle\t4000\nよ\t余\tnoun\t9000\nよ\t夜\tnoun\t9000\nね\tね\tparticle\t4000\nね\t根\tnoun\t9000"
        ).unwrap();
        for (reading, expected) in [
            ("せいどがいい", "精度がいい"),
            ("せいどがいいよ", "精度がいいよ"),
            ("おとがいいね", "音がいいね"),
            ("せいどがわるい", "精度が悪い"),
            ("せいどはわるい", "精度は悪い"),
        ] {
            assert_eq!(convert(&dict, reading, 8).unwrap()[0].surface, expected);
        }
        assert_eq!(convert(&dict, "がいい", 8).unwrap()[0].surface, "害意");
        assert_eq!(convert(&dict, "よ", 8).unwrap()[0].surface, "余");
    }

    #[test]
    fn demonstrative_context_prefers_feeling_without_removing_kanji() {
        let dict = Dictionary::from_tsv(
            "こんな\tこんな\tadnominal\t5000\nそんな\tそんな\tadnominal\t5000\nあんな\tあんな\tadnominal\t5000\nどんな\tどんな\tadnominal\t5000\nかんじ\t漢字\tnoun\t8500\nかんじ\t感じ\tnoun\t8000"
        ).unwrap();
        for prefix in ["こんな", "そんな", "あんな", "どんな"] {
            let result = convert(&dict, &format!("{prefix}かんじ"), 8).unwrap();
            assert_eq!(result[0].surface, format!("{prefix}感じ"));
            assert!(result.iter().any(|c| c.surface == format!("{prefix}漢字")));
        }
        assert_eq!(convert(&dict, "かんじ", 8).unwrap()[0].surface, "漢字");
    }
    #[test]
    fn noun_connectives_do_not_turn_into_night_or_nonfinite_verbs() {
        let dictionary = Dictionary::from_tsv("あい\t愛\tnoun\t6000\nあい\t会い\tmozc:829:829:2002\t9000\nに\tに\tparticle\t8000\nよる\t夜\tnoun\t9000\n").unwrap();
        let result = convert(&dictionary, "あいによる", 8).unwrap();
        assert_eq!(result[0].surface, "愛による");
        assert!(
            result
                .iter()
                .all(|c| !c.surface.contains('夜') && !c.surface.starts_with("会い"))
        );
    }
    use nextime_dict::Dictionary;
    #[test]
    fn homophones_and_frequency() {
        let result = convert(&Dictionary::bundled(), "はし", 8).unwrap();
        assert_eq!(result[0].surface, "橋");
        for surface in ["橋", "箸", "端", "はし"] {
            assert!(result.iter().any(|c| c.surface == surface));
        }
        assert_eq!(
            convert(&Dictionary::bundled(), "はし", 1).unwrap()[0].surface,
            "橋"
        );
    }
    #[test]
    fn segmentation_and_unknown_characters() {
        let dict = Dictionary::bundled();
        assert_eq!(
            convert(&dict, "にほんごにゅうりょく", 8).unwrap()[0].surface,
            "日本語入力"
        );
        assert_eq!(
            convert(&dict, "🙂にほんご", 8).unwrap()[0].surface,
            "🙂日本語"
        );
        assert_eq!(convert(&dict, "謎🙂", 8).unwrap()[0].surface, "謎🙂");
    }
    #[test]
    fn boundaries_and_limits() {
        let dict = Dictionary::bundled();
        assert!(convert(&dict, "", 8).unwrap().is_empty());
        assert_eq!(convert(&dict, "a", 0), Err(ConvertError::InvalidLimit));
        assert_eq!(convert(&dict, "a", 33), Err(ConvertError::InvalidLimit));
        assert_eq!(
            convert(&dict, &"あ".repeat(129), 8),
            Err(ConvertError::InputTooLong)
        );
        assert!(convert(&dict, &"🙂".repeat(128), 1).is_ok());
    }

    #[test]
    fn whole_words_do_not_spawn_arbitrary_kanji_compounds() {
        let dict = Dictionary::from_tsv(
            "あなる\tアナル\tmozc:1851:1851:100\t3000\nあ\t亜\tmozc:1924:1924:100\t3000\nなる\t成\tmozc:1922:1922:100\t3000\nなる\t鳴る\tmozc:837:837:100\t3000"
        ).unwrap();
        let result = convert(&dict, "あなる", 24).unwrap();
        assert_eq!(
            result
                .iter()
                .map(|c| c.surface.as_str())
                .collect::<Vec<_>>(),
            ["アナル", "あなる"]
        );
    }

    #[test]
    fn suru_suffix_requires_a_compatible_noun() {
        let dict = Dictionary::from_tsv(
            "たいさく\t対策\tmozc:1841:1841:100\t3000\nたいさく\t大作\tmozc:1851:1851:100\t3100\nたいさく\t太作\tmozc:1922:1922:100\t3200\nして\t仕手\tmozc:1851:1851:100\t4000\nほしい\tほしい\tmozc:2579:2579:100\t4000\nです\tです\tmozc:172:172:100\t4000"
        ).unwrap();
        let result = convert(&dict, "たいさくしてほしいです", 24).unwrap();
        assert_eq!(result[0].surface, "対策してほしいです");
        assert!(result.iter().all(|c| !c.surface.contains("仕手")
            && !c.surface.starts_with("大作")
            && !c.surface.starts_with("太作")));
        // The standalone word is valid and must remain convertible.
        assert!(
            convert(&dict, "して", 24)
                .unwrap()
                .iter()
                .any(|c| c.surface == "仕手")
        );
    }

    #[test]
    fn particles_do_not_become_homophone_nouns_in_sentences() {
        let dict = Dictionary::from_tsv(
            "sex\tsex\tenglish\t5500\nが\tが\tmozc:332:332:0\t4000\nが\t我\tmozc:1851:1851:0\t4100\nが\t画\tmozc:1949:1949:0\t4200\nが\t蛾\tmozc:1851:1851:0\t4000\nできない\tできない\tmozc:697:204:0\t4000\nできない\t出来ない\tmozc:699:204:0\t3990\nできない\tデキナイ\tmozc:1851:1851:0\t4100"
        ).unwrap();
        let result = convert(&dict, "sexができない", 24).unwrap();
        assert_eq!(
            result
                .iter()
                .map(|c| c.surface.as_str())
                .collect::<Vec<_>>(),
            ["sexができない", "sexが出来ない"]
        );
        assert!(
            convert(&dict, "が", 24)
                .unwrap()
                .iter()
                .any(|c| c.surface == "我")
        );
    }
}
