// SPDX-License-Identifier: AGPL-3.0-only
//! Explicit experimental statistical lattice. No network, training, or installation.
//! Connection data is caller-supplied; this implements its own bounded search.
use crate::{ConvertError, MAX_CANDIDATES, MAX_INPUT_CHARS};
use nextime_dict::{DictionarySource, Entry, LookupError};
use nextime_ranker::{Candidate, rank, word_score};
use std::io::{self, BufRead};

#[derive(Debug)]
pub struct ConnectionCosts {
    size: usize,
    // Dense diagnostic representation, not a compact mobile deployment format.
    costs: Box<[i16]>,
}
impl ConnectionCosts {
    /// Text contains dimension followed by row-major costs: previous right ID,
    /// then current left ID. Require exactly size² signed 16-bit cells.
    pub fn from_text(mut input: impl BufRead) -> io::Result<Self> {
        let invalid = || io::Error::new(io::ErrorKind::InvalidData, "invalid connection matrix");
        let mut line = String::new();
        input.read_line(&mut line)?;
        let size: usize = line.trim().parse().map_err(|_| invalid())?;
        if !(1..=4096).contains(&size) {
            return Err(invalid());
        }
        let cells = size * size;
        let mut costs = Vec::with_capacity(cells);
        for _ in 0..cells {
            line.clear();
            if input.read_line(&mut line)? == 0 {
                return Err(invalid());
            }
            costs.push(line.trim().parse::<i16>().map_err(|_| invalid())?);
        }
        line.clear();
        if input.read_line(&mut line)? != 0 {
            return Err(invalid());
        }
        Ok(Self {
            size,
            costs: costs.into_boxed_slice(),
        })
    }
    pub fn dimension(&self) -> usize {
        self.size
    }
    pub fn resident_bytes(&self) -> usize {
        self.costs.len() * 2
    }
    pub fn cost(&self, previous_right: usize, current_left: usize) -> Option<i64> {
        if previous_right >= self.size || current_left >= self.size {
            return None;
        }
        Some(i64::from(
            self.costs[previous_right * self.size + current_left],
        ))
    }
}

#[derive(Clone)]
struct Path {
    surface: String,
    cost: i64,
    right: usize,
    segments: usize,
}
fn prune(paths: &mut Vec<Path>, limit: usize) {
    paths.sort_unstable_by(|a, b| {
        a.cost
            .cmp(&b.cost)
            .then_with(|| a.segments.cmp(&b.segments))
            .then_with(|| a.surface.cmp(&b.surface))
            .then_with(|| a.right.cmp(&b.right))
    });
    let mut seen = std::collections::HashSet::new();
    paths.retain(|p| seen.insert((p.surface.clone(), p.right)));
    paths.truncate(limit);
}
fn ids(entry: &Entry, size: usize, noun: usize) -> Result<(usize, usize, i64), ConvertError> {
    if !entry.part_of_speech.starts_with("mozc:") {
        return Ok((noun, noun, -word_score(entry.frequency)));
    }
    let mut fields = entry.part_of_speech.split(':').skip(1);
    let left = fields.next().and_then(|s| s.parse::<usize>().ok());
    let right = fields.next().and_then(|s| s.parse::<usize>().ok());
    let cost = fields.next().and_then(|s| s.parse::<i32>().ok());
    match (left, right, cost) {
        (Some(left), Some(right), Some(cost)) if left < size && right < size => {
            Ok((left, right, i64::from(cost)))
        }
        _ => Err(ConvertError::Dictionary(LookupError(
            "dictionary IDs do not match connection model".into(),
        ))),
    }
}
fn append(
    target: &mut Vec<Path>,
    prefix: &Path,
    surface: &str,
    left: usize,
    right: usize,
    word_cost: i64,
    costs: &ConnectionCosts,
) {
    let mut text = prefix.surface.clone();
    text.push_str(surface);
    target.push(Path {
        surface: text,
        cost: prefix.cost + word_cost + costs.cost(prefix.right, left).unwrap(),
        right,
        segments: prefix.segments + 1,
    });
    if target.len() >= 256 {
        prune(target, 128);
    }
}
/// Approximate N-best search. No hard grammatical exclusions; statistical costs
/// include BOS/EOS. Supplied metadata must belong to the same matrix version.
pub fn convert(
    dictionary: &(impl DictionarySource + ?Sized),
    costs: &ConnectionCosts,
    reading: &str,
    limit: usize,
) -> Result<Vec<Candidate>, ConvertError> {
    convert_with_entered(dictionary, costs, reading, "", limit)
}
/// Raw input is used only to identify already entered non-ASCII spans containing
/// ideographs. Romaji-derived kana is never treated as already converted text.
pub fn convert_with_entered(
    dictionary: &(impl DictionarySource + ?Sized),
    costs: &ConnectionCosts,
    reading: &str,
    raw_input: &str,
    limit: usize,
) -> Result<Vec<Candidate>, ConvertError> {
    if raw_input.len() > MAX_INPUT_CHARS * 4 {
        return Err(ConvertError::InputTooLong);
    }
    let entered: Vec<_> = raw_input
        .split(|c: char| c.is_ascii())
        .filter(|text| {
            text.chars()
                .any(|c| matches!(c as u32, 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0x20000..=0x2fa1f))
        })
        .collect();
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
    let noun = include_str!("../../../dictionaries/mozc-id.def")
        .lines()
        .position(|line| {
            line.split_once(' ')
                .is_some_and(|(_, p)| p.starts_with("名詞,一般,"))
        })
        .filter(|&id| id < costs.size)
        .unwrap_or(0);
    let mut beams = vec![Vec::<Path>::new(); count + 1];
    beams[0].push(Path {
        surface: String::new(),
        cost: 0,
        right: 0,
        segments: 0,
    });
    for start in 0..count {
        prune(&mut beams[start], 128);
        let prefixes = std::mem::take(&mut beams[start]);
        let last = (start + dictionary.max_reading_chars()).min(count);
        let slices: Vec<_> = (start + 1..=last)
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
        for (offset, entries) in batches.iter().enumerate() {
            for entry in entries {
                let (left, right, cost) = ids(entry, costs.size, noun)?;
                for prefix in &prefixes {
                    append(
                        &mut beams[start + 1 + offset],
                        prefix,
                        &entry.surface,
                        left,
                        right,
                        cost,
                        costs,
                    );
                }
            }
        }
        // Retain ASCII as a single run rather than interpreting each letter as a token.
        let tail = &reading[boundaries[start]..];
        // Entered ideographs are not phonetic dictionary keys. Keep their
        // contiguous non-ASCII span as an edge, including following kana.
        for text in entered.iter().filter(|text| tail.starts_with(**text)) {
            let entered_length = text.chars().count();
            for prefix in &prefixes {
                append(
                    &mut beams[start + entered_length],
                    prefix,
                    text,
                    noun,
                    noun,
                    6000,
                    costs,
                );
            }
        }
        if tail.starts_with(|c: char| c.is_ascii_alphabetic()) {
            let length = tail.chars().take_while(|c| c.is_ascii_alphabetic()).count();
            let text = &reading[boundaries[start]..boundaries[start + length]];
            for prefix in &prefixes {
                append(
                    &mut beams[start + length],
                    prefix,
                    text,
                    noun,
                    noun,
                    6000,
                    costs,
                );
            }
        }
        let literal = &reading[boundaries[start]..boundaries[start + 1]];
        for prefix in &prefixes {
            append(&mut beams[start + 1], prefix, literal, 0, 0, 12000, costs);
        }
    }
    let mut result: Vec<_> = beams[count]
        .iter()
        .map(|path| Candidate {
            surface: path.surface.clone(),
            score: -(path.cost + costs.cost(path.right, 0).unwrap()),
            segments: path.segments,
        })
        .collect();
    rank(&mut result, limit);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matrix_is_directional_and_rejects_incomplete_or_excess_data() {
        let model = ConnectionCosts::from_text(&b"2\n0\n5\n19\n3\n"[..]).unwrap();
        assert_eq!(model.cost(0, 1), Some(5));
        assert_eq!(model.cost(1, 0), Some(19));
        assert_eq!(model.cost(2, 0), None);
        for bad in ["0\n", "4097\n", "2\n0\n", "1\n32768\n", "1\n0\n1\n"] {
            assert!(ConnectionCosts::from_text(bad.as_bytes()).is_err());
        }
    }
    #[test]
    fn connection_and_eos_costs_can_reverse_frequency_order() {
        let dict = nextime_dict::Dictionary::from_tsv(
            "あ\t甲\tmozc:1:1:10\t100\nあ\t乙\tmozc:2:2:20\t1\nい\t丙\tmozc:2:2:10\t100\n",
        )
        .unwrap();
        let model = ConnectionCosts::from_text(&b"3\n0\n0\n0\n0\n0\n100\n0\n0\n0\n"[..]).unwrap();
        let result = convert(&dict, &model, "あい", 2).unwrap();
        assert_eq!(result[0].surface, "乙丙");
        assert_eq!(result[0].score, -30);
        assert_eq!(convert(&dict, &model, "あ", 2).unwrap()[0].surface, "甲");
        let eos = ConnectionCosts::from_text(&b"3\n0\n0\n0\n100\n0\n0\n0\n0\n0\n"[..]).unwrap();
        assert_eq!(convert(&dict, &eos, "あ", 2).unwrap()[0].surface, "乙");
        assert!(convert(&dict, &model, "あ", 0).is_err());
        assert!(convert(&dict, &model, &"あ".repeat(129), 1).is_err());
    }
    #[test]
    fn unknown_unicode_survives_and_mismatched_ids_fail() {
        let dict = nextime_dict::Dictionary::from_tsv("あ\t甲\tmozc:9:9:10\t100\n").unwrap();
        let model = ConnectionCosts::from_text(&b"1\n0\n"[..]).unwrap();
        assert!(convert(&dict, &model, "あ", 1).is_err());
        assert_eq!(convert(&dict, &model, "😊", 1).unwrap()[0].surface, "😊");
    }
    #[test]
    fn entered_ideograph_spans_remain_available_without_rewriting_kana_input() {
        let dictionary = nextime_dict::Dictionary::from_tsv("る\t流\tmozc:0:0:10\t100\n").unwrap();
        let model = ConnectionCosts::from_text(&b"1\n0\n"[..]).unwrap();
        assert_eq!(
            convert_with_entered(&dictionary, &model, "調べる", "調べる", 32).unwrap()[0].surface,
            "調べる"
        );
        assert_eq!(
            convert(&dictionary, &model, "る", 32).unwrap()[0].surface,
            "流"
        );
        assert_eq!(
            convert_with_entered(&dictionary, &model, "調べるRust", "調べるRust", 32).unwrap()[0]
                .surface,
            "調べるRust"
        );
    }
}
