// SPDX-License-Identifier: AGPL-3.0-only
//! Explicit research demo; never changes installed IME settings or files.
#[cfg(feature = "mobile-neural")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: mobile_neural MODEL INPUT [CONTEXT]")?;
    let input = args.next().ok_or("missing input")?;
    let context = args.next().unwrap_or_default();
    let model = nextime_core::NeuralRanker::from_bytes(&std::fs::read(path)?)?;
    let engine = nextime_core::Engine::bundled();
    let result = engine.convert_with_neural(&input, &context, &model, 8)?;
    println!(
        "reading: {} / model resident bytes: {}",
        result.reading,
        model.resident_bytes()
    );
    for candidate in result.candidates {
        println!("{}\t{}", candidate.score, candidate.surface);
    }
    Ok(())
}
#[cfg(not(feature = "mobile-neural"))]
fn main() {
    eprintln!("enable --features mobile-neural");
    std::process::exit(1);
}
