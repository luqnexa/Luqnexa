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
        for suffix in grammar::SUFFIXES {
            if reading[boundaries[start]..].starts_with(suffix) {
                let end = start + suffix.chars().count();
                extend(
                    &mut beams[end],
                    &prefixes,
                    suffix,
                    word_score(9000),
                    "suru-suffix",
                );
            }
        }
        for end in start + 1..=(start + dictionary.max_reading_chars()).min(count) {
            let slice = &reading[boundaries[start]..boundaries[end]];
            let matches = dictionary
                .lookup(slice, MAX_CANDIDATES)
                .map_err(ConvertError::Dictionary)?;
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
                score: prefix.candidate.score + score,
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
