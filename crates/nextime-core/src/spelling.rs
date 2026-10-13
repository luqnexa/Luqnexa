// SPDX-License-Identifier: AGPL-3.0-only
//! Deterministic dictionary-verified spelling proposals; no model or training.
pub fn typo_readings(input: &str) -> Vec<String> {
    if !(4..=24).contains(&input.len())
        || !input.bytes().any(|b| b.is_ascii_lowercase())
        || !input
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b";[]'".contains(&b))
    {
        return Vec::new();
    }
    const ROWS: &[(&str, f32)] = &[
        ("qwertyuiop[]", 0.0),
        ("asdfghjkl;'", 0.25),
        ("zxcvbnm", 0.75),
    ];
    let keys: Vec<_> = ROWS
        .iter()
        .enumerate()
        .flat_map(|(y, (row, offset))| {
            row.bytes()
                .enumerate()
                .map(move |(x, b)| (b, x as f32 + offset, y as i32))
        })
        .collect();
    let bytes = input.as_bytes();
    let mut result = Vec::new();
    // A hand starting one key to the left/right produces a whole-row shift.
    // Offer both inverse shifts only when every key has a valid neighbor.
    for direction in [-1isize, 1] {
        let shifted: Option<String> = bytes
            .iter()
            .map(|&b| {
                ROWS.iter().find_map(|(row, _)| {
                    row.bytes().position(|key| key == b).and_then(|index| {
                        row.as_bytes()
                            .get(index.checked_add_signed(direction)?)
                            .copied()
                            .map(char::from)
                    })
                })
            })
            .collect();
        if let Some(shifted) = shifted {
            result.push(shifted);
        }
    }
    // Enumerate a single omitted Latin letter at every position, including the
    // beginning/end. No vocabulary is assumed here: callers verify dictionary
    // entries and preserve the literal. Existing batched lookups bound I/O.
    if bytes.iter().all(u8::is_ascii_lowercase) {
        for position in 0..=bytes.len() {
            for letter in b'a'..=b'z' {
                let mut inserted = Vec::with_capacity(bytes.len() + 1);
                inserted.extend_from_slice(&bytes[..position]);
                inserted.push(letter);
                inserted.extend_from_slice(&bytes[position..]);
                result.push(String::from_utf8(inserted).unwrap());
            }
        }
    }
    for i in 0..bytes.len() {
        // Extra letter or accidental repeated key; dictionary-checked upstream.
        let mut deleted = bytes.to_vec();
        deleted.remove(i);
        result.push(String::from_utf8(deleted).unwrap());
        if i + 1 < bytes.len() && bytes[i] != bytes[i + 1] {
            let mut changed = bytes.to_vec();
            changed.swap(i, i + 1);
            result.push(String::from_utf8(changed).unwrap());
        }
        let (_, x, y) = keys.iter().find(|(b, _, _)| *b == bytes[i]).unwrap();
        for &(b, other_x, other_y) in &keys {
            if b != bytes[i] && (other_y - y).abs() <= 1 && (other_x - x).abs() <= 1.05 {
                let mut changed = bytes.to_vec();
                changed[i] = b;
                result.push(String::from_utf8(changed).unwrap());
            }
        }
    }
    result.sort();
    result.dedup();
    result
}
