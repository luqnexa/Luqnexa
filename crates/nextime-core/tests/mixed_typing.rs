// SPDX-License-Identifier: AGPL-3.0-only
use nextime_core::{Dictionary, Engine};

#[test]
fn dictionary_verified_prefix_typo_keeps_japanese_suffix_and_literal() {
    let dictionary = Dictionary::from_tsv(
        "harbor\tHarbor\tenglish\t10000\nで\tで\tparticle\t10000\nつくる\t作る\tverb\t10000\n",
    )
    .unwrap();
    let engine = Engine::new(dictionary);
    let result = engine
        .convert_with_context("harbrpdetukuru", "", 32)
        .unwrap();
    // This is not a valid one-edit spelling and must not invent a correction.
    assert!(
        !result
            .candidates
            .iter()
            .any(|c| c.surface == "Harborで作る")
    );
    let result = engine
        .convert_with_context("harbrodetsukuru", "", 32)
        .unwrap();
    assert!(
        result
            .candidates
            .iter()
            .any(|c| c.surface == "Harborで作る")
    );
    assert!(
        result
            .candidates
            .iter()
            .any(|c| c.surface == result.reading)
    );
    for input in ["Harbrodetsukuru", "harbro🙂", "Harbor"] {
        let result = engine.convert_with_context(input, "", 32).unwrap();
        assert!(
            !result
                .candidates
                .iter()
                .any(|c| c.surface == "Harborで作る")
        );
    }
}
