// SPDX-License-Identifier: AGPL-3.0-only
use nexc_lite::{FileDictionary, LiteDictionary};
use nextime_core::{Dictionary, DictionarySource, Entry};
use std::{
    error::Error,
    io::{self, Write},
};

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.is_empty() || args.iter().any(|s| s == "--help" || s == "-h") {
        println!(
            "Dictionary commands:\n  nextime dict build INPUT.tsv OUTPUT.nud [GENERATION]\n  nextime dict verify FILE.nud\n  nextime dict unpack FILE.nud  (TSV to stdout)\n  nextime dict add BASE.nud JOURNAL.nxl READING SURFACE POS FREQUENCY\nBuild refuses to overwrite an existing output. Add stores only the explicitly supplied word locally."
        );
        return Ok(());
    }
    match args[0].as_str() {
        "build" if args.len() == 3 || args.len() == 4 => {
            let dict = Dictionary::from_tsv(&std::fs::read_to_string(&args[1])?)?;
            let generation = if args.len() == 4 { args[3].parse()? } else { 1 };
            let stats = nexc_max::build_dictionary(&args[2], &dict, generation)?;
            println!(
                "entries={}; blocks={}; bytes={}; index_bytes={}; generation={generation}",
                stats.entries, stats.blocks, stats.bytes, stats.index_bytes
            );
        }
        "verify" if args.len() == 2 => {
            let dict = FileDictionary::open(&args[1])?;
            dict.validate_all()?;
            println!(
                "valid NUD v1; entries={}; blocks={}; generation={}; index_memory_estimate_bytes={}",
                dict.len(),
                dict.block_count(),
                dict.generation(),
                dict.index_memory_bytes()
            );
        }
        "unpack" if args.len() == 2 => {
            let dict = FileDictionary::open(&args[1])?;
            // Validate before emitting so a damaged later block cannot produce
            // seemingly complete TSV output; no output file is created here.
            dict.validate_all()?;
            let stdout = io::stdout();
            let mut output = stdout.lock();
            dict.for_each_entry(|e| {
                writeln!(
                    output,
                    "{}\t{}\t{}\t{}",
                    e.reading, e.surface, e.part_of_speech, e.frequency
                )
            })?;
        }
        "add" if args.len() == 7 => {
            let mut dict = LiteDictionary::open(&args[1], &args[2])?;
            dict.insert(Entry {
                reading: args[3].clone(),
                surface: args[4].clone(),
                part_of_speech: args[5].clone(),
                frequency: args[6].parse()?,
            })?;
            println!(
                "saved locally; overlay_entries={}; journal_bytes={}",
                dict.overlay_len(),
                dict.journal_bytes()?
            );
        }
        _ => return Err("invalid dictionary command; use nextime dict --help".into()),
    }
    Ok(())
}
