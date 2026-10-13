// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded dictionary-verified English prefix correction at Japanese particles.
use crate::{Candidate, ConvertError, Engine, MAX_CANDIDATES};

impl Engine {
    pub(crate) fn mixed_typo_candidates(
        &self,
        input: &str,
    ) -> Result<Vec<Candidate>, ConvertError> {
        // Preserve explicit uppercase/code/foreign scripts; no correction during preedit.
        if !(7..=96).contains(&input.len()) || !input.bytes().all(|b| b.is_ascii_lowercase()) {
            return Ok(Vec::new());
        }
        let mut result = Vec::new();
        let mut boundaries = 0;
        for end in 4..=input.len().min(24) {
            let tail = &input[end..];
            if tail.len() < 3
                || ![
                    "de", "ni", "wo", "no", "ga", "ha", "to", "mo", "kara", "made",
                ]
                .iter()
                .any(|p| tail.starts_with(p))
            {
                continue;
            }
            let reading = self.normalize_input(tail, false);
            if reading.chars().any(|c| c.is_ascii_alphabetic())
                || self.is_english_word(&input[..end])
            {
                continue;
            }
            boundaries += 1;
            if boundaries > 2 {
                break;
            }
            let mut entries = Vec::new();
            for spelling in nextime_ai::typo_readings(&input[..end]) {
                for entry in self
                    .dictionary
                    .lookup(&spelling, 4)
                    .map_err(ConvertError::Dictionary)?
                    .iter()
                {
                    if matches!(entry.part_of_speech.as_str(), "english" | "slang-en")
                        && entry.surface.is_ascii()
                        && !entries
                            .iter()
                            .any(|old: &crate::Entry| old.surface == entry.surface)
                    {
                        entries.push(entry.clone());
                    }
                }
            }
            entries.sort_by(|a, b| {
                b.frequency
                    .cmp(&a.frequency)
                    .then_with(|| a.surface.cmp(&b.surface))
            });
            entries.truncate(2);
            if entries.is_empty() {
                continue;
            }
            let suffix = self.convert(tail, 8)?;
            for entry in entries {
                for candidate in &suffix.candidates {
                    result.push(Candidate {
                        surface: format!("{}{}", entry.surface, candidate.surface),
                        score: candidate
                            .score
                            .saturating_add(nextime_ranker::word_score(entry.frequency))
                            .saturating_sub(512),
                        segments: candidate.segments.saturating_add(1),
                    });
                }
            }
        }
        nextime_ranker::rank(&mut result, MAX_CANDIDATES);
        Ok(result)
    }
}
