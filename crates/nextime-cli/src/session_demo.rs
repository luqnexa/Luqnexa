// SPDX-License-Identifier: AGPL-3.0-only
use nextime_core::{Engine, Key, Session};
use std::{
    error::Error,
    io::{BufRead, Write},
};

pub fn run(
    engine: Engine,
    limit: usize,
    input: impl BufRead,
    output: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let mut session = Session::new(engine, limit)?;
    writeln!(
        output,
        "Luqnexa session: text VALUE / space / next / prev / backspace / enter / escape / ai / poll-ai / training / clear-context / clear-learning / quit"
    )?;
    for line in input.lines() {
        let line = line?;
        if line == "quit" {
            break;
        }
        let key = if let Some(text) = line.strip_prefix("text ") {
            Key::Text(text)
        } else {
            match line.as_str() {
                "space" => Key::Space,
                "poll-ai" => Key::PollAi,
                "literal" => Key::Literal,
                "mode" => Key::ToggleMode,
                "ai" => Key::ToggleAi,
                "clear-context" => Key::ClearContext,
                "clear-learning" => Key::ClearLearning,
                "training" => Key::ToggleTraining,
                "next" => Key::Next,
                "prev" => Key::Previous,
                "backspace" => Key::Backspace,
                "enter" => Key::Enter,
                "escape" => Key::Cancel,
                _ => return Err(format!("unknown session command: {line}").into()),
            }
        };
        let outcome = session.event(key)?;
        if !outcome.handled {
            writeln!(output, "pass-through")?;
        }
        if let Some(text) = outcome.committed {
            writeln!(output, "commit: {text}")?;
        }
        writeln!(output, "preedit: {}", session.display())?;
        for (index, candidate) in session.candidates().iter().enumerate() {
            let marker = if session.selected_index() == Some(index) {
                ">"
            } else {
                " "
            };
            writeln!(output, "{marker} {}. {}", index + 1, candidate.surface)?;
        }
        writeln!(
            output,
            "learning: {} features={}",
            if session.text_learning_enabled() && session.ai_enabled() {
                "on"
            } else {
                "off"
            },
            session.learned_text_features()
        )?;
        output.flush()?;
    }
    Ok(())
}
