// SPDX-License-Identifier: AGPL-3.0-only
use nexc_lite::{FileDictionary, LiteDictionary};
use nextime_core::{Dictionary, Engine};
mod commands;
mod session_demo;
use std::{
    error::Error,
    io::{self, BufRead, Write},
};

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--model-info") {
        let (model, anchors, research) = nextime_ai::native_model_identity();
        println!(
            "{{\"model_sha256\":\"{model}\",\"anchors_sha256\":\"{anchors}\",\"research_model\":{research}}}"
        );
        return;
    }
    if let Err(error) = run() {
        eprintln!("nextime: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1).peekable();
    if args.peek().is_some_and(|s| s == "dict") {
        args.next();
        return commands::run(args.collect());
    }
    let session_mode = args.peek().is_some_and(|s| s == "session");
    if session_mode {
        args.next();
    }
    let mut dictionary_path = None;
    let mut nud_path = None;
    let mut journal_path = None;
    let mut limit = 8;
    let mut input = None;
    let mut positional = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" if !positional => {
                println!(
                    "Luqnexa offline demo\nUsage: nextime [--dictionary FILE.tsv | --nud FILE.nud [--journal FILE.nxl]] [--limit 1..32] [--] [INPUT]\nOmit INPUT to convert stdin one line at a time.\nSession demo: nextime session [dictionary options] [--limit 1..32]\nSession stdin commands: text VALUE / space / next / prev / backspace / enter / escape / quit\nDictionary tools: nextime dict build|verify|unpack|add --help\nConversion input is never saved or transmitted; dict add explicitly saves a word locally."
                );
                return Ok(());
            }
            "--dictionary" if !positional => {
                dictionary_path = Some(args.next().ok_or("--dictionary requires a path")?)
            }
            "--nud" if !positional => nud_path = Some(args.next().ok_or("--nud requires a path")?),
            "--journal" if !positional => {
                journal_path = Some(args.next().ok_or("--journal requires a path")?)
            }
            "--limit" if !positional => {
                limit = args.next().ok_or("--limit requires a number")?.parse()?
            }
            "--" if !positional => positional = true,
            _ if !positional && arg.starts_with('-') => {
                return Err(format!("unknown option: {arg}").into());
            }
            _ => {
                if input.replace(arg).is_some() {
                    return Err("provide one quoted INPUT argument".into());
                }
            }
        }
    }
    if !(1..=nextime_core::MAX_CANDIDATES).contains(&limit) {
        return Err("--limit must be between 1 and 32".into());
    }
    if dictionary_path.is_some() && nud_path.is_some() {
        return Err("--dictionary and --nud are mutually exclusive".into());
    }
    if journal_path.is_some() && nud_path.is_none() {
        return Err("--journal requires --nud".into());
    }
    let engine = if let Some(path) = nud_path {
        if let Some(journal) = journal_path {
            Engine::new(nextime_core::WithCommonPredictions::new(
                LiteDictionary::open(path, journal)?,
            ))
        } else {
            Engine::new(nextime_core::WithCommonPredictions::new(
                FileDictionary::open(path)?,
            ))
        }
    } else if let Some(path) = dictionary_path {
        Engine::new(Dictionary::from_tsv(&std::fs::read_to_string(path)?)?)
    } else {
        Engine::bundled()
    };
    let stdout = io::stdout();
    let mut output = stdout.lock();
    if session_mode {
        if input.is_some() {
            return Err("session accepts stdin commands, not an INPUT argument".into());
        }
        return session_demo::run(engine, limit, io::stdin().lock(), &mut output);
    }
    if let Some(input) = input {
        print_conversion(&engine, &input, limit, &mut output)?;
    } else {
        for line in io::stdin().lock().lines() {
            print_conversion(&engine, &line?, limit, &mut output)?;
        }
    }
    Ok(())
}

fn print_conversion(
    engine: &Engine,
    input: &str,
    limit: usize,
    output: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let result = engine.convert(input, limit)?;
    writeln!(output, "reading: {}", result.reading)?;
    for (index, candidate) in result.candidates.iter().enumerate() {
        writeln!(
            output,
            "{}. {}\t(score={}, segments={})",
            index + 1,
            candidate.surface,
            candidate.score,
            candidate.segments
        )?;
    }
    Ok(())
}
