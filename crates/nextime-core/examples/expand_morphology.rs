// SPDX-License-Identifier: AGPL-3.0-only
//! Offline lexical expansion, not sentence memorization or neural training.
use sha2::{Digest, Sha256};
use std::{fs::OpenOptions, io::Write};
fn hash(path: &str) -> Result<String, std::io::Error> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut sum = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        sum.update(&buffer[..size]);
    }
    Ok(format!("{:x}", sum.finalize()))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: expand_morphology BASE.nud NEW.tsv".into());
    }
    let source_sha = hash(&args[0])?;
    let base = nexc_lite::FileDictionary::open(&args[0])?;
    // Validate all blocks before publishing a derived artifact.
    base.validate_all()?;
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    let mut output = std::io::BufWriter::new(file);
    let mut lemmas = 0;
    let mut forms = 0;
    base.for_each_entry(|entry| {
        let derived = nextime_convert::productive_forms(&entry);
        lemmas += usize::from(!derived.is_empty());
        for entry in derived {
            writeln!(
                output,
                "{}\t{}\t{}\t{}",
                entry.reading, entry.surface, entry.part_of_speech, entry.frequency
            )?;
            forms += 1;
        }
        Ok(())
    })?;
    output.flush()?;
    output.get_ref().sync_all()?;
    if source_sha != hash(&args[0])? {
        return Err("source changed during expansion".into());
    }
    println!(
        "{}",
        serde_json::json!({"source_sha256":source_sha,"output_sha256":hash(&args[1])?,"lemmas":lemmas,"generated_forms":forms,
        "installed":false,"neural_training":false,"sentences_used":false,"limits":"Productive lexical forms, not independently reviewed examples. Source metadata/weights retained; partial output must be discarded if the command fails."})
    );
    Ok(())
}
