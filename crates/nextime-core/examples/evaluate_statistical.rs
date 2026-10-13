// SPDX-License-Identifier: AGPL-3.0-only
//! Isolated full-conversion comparison; reference text is never an input to search.
#[cfg(feature = "mobile-neural")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use nextime_convert::statistical::{self, ConnectionCosts};
    use nextime_core::{Engine, NeuralRanker, WithCommonPredictions};
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    use std::{
        io::{BufReader, Read},
        sync::Arc,
        time::Instant,
    };
    fn hash(path: &str) -> std::io::Result<String> {
        let mut file = std::fs::File::open(path)?;
        let mut sha = Sha256::new();
        let mut chunk = [0; 8192];
        loop {
            let n = file.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            sha.update(&chunk[..n]);
        }
        Ok(format!("{:x}", sha.finalize()))
    }
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 7 || !["train", "validation", "test"].contains(&args[2].as_str()) {
        return Err("usage: evaluate_statistical MATRIX.txt DATA.jsonl SPLIT BASE.nud OVERLAY.nxl FORMS.nud MODEL.lqml".into());
    }
    let costs = ConnectionCosts::from_text(BufReader::new(std::fs::File::open(&args[0])?))?;
    let source = Arc::new(WithCommonPredictions::new(
        nextime_dict::SupplementedDictionary::new(
            nexc_lite::LiteDictionary::open(&args[3], &args[4])?,
            nexc_lite::FileDictionary::open_with_cache(&args[5], 32)?,
        ),
    ));
    let engine = Engine::new(source.clone());
    let neural = NeuralRanker::from_bytes(&std::fs::read(&args[6])?)?;
    let mut rows = Vec::new();
    let mut totals = [0; 3];
    let mut pools = [0; 3];
    let mut timings: [Vec<u128>; 3] = std::array::from_fn(|_| Vec::new());
    for line in std::fs::read_to_string(&args[1])?
        .lines()
        .filter(|s| !s.trim().is_empty())
    {
        let row: Value = serde_json::from_str(line)?;
        if row["split"].as_str() != Some(&args[2]) {
            continue;
        }
        let input = row["input"].as_str().ok_or("input missing")?;
        let context = row["context"].as_str().unwrap_or("");
        let reference = row["good"].as_str().ok_or("reference missing")?;
        let mut modes = Vec::new();
        for mode in 0..3 {
            let convert = || -> Result<Vec<nextime_core::Candidate>, nextime_core::ConvertError> {
                if mode == 0 {
                    return Ok(engine
                        .convert_with_neural(input, context, &neural, 32)?
                        .candidates);
                }
                let reading = engine.normalize_input(input, false);
                let mut candidates = statistical::convert(source.as_ref(), &costs, &reading, 32)?;
                if mode == 2 {
                    neural
                        .rerank(&reading, context, &mut candidates, 32)
                        .map_err(|reason| {
                            nextime_core::ConvertError::Dictionary(nextime_core::LookupError(
                                reason.into(),
                            ))
                        })?;
                }
                Ok(candidates)
            };
            let candidates = convert()?;
            for _ in 0..10 {
                let start = Instant::now();
                std::hint::black_box(convert()?);
                timings[mode].push(start.elapsed().as_nanos());
            }
            let correct = candidates.first().is_some_and(|c| c.surface == reference);
            let present = candidates.iter().any(|c| c.surface == reference);
            totals[mode] += usize::from(correct);
            pools[mode] += usize::from(present);
            modes.push(json!({"correct":correct,"reference_in_pool":present,"first":candidates.first().map(|c|&c.surface),
                "candidates":candidates.iter().map(|c|json!({"surface":c.surface,"score":c.score})).collect::<Vec<_>>()}));
        }
        rows.push(json!({"input":input,"context":context,"reference":reference,"modes":modes}));
    }
    if rows.is_empty() {
        return Err("empty evaluation split".into());
    }
    let metrics: Vec<_> = ["existing_neural", "statistical", "statistical_neural"]
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            timings[i].sort_unstable();
            json!({"mode":name,"top_one":totals[i],"examples":rows.len(),"reference_in_32":pools[i],
            "warm_p95_ms":timings[i][timings[i].len()*95/100] as f64/1e6})
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"partition":args[2],"metrics":metrics,"rows":rows,
        "matrix_sha256":hash(&args[0])?,"dataset_sha256":hash(&args[1])?,"dictionary_sha256":hash(&args[3])?,
        "overlay_sha256":hash(&args[4])?,"forms_sha256":hash(&args[5])?,"model_sha256":hash(&args[6])?,
        "matrix_dimension":costs.dimension(),"matrix_resident_bytes":costs.resident_bytes(),
        "installed":false,"training_during_evaluation":false,
        "scope":"Bounded batch conversion, warm cache Windows host. Public third-party connection costs, own Rust search. Not mobile, power or independent blind accuracy."}))?
    );
    Ok(())
}
#[cfg(not(feature = "mobile-neural"))]
fn main() {
    eprintln!("enable mobile-neural feature");
}
