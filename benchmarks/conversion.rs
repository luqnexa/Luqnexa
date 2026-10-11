// SPDX-License-Identifier: AGPL-3.0-only
//! Dependency-free executable benchmark; timings include the entire Engine call.
use nextime_core::{Dictionary, Engine, Entry};
use std::{hint::black_box, time::Instant};

fn main() {
    if let Err(error) = run() {
        eprintln!("benchmark: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut entries = 10_000_usize;
    let mut iterations = 2_000_usize;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--entries" => entries = args.next().ok_or("missing --entries value")?.parse()?,
            "--iterations" => {
                iterations = args.next().ok_or("missing --iterations value")?.parse()?
            }
            "--bench" => {} // Cargo supplies this for custom harnesses.
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
    if entries == 0 || iterations == 0 {
        return Err("entries and iterations must be positive".into());
    }
    println!(
        "NextIME Phase 1; OS={}; arch={}; iterations={iterations}; limit=8; warmup=100",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let sample = Engine::bundled();
    println!("sample_entries={}", sample.dictionary_len());
    measure("sample/repeated_hit", &sample, iterations, |_| {
        "nihongo".into()
    });
    let mixed = [
        "hashi",
        "kisha",
        "konnichiha",
        "nihongonyuuryoku",
        "🙂xyz",
        "せんせいにれんらく",
        "kyou",
    ];
    measure("sample/mixed", &sample, iterations, |i| {
        mixed[i % mixed.len()].into()
    });
    measure(
        "sample/long_128_chars",
        &sample,
        iterations.min(200),
        |_| "は".repeat(128),
    );

    let build = Instant::now();
    let generated = (0..entries)
        .map(|i| Entry {
            reading: format!("ごい{i:09}"),
            surface: format!("語彙{i:09}"),
            part_of_speech: "noun".into(),
            frequency: 1000 + (i % 10000) as u32,
        })
        .collect();
    let engine = Engine::new(Dictionary::from_entries(generated)?);
    println!(
        "synthetic_entries={}; build_ms={:.3}; synthetic_data=true; storage=in_memory",
        engine.dictionary_len(),
        build.elapsed().as_secs_f64() * 1000.0
    );
    measure("synthetic/repeated_hit", &engine, iterations, |_| {
        "goi000000000".into()
    });
    measure("synthetic/varying_hit", &engine, iterations, |i| {
        format!("goi{:09}", i % entries)
    });
    measure("synthetic/lookup_miss", &engine, iterations, |_| {
        "みとうろく🙂".into()
    });
    Ok(())
}

fn measure(label: &str, engine: &Engine, iterations: usize, input: impl Fn(usize) -> String) {
    for i in 0..100 {
        black_box(engine.convert(&input(i), 8).unwrap());
    }
    let mut timings = Vec::with_capacity(iterations);
    for i in 0..iterations {
        // Workload construction is excluded; parsing is included.
        let query = input(i);
        let start = Instant::now();
        black_box(engine.convert(black_box(&query), 8).unwrap());
        timings.push(start.elapsed().as_nanos());
    }
    let total: u128 = timings.iter().sum();
    timings.sort_unstable();
    let percentile =
        |percent: usize| timings[(iterations * percent).div_ceil(100) - 1] as f64 / 1_000_000.0;
    println!(
        "{label}: n={iterations}; p50_ms={:.6}; p95_ms={:.6}; p99_ms={:.6}; max_ms={:.6}; mean_ms={:.6}",
        percentile(50),
        percentile(95),
        percentile(99),
        *timings.last().unwrap() as f64 / 1_000_000.0,
        total as f64 / iterations as f64 / 1_000_000.0
    );
}
