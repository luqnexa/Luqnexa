// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(feature = "mobile-neural")]
use nextime_core::{ConvertError, Engine, NeuralRanker};

#[test]
fn explicit_neural_path_retains_the_real_dictionary_candidate_pool() {
    let bytes = luqnexa_ml::Trainer::new(1337).quantize().to_bytes();
    let model = NeuralRanker::from_bytes(&bytes).unwrap();
    let engine = Engine::bundled();
    let baseline = engine.convert("hashi", 32).unwrap();
    let result = engine
        .convert_with_neural("hashi", "夕食の食器を並べる", &model, 32)
        .unwrap();
    assert_eq!(baseline.reading, result.reading);
    assert_eq!(baseline.candidates.len(), result.candidates.len());
    for candidate in &result.candidates {
        let old = baseline
            .candidates
            .iter()
            .find(|c| c.surface == candidate.surface)
            .unwrap();
        assert!((candidate.score - old.score).abs() <= 2048);
    }
    assert_eq!(
        engine.convert_with_neural("hashi", "", &model, 0),
        Err(ConvertError::InvalidLimit)
    );
    assert_eq!(
        engine.convert_with_neural("hashi", "", &model, 33),
        Err(ConvertError::InvalidLimit)
    );
    assert_eq!(
        engine
            .convert_with_neural("hashi", "", &model, 1)
            .unwrap()
            .candidates
            .len(),
        1
    );
}
