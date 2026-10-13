// SPDX-License-Identifier: AGPL-3.0-only
//! Offline full conversion evaluation. References never train the model.
#[cfg(feature = "mobile-neural")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use nextime_core::{Engine, NeuralRanker, WithCommonPredictions};
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    use std::{io::Read, time::Instant};
    fn hash(path: &str) -> Result<String, std::io::Error> {
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut chunk = [0; 8192];
        loop {
            let size = file.read(&mut chunk)?;
            if size == 0 {
                break;
            }
            hasher.update(&chunk[..size]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    let mut temperature = None;
    if args.len() >= 6 && args[args.len() - 2] == "--temperature" {
        temperature = Some(args.pop().unwrap().parse::<f32>()?);
        args.pop();
    }
    if !(4..=5).contains(&args.len())
        || !["train", "validation", "test"].contains(&args[2].as_str())
    {
        return Err(
            "usage: evaluate_neural MODEL JSONL SPLIT BASE.nud [OVERLAY.nxl] [--temperature FLOAT]"
                .into(),
        );
    }
    let bytes = std::fs::read(&args[0])?;
    let model = if let Some(temperature) = temperature {
        NeuralRanker::from_bytes_with_temperature(&bytes, temperature)?
    } else {
        NeuralRanker::from_bytes(&bytes)?
    };
    let engine = if let Some(overlay) = args.get(4) {
        Engine::new(WithCommonPredictions::new(nexc_lite::LiteDictionary::open(
            &args[3], overlay,
        )?))
    } else {
        Engine::new(WithCommonPredictions::new(
            nexc_lite::FileDictionary::open_with_cache(&args[3], 32)?,
        ))
    };
    let mut rows = Vec::new();
    let mut totals = [0; 3];
    let mut coverage = [0; 3];
    let mut samples: [Vec<u128>; 3] = std::array::from_fn(|_| Vec::new());
    for line in std::fs::read_to_string(&args[1])?
        .lines()
        .filter(|s| !s.trim().is_empty())
    {
        let record: Value = serde_json::from_str(line)?;
        if record["split"].as_str() != Some(&args[2]) {
            continue;
        }
        let input = record["input"].as_str().ok_or("missing input")?;
        let context = record["context"].as_str().unwrap_or_default();
        let reference = record["good"].as_str().ok_or("missing reference")?;
        let mut results = Vec::new();
        for mode in 0..3 {
            let convert = || match mode {
                0 => engine.convert(input, 32),
                1 => engine.convert_with_context(input, context, 32),
                _ => engine.convert_with_neural(input, context, &model, 32),
            };
            let result = convert()?;
            for _ in 0..10 {
                let start = Instant::now();
                std::hint::black_box(convert()?);
                samples[mode].push(start.elapsed().as_nanos());
            }
            let correct = result
                .candidates
                .first()
                .is_some_and(|c| c.surface == reference);
            let included = result.candidates.iter().any(|c| c.surface == reference);
            totals[mode] += usize::from(correct);
            coverage[mode] += usize::from(included);
            results.push(json!({"correct":correct, "reference_in_pool":included,
                "first":result.candidates.first().map(|c| &c.surface),
                "candidates":result.candidates.iter().map(|c| json!({"surface":c.surface,"score":c.score})).collect::<Vec<_>>() }));
        }
        rows.push(json!({"input":input,"context":context,"reference":reference,"modes":results}));
    }
    if rows.is_empty() {
        return Err("empty selected partition".into());
    }
    let mut metrics = Vec::new();
    for (mode, name) in ["dictionary", "existing_context", "rust_neural"]
        .into_iter()
        .enumerate()
    {
        samples[mode].sort_unstable();
        metrics.push(json!({"mode":name,"top_one":totals[mode],"examples":rows.len(),
            "top_one_percent":totals[mode] as f64 *100.0 / rows.len() as f64,
            "reference_in_32":coverage[mode],"warm_p95_ms": samples[mode][samples[mode].len()*95/100] as f64 /1_000_000.0}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "model_sha256": hash(&args[0])?, "dataset_sha256":hash(&args[1])?, "dictionary_sha256":hash(&args[3])?,
            "overlay_sha256": args.get(4).map(|p| hash(p)).transpose()?,
            "partition":args[2],"metrics":metrics,"rows":rows,
            "neural_ranking_temperature":temperature,
            "scope":"Real offline dictionary candidate generation and ranking; batch conversion, warm cache, synthetic context. Not key-by-key Session, phone, power, or general accuracy.",
            "training_during_evaluation":false,"installed":false,
            "host_os":std::env::consts::OS,"host_arch":std::env::consts::ARCH
        }))?
    );
    Ok(())
}
#[cfg(not(feature = "mobile-neural"))]
fn main() {
    eprintln!("enable --features mobile-neural");
    std::process::exit(1);
}
