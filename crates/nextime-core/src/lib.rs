// SPDX-License-Identifier: AGPL-3.0-only
//! Platform-neutral offline IME API for in-memory and NEXC dictionaries.
//!
//! ```
//! use nextime_core::Engine;
//! let engine = Engine::bundled();
//! let result = engine.convert("nihongo", 8)?;
//! assert_eq!(result.reading, "にほんご");
//! assert_eq!(result.candidates[0].surface, "日本語");
//! # Ok::<(), nextime_core::ConvertError>(())
//! ```
pub use nextime_convert::{ConvertError, MAX_CANDIDATES, MAX_INPUT_CHARS};
pub use nextime_dict::{
    Dictionary, DictionaryError, DictionarySource, Entry, LookupError, WithCommonPredictions,
};
pub use nextime_ranker::Candidate;
mod session;
mod spelling;
mod typing;
pub mod width;
pub use session::{Key, Outcome, Session, SessionError};

#[derive(Debug, PartialEq, Eq)]
pub struct Conversion {
    pub reading: String,
    pub candidates: Vec<Candidate>,
}

#[derive(Debug)]
pub struct Engine {
    dictionary: Box<dyn DictionarySource>,
}

impl Engine {
    /// Conversion-time alternatives only; live preedit and literal input stay intact.
    fn corrected_reading(&self, input: &str) -> String {
        let words: Vec<_> = input.split(' ').collect();
        let input = if words.len() > 1
            && words.iter().any(|s| !self.is_english_word(s))
            && words
                .iter()
                .all(|s| s.bytes().all(|b| b.is_ascii_alphabetic()))
        {
            words.concat()
        } else {
            input.to_owned()
        };
        let aliases: Vec<_> = include_str!("../../../dictionaries/input-aliases.tsv")
            .lines()
            .filter(|s| !s.starts_with('#'))
            .filter_map(|s| s.split_once('\t'))
            .collect();
        let mut output = String::new();
        let mut rest = input.as_str();
        let mut prefix = String::new();
        while !rest.is_empty() {
            let alias = aliases.iter().find(|(from, _)| {
                rest.strip_prefix(from).is_some_and(|tail| {
                    ["de", "ni", "wo", "to", "no", "tte"]
                        .iter()
                        .any(|p| tail.starts_with(p))
                        && !nextime_parser::to_hiragana(&prefix)
                            .chars()
                            .any(|c| c.is_ascii_alphabetic())
                })
            });
            if let Some((from, to)) = alias {
                output.push_str(&self.normalize_input(&prefix, false));
                prefix.clear();
                output.push_str(to);
                rest = &rest[from.len()..];
            } else {
                let c = rest.chars().next().unwrap();
                prefix.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
        output.push_str(&self.normalize_input(&prefix, false));
        output
    }
    fn common_english(input: &str) -> bool {
        let explicit_acronym = input.len() >= 2 && input.chars().all(|c| c.is_ascii_uppercase());
        static COMMON: std::sync::OnceLock<Dictionary> = std::sync::OnceLock::new();
        COMMON
            .get_or_init(Dictionary::bundled)
            .lookup(&input.to_ascii_lowercase())
            .iter()
            .any(|e| {
                e.part_of_speech == "slang-en"
                    || ((input.len() >= 3 || explicit_acronym) && e.part_of_speech == "english")
            })
    }

    /// Preserve recognized ASCII words at script boundaries, and before a
    /// romaji particle (sexga -> sexが). Never split arbitrary English prefixes.
    pub fn normalize_input(&self, input: &str, live: bool) -> String {
        let mut output = String::new();
        let mut remaining = input;
        while !remaining.is_empty() {
            let ascii = remaining
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '\'');
            let end = remaining
                .char_indices()
                .find(|(_, c)| (c.is_ascii_alphabetic() || *c == '\'') != ascii)
                .map_or(remaining.len(), |(i, _)| i);
            let run = &remaining[..end];
            // Join an unfinished lowercase romaji syllable with an explicitly
            // typed kana vowel: zy + え -> zye -> じぇ. Known English and
            // uppercase choices remain literal; a lone n keeps its boundary.
            if ascii && run.chars().all(|c| c.is_ascii_lowercase()) && !self.is_english_word(run) {
                let pending = nextime_parser::to_preedit(run);
                let trailing = pending
                    .chars()
                    .rev()
                    .take_while(char::is_ascii_alphabetic)
                    .count();
                let vowel = remaining[end..].chars().next().and_then(|c| match c {
                    'あ' | 'ア' => Some('a'),
                    'い' | 'イ' => Some('i'),
                    'う' | 'ウ' => Some('u'),
                    'え' | 'エ' => Some('e'),
                    'お' | 'オ' => Some('o'),
                    _ => None,
                });
                if trailing >= 2 {
                    if let Some(vowel) = vowel {
                        let joined = nextime_parser::to_hiragana(&format!("{run}{vowel}"));
                        if !joined.chars().any(|c| c.is_ascii_alphabetic()) {
                            output.push_str(&joined);
                            let size = remaining[end..].chars().next().unwrap().len_utf8();
                            remaining = &remaining[end + size..];
                            continue;
                        }
                    }
                }
            }
            let parse = |s: &str| {
                if live && end == remaining.len() {
                    nextime_parser::to_preedit(s)
                } else {
                    nextime_parser::to_hiragana(s)
                }
            };
            let uppercase = run.chars().take_while(char::is_ascii_uppercase).count();
            let lower = run.to_ascii_lowercase();
            let split = ascii
                .then(|| {
                    (2..run.len()).rev().find(|&i| {
                        [
                            "ga", "wo", "ha", "ni", "de", "to", "mo", "no", "kara", "made",
                            "yori", "tte", "he", "ya",
                        ]
                            .iter()
                            .any(|p| lower[i..].starts_with(p))
                            // The active base/dynamic dictionary may contain
                            // learned English words absent from the bundled
                            // starter lexicon. Use the same recognition rule
                            // here as for a standalone word.
                            && self.is_english_word(&run[..i])
                    })
                })
                .flatten();
            if ascii && self.is_english_word(run) {
                output.push_str(run);
            } else if let Some(i) = split {
                output.push_str(&run[..i]);
                output.push_str(&parse(&run[i..]));
            } else if ascii && uppercase > 0 {
                // Uppercase is an explicit Latin input choice (Shift/Caps Lock),
                // including a single letter and unregistered abbreviations.
                output.push_str(&run[..uppercase]);
                if uppercase < run.len() {
                    let rest = &run[uppercase..];
                    output.push_str(&parse(rest));
                }
            } else {
                output.push_str(&parse(run));
            }
            remaining = &remaining[end..];
        }
        output
    }

    pub fn is_english_word(&self, input: &str) -> bool {
        input.is_ascii()
            && (Self::common_english(input)
                || self
                    .dictionary
                    .lookup(&input.to_ascii_lowercase(), 32)
                    .is_ok_and(|entries| {
                        entries.iter().any(|e| {
                            e.part_of_speech == "slang-en"
                                || (input.len() >= 3 && e.part_of_speech == "english")
                        })
                    }))
    }
    /// Prefix suggestions, not generated prose. Work is bounded by the source.
    pub fn predict(
        &self,
        input: &str,
        limit: usize,
        english: bool,
    ) -> Result<Vec<Candidate>, ConvertError> {
        if !(1..=MAX_CANDIDATES).contains(&limit) {
            return Err(ConvertError::InvalidLimit);
        }
        if input.len() > MAX_INPUT_CHARS * 4 {
            return Err(ConvertError::InputTooLong);
        }
        if input.is_empty() {
            return Ok(Vec::new());
        }
        let raw = input.to_ascii_lowercase();
        let kana = self.normalize_input(input, false);
        // Romaji is not an English completion prefix in Japanese mode.
        let prefixes = if english || self.is_english_word(input) {
            vec![raw]
        } else {
            vec![kana]
        };
        let mut result = Vec::new();
        for prefix in prefixes {
            for entry in self
                .dictionary
                .predict(&prefix, MAX_CANDIDATES)
                .map_err(ConvertError::Dictionary)?
            {
                let exact = entry.reading == prefix;
                let extra = entry
                    .reading
                    .chars()
                    .count()
                    .saturating_sub(prefix.chars().count());
                // Predict nearby words; do not invent the rest of a sentence
                // or expand an abbreviation into a whole English phrase.
                if (!exact
                    && (entry.part_of_speech == "phrase" || extra > if english { 4 } else { 2 }))
                    || (!exact && entry.surface.chars().count() > 12)
                    || (matches!(entry.part_of_speech.as_str(), "english" | "slang-en")
                        && entry.surface.chars().any(char::is_whitespace))
                {
                    continue;
                }
                result.push(Candidate {
                    surface: if exact
                        && matches!(entry.part_of_speech.as_str(), "english" | "slang-en")
                        && entry.surface == entry.reading
                        && input.chars().all(|c| c.is_ascii_alphabetic())
                    {
                        input.to_owned()
                    } else {
                        entry.surface
                    },
                    score: nextime_ranker::word_score(entry.frequency),
                    segments: 1,
                });
            }
        }
        nextime_ranker::rank(&mut result, limit);
        Ok(result)
    }
    pub fn new(dictionary: impl DictionarySource + 'static) -> Self {
        Self {
            dictionary: Box::new(dictionary),
        }
    }
    pub fn bundled() -> Self {
        Self::new(Dictionary::bundled())
    }
    pub fn dictionary_len(&self) -> usize {
        self.dictionary.len()
    }

    /// Includes input analysis, lookup, candidate generation, and ranking.
    /// Raw input is bounded separately to bound parser allocation/work.
    pub fn convert(&self, input: &str, limit: usize) -> Result<Conversion, ConvertError> {
        if input.len() > MAX_INPUT_CHARS * 4 {
            return Err(ConvertError::InputTooLong);
        }
        let reading = self.normalize_input(input, false);
        if self.is_english_word(input) {
            if !(1..=MAX_CANDIDATES).contains(&limit) {
                return Err(ConvertError::InvalidLimit);
            }
            let mut candidates = vec![Candidate {
                surface: input.into(),
                score: 0,
                segments: 1,
            }];
            for entry in self
                .dictionary
                .lookup(&input.to_ascii_lowercase(), limit)
                .map_err(ConvertError::Dictionary)?
                .iter()
            {
                if !candidates.iter().any(|c| c.surface == entry.surface)
                    && candidates.len() < limit
                {
                    candidates.push(Candidate {
                        surface: entry.surface.clone(),
                        score: nextime_ranker::word_score(entry.frequency),
                        segments: 1,
                    });
                }
            }
            return Ok(Conversion {
                reading: input.into(),
                candidates,
            });
        }
        let mut candidates = nextime_convert::convert(self.dictionary.as_ref(), &reading, limit)?;
        // The canonical reading is never changed. Missing n-boundaries are
        // conversion alternatives evaluated against actual dictionary entries,
        // with a penalty and a strict bound. Exact words keep their semantics.
        let alternatives = nextime_parser::ambiguous_n_inputs(input);
        if !alternatives.is_empty()
            && self
                .dictionary
                .lookup(&reading, 1)
                .map_err(ConvertError::Dictionary)?
                .is_empty()
        {
            let literal = candidates.iter().find(|c| c.surface == reading).cloned();
            for alternative in alternatives {
                let other = self.normalize_input(&alternative, false);
                if other == reading {
                    continue;
                }
                for mut candidate in
                    nextime_convert::convert(self.dictionary.as_ref(), &other, limit)?
                {
                    candidate.score = candidate.score.saturating_sub(256);
                    candidates.push(candidate);
                }
            }
            nextime_ranker::rank(&mut candidates, limit);
            if limit > 1 {
                if let Some(literal) = literal {
                    if !candidates.iter().any(|c| c.surface == reading) {
                        if candidates.len() == limit {
                            candidates.pop();
                        }
                        candidates.push(literal);
                    }
                }
            }
        }
        Ok(Conversion {
            reading,
            candidates,
        })
    }

    /// Compatibility API: context is ignored after AI removal.
    pub fn convert_with_context(
        &self,
        input: &str,
        _context: &str,
        limit: usize,
    ) -> Result<Conversion, ConvertError> {
        if !(1..=MAX_CANDIDATES).contains(&limit) {
            return Err(ConvertError::InvalidLimit);
        }
        let mut result = self.conversion_candidates(input, MAX_CANDIDATES)?;
        nextime_ranker::rank(&mut result.candidates, limit);
        Ok(result)
    }

    // Shared lexical generation only. Does not apply old model weights or start a backend.
    fn conversion_candidates(&self, input: &str, limit: usize) -> Result<Conversion, ConvertError> {
        // Request a wider set before contextual ranking; preserve the literal
        // fallback even when it would otherwise be truncated.
        if !(1..=MAX_CANDIDATES).contains(&limit) {
            return Err(ConvertError::InvalidLimit);
        }
        if input.len() > MAX_INPUT_CHARS * 4 {
            return Err(ConvertError::InputTooLong);
        }
        let original = self.normalize_input(input, false);
        if original.chars().count() > MAX_INPUT_CHARS {
            return Err(ConvertError::InputTooLong);
        }
        let corrected = self.corrected_reading(input);
        let reviewed = include_str!("../../../dictionaries/input-corrections.tsv")
            .lines()
            .filter_map(|s| s.split_once('\t'))
            .any(|(from, _)| corrected == from);
        let exact_correction = corrected != original
            && !self
                .dictionary
                .lookup(&corrected, MAX_CANDIDATES)
                .map_err(ConvertError::Dictionary)?
                .is_empty();
        // A verified whole-word correction need not search every segmentation
        // of the broken input. Preserve its literal and exact original entries.
        let fast = reviewed || exact_correction;
        let mut result = if fast {
            let mut candidates = if exact_correction {
                nextime_convert::convert(self.dictionary.as_ref(), &corrected, MAX_CANDIDATES)?
            } else {
                Vec::new()
            };
            for entry in self
                .dictionary
                .lookup(&original, MAX_CANDIDATES)
                .map_err(ConvertError::Dictionary)?
                .iter()
            {
                candidates.push(Candidate {
                    surface: entry.surface.clone(),
                    score: nextime_ranker::word_score(entry.frequency),
                    segments: 1,
                });
            }
            candidates.push(Candidate {
                surface: original.clone(),
                score: -100_000,
                segments: 1,
            });
            Conversion {
                reading: original,
                candidates,
            }
        } else {
            self.convert(input, MAX_CANDIDATES)?
        };
        if !fast && corrected != result.reading {
            for mut candidate in
                nextime_convert::convert(self.dictionary.as_ref(), &corrected, MAX_CANDIDATES)?
            {
                candidate.score -= 256;
                if !result
                    .candidates
                    .iter()
                    .any(|c| c.surface == candidate.surface)
                {
                    result.candidates.push(candidate);
                }
            }
        }
        for (from, to) in include_str!("../../../dictionaries/input-corrections.tsv")
            .lines()
            .filter(|s| !s.starts_with('#'))
            .filter_map(|s| s.split_once('\t'))
        {
            if corrected == from {
                result.candidates.push(Candidate {
                    surface: to.into(),
                    score: 8192,
                    segments: 1,
                });
            }
        }
        if !self.is_english_word(input) && result.reading.chars().any(|c| c.is_ascii_lowercase()) {
            let spellings = crate::spelling::typo_readings(input);
            for chunk in spellings.chunks(128) {
                let queries: Vec<_> = chunk.iter().map(String::as_str).collect();
                for matches in self
                    .dictionary
                    .lookup_batch(&queries, 4)
                    .map_err(ConvertError::Dictionary)?
                {
                    for entry in &matches {
                        if matches!(entry.part_of_speech.as_str(), "english" | "slang-en")
                            && !result.candidates.iter().any(|c| c.surface == entry.surface)
                        {
                            result.candidates.push(Candidate {
                                surface: entry.surface.clone(),
                                score: nextime_ranker::word_score(entry.frequency) - 512,
                                segments: 1,
                            });
                        }
                    }
                }
            }
        }
        for candidate in self.mixed_typo_candidates(input)? {
            if !result
                .candidates
                .iter()
                .any(|old| old.surface == candidate.surface)
            {
                result.candidates.push(candidate);
            }
        }
        Ok(result)
    }
}
