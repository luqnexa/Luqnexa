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

#[test]
fn neural_path_can_rank_dictionary_verified_typo_candidates_without_an_llm() {
    let model =
        NeuralRanker::from_bytes(&luqnexa_ml::Trainer::new(3).quantize().to_bytes()).unwrap();
    let result = Engine::bundled()
        .convert_with_neural("pythpn", "コードを書く", &model, 32)
        .unwrap();
    assert!(
        result
            .candidates
            .iter()
            .any(|c| c.surface.eq_ignore_ascii_case("Python"))
    );
    assert!(
        result
            .candidates
            .iter()
            .any(|c| c.surface == result.reading)
    );
    assert!(result.candidates.len() <= 32);
}

#[test]
fn explicit_neural_path_offers_registered_case_without_overriding_shift() {
    let model =
        NeuralRanker::from_bytes(&luqnexa_ml::Trainer::new(2).quantize().to_bytes()).unwrap();
    let engine = Engine::bundled();
    let result = engine
        .convert_with_neural("githubde", "", &model, 32)
        .unwrap();
    assert!(result.candidates.iter().any(|c| c.surface == "GitHubで"));
    for input in ["github", "GITHUBde"] {
        let result = engine.convert_with_neural(input, "", &model, 32).unwrap();
        assert!(
            !result
                .candidates
                .iter()
                .any(|c| c.surface.contains("GitHub"))
        );
    }
}
