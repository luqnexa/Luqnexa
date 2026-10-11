// SPDX-License-Identifier: AGPL-3.0-only
use nextime_core::Engine;
use std::{hint::black_box, time::Instant};

fn main() {
    let engine = Engine::bundled();
    let startup = Instant::now();
    let model = nextime_ai::Model::bundled();
    println!(
        "model_init_ms={:.4} features={}",
        startup.elapsed().as_secs_f64() * 1000.0,
        model.feature_count()
    );
    for enabled in [false, true] {
        let mut samples = Vec::new();
        for i in 0..2100 {
            let start = Instant::now();
            let result = if enabled {
                engine.convert_with_context("hashi", "食事に", 8)
            } else {
                engine.convert("hashi", 8)
            }
            .unwrap();
            black_box(result);
            if i >= 100 {
                samples.push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "ai={enabled} entries={} iterations=2000 warmup=100 p50_ms={:.4} p95_ms={:.4} max_ms={:.4}",
            engine.dictionary_len(),
            samples[999],
            samples[1899],
            samples[1999]
        );
    }
    let mut online = nextime_ai::Model::default();
    for line in include_str!("../modules/Luqnexa-AI/models/context-synthetic.txt")
        .lines()
        .filter(|s| !s.starts_with('#'))
    {
        online.learn_confirmed("", line);
    }
    for i in 0..10_000 {
        online.learn_confirmed("", &format!("{}例文", char::from_u32(0x4E00 + i).unwrap()));
    }
    let mut samples = Vec::new();
    for i in 0..2100 {
        let start = Instant::now();
        let mut result = engine.convert_with_context("hashi", "食事に", 8).unwrap();
        online.rerank_online("食事に", &mut result.candidates, 8);
        black_box(result);
        if i >= 100 {
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "ai=true online_features={} entries={} iterations=2000 warmup=100 p50_ms={:.4} p95_ms={:.4} max_ms={:.4}",
        online.feature_count(),
        engine.dictionary_len(),
        samples[999],
        samples[1899],
        samples[1999]
    );
}
