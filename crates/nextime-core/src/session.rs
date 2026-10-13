// SPDX-License-Identifier: AGPL-3.0-only
//! Host-independent composition state. Only explicit Enter produces committed text.
use crate::{Candidate, ConvertError, Engine, MAX_CANDIDATES, MAX_INPUT_CHARS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key<'a> {
    Text(&'a str),
    Backspace,
    Convert,
    Next,
    Previous,
    Enter,
    Cancel,
    Literal,
    FullWidth,
    HalfWidth,
    ToggleMode,
    ToggleAi,
    ClearContext,
    ClearLearning,
    ToggleTraining,
    Space,
    PollAi,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// False means the host should pass this key to the application.
    pub handled: bool,
    /// Host may insert this text, but must never send a message automatically.
    pub committed: Option<String>,
}

#[derive(Debug)]
pub enum SessionError {
    Conversion(ConvertError),
    InvalidText,
}
impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conversion(e) => e.fmt(f),
            Self::InvalidText => {
                f.write_str("text must be nonempty and contain no control characters")
            }
        }
    }
}
impl std::error::Error for SessionError {}
impl From<ConvertError> for SessionError {
    fn from(e: ConvertError) -> Self {
        Self::Conversion(e)
    }
}

#[derive(Debug)]
pub struct Session {
    engine: Engine,
    limit: usize,
    raw: String,
    preedit: String,
    candidates: Vec<Candidate>,
    selected: usize,
    selecting: bool,
    english: bool,
}

impl Session {
    pub fn new(engine: Engine, limit: usize) -> Result<Self, SessionError> {
        if !(1..=MAX_CANDIDATES).contains(&limit) {
            return Err(ConvertError::InvalidLimit.into());
        }
        Ok(Self {
            engine,
            limit,
            raw: String::new(),
            preedit: String::new(),
            candidates: Vec::new(),
            selected: 0,
            selecting: false,
            english: false,
        })
    }
    pub fn raw(&self) -> &str {
        &self.raw
    }
    pub fn preedit(&self) -> &str {
        &self.preedit
    }
    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }
    /// Read-only score diagnostic. Only selected Japanese conversions have the
    /// native model contribution; explicit English input remains literal.
    pub fn native_candidate_bonus(&self, index: usize) -> Option<i64> {
        self.candidates.get(index).map(|_| 0)
    }
    pub fn selected_index(&self) -> Option<usize> {
        if self.candidates.is_empty() || !self.selecting {
            None
        } else {
            Some(self.selected)
        }
    }
    /// Host draws this as preedit; it is not committed to the document.
    pub fn display(&self) -> &str {
        if !self.selecting {
            return &self.preedit;
        }
        self.candidates
            .get(self.selected)
            .map_or(&self.preedit, |c| &c.surface)
    }

    pub fn event(&mut self, key: Key<'_>) -> Result<Outcome, SessionError> {
        if matches!(
            key,
            Key::PollAi
                | Key::ToggleAi
                | Key::ToggleTraining
                | Key::ClearLearning
                | Key::ClearContext
        ) {
            return Ok(Outcome::default());
        }
        if key == Key::ToggleMode {
            self.english = !self.english;
            self.refresh();
            return Ok(Outcome {
                handled: true,
                committed: None,
            });
        }
        if key == Key::Space && !self.raw.is_empty() {
            if self.english {
                let mut committed = if self.selecting {
                    self.display().to_owned()
                } else {
                    self.raw.clone()
                };
                committed.push(' ');
                self.clear();
                return Ok(Outcome {
                    handled: true,
                    committed: Some(committed),
                });
            }
            return self.event(Key::Convert);
        }
        if let Key::Text(text) = key {
            if text.is_empty() || text.chars().any(char::is_control) {
                return Err(SessionError::InvalidText);
            }
            if self.raw.len().saturating_add(text.len()) > MAX_INPUT_CHARS * 4 {
                return Err(ConvertError::InputTooLong.into());
            }

            let next = format!("{}{text}", self.raw);
            let preedit = if self.english || self.engine.is_english_word(&next) {
                next.clone()
            } else {
                self.engine.normalize_input(&next, true)
            };
            if preedit.chars().count() > MAX_INPUT_CHARS {
                return Err(ConvertError::InputTooLong.into());
            }
            self.raw = next;
            self.preedit = preedit;
            self.refresh();
            return Ok(Outcome {
                handled: true,
                committed: None,
            });
        }
        if self.raw.is_empty() {
            return Ok(Outcome::default());
        }
        match key {
            Key::Backspace => {
                // Undo one raw Unicode scalar/key, not one rendered kana.
                self.raw.pop();
                self.refresh();
            }
            Key::Convert if !self.selecting => {
                // The local helper can start or rotate its authentication token
                // after this input session was created. Refresh on conversion,
                // never on every typed character or timer tick.

                self.candidates = if self.english {
                    vec![Candidate {
                        surface: self.raw.clone(),
                        score: 0,
                        segments: 1,
                    }]
                } else {
                    self.engine
                        .convert_with_context(&self.raw, "", self.limit)?
                        .candidates
                };
                self.selected = 0;
                self.selecting = true;
            }
            Key::Next | Key::Previous if !self.selecting && !self.candidates.is_empty() => {
                self.selected = if key == Key::Previous {
                    self.candidates.len() - 1
                } else {
                    0
                };
                self.selecting = true;
            }
            Key::Convert | Key::Next if !self.candidates.is_empty() => {
                self.selected = (self.selected + 1) % self.candidates.len()
            }
            Key::Previous if !self.candidates.is_empty() => {
                self.selected = (self.selected + self.candidates.len() - 1) % self.candidates.len()
            }
            Key::Next | Key::Previous => return Ok(Outcome::default()),
            Key::Enter => {
                let committed = if self.selecting {
                    self.display().to_owned()
                } else if self.english || self.engine.is_english_word(&self.raw) {
                    self.raw.clone()
                } else {
                    self.engine.normalize_input(&self.raw, false)
                };

                self.clear();
                return Ok(Outcome {
                    handled: true,
                    committed: Some(committed),
                });
            }
            Key::Cancel => self.clear(),
            Key::Literal => {
                self.candidates = vec![Candidate {
                    surface: self.raw.clone(),
                    score: 0,
                    segments: 1,
                }];
                self.selected = 0;
                self.selecting = true;
            }
            Key::FullWidth | Key::HalfWidth => {
                let source = if self.selecting {
                    self.display()
                } else {
                    &self.raw
                };
                let surface = if key == Key::FullWidth {
                    crate::width::full_width(source)
                } else {
                    crate::width::half_width(source)
                };
                self.candidates = vec![Candidate {
                    surface,
                    score: 0,
                    segments: 1,
                }];
                self.selected = 0;
                self.selecting = true;
            }
            Key::Text(_) => unreachable!(),
            Key::Convert => unreachable!(),
            Key::ToggleMode
            | Key::ToggleAi
            | Key::ClearContext
            | Key::ClearLearning
            | Key::ToggleTraining
            | Key::Space => unreachable!(),
            Key::PollAi => unreachable!(),
        }
        Ok(Outcome {
            handled: true,
            committed: None,
        })
    }

    fn clear(&mut self) {
        self.raw.clear();
        self.preedit.clear();
        self.candidates.clear();
        self.selected = 0;
        self.selecting = false;
    }
    // Retained C ABI queries always report disabled; no AI or learning code remains.
    pub fn ai_enabled(&self) -> bool {
        false
    }
    pub fn local_ai_pending(&self) -> bool {
        false
    }
    pub fn local_ai_enabled(&self) -> bool {
        false
    }
    pub fn local_ai_status(&self) -> &'static str {
        "local:removed"
    }
    pub fn learned_entries(&self) -> usize {
        0
    }
    pub fn text_learning_enabled(&self) -> bool {
        false
    }
    pub fn learned_text_features(&self) -> usize {
        0
    }
    pub fn clear_context(&mut self) {}
    pub fn english_mode(&self) -> bool {
        self.english
    }
    fn refresh(&mut self) {
        self.preedit = if self.english || self.engine.is_english_word(&self.raw) {
            self.raw.clone()
        } else {
            self.engine.normalize_input(&self.raw, true)
        };
        self.candidates = self
            .engine
            .predict(&self.raw, self.limit, self.english)
            .unwrap_or_default();

        self.selected = 0;
        self.selecting = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> Session {
        Session::new(Engine::bundled(), 8).unwrap()
    }

    #[test]
    fn english_slang_literal_and_space() {
        let mut s = session();
        for text in ["omg", "OMG", "lol", "gg", "www"] {
            s.event(Key::Text(text)).unwrap();
            assert_eq!(s.display(), text);
            assert_eq!(
                s.event(Key::Enter).unwrap().committed.as_deref(),
                Some(text)
            );
        }
        s.event(Key::Text("unregisteredXYZ")).unwrap();
        s.event(Key::Literal).unwrap();
        assert_eq!(
            s.event(Key::Enter).unwrap().committed.as_deref(),
            Some("unregisteredXYZ")
        );
        s.event(Key::ToggleMode).unwrap();
        assert!(s.english_mode());
        s.event(Key::Text("Hello123!")).unwrap();
        assert_eq!(s.display(), "Hello123!");
        assert_eq!(
            s.event(Key::Space).unwrap().committed.as_deref(),
            Some("Hello123! ")
        );
        s.event(Key::Text("newSlang2026")).unwrap();
        assert_eq!(
            s.event(Key::Enter).unwrap().committed.as_deref(),
            Some("newSlang2026")
        );
        s.event(Key::ToggleMode).unwrap();
        s.event(Key::Text("no")).unwrap();
        assert_eq!(s.display(), "の");
    }

    #[test]
    fn mixed_english_japanese_survives_editing_and_commit() {
        for (raw, expected) in [
            ("AI", "AI"),
            ("XYZ", "XYZ"),
            ("XYZga", "XYZが"),
            ("XYZによる", "XYZによる"),
            ("A", "A"),
            ("AIによる", "AIによる"),
            ("AIniyoru", "AIによる"),
            ("AIga", "AIが"),
            ("ai", "あい"),
            ("sex", "sex"),
            ("SEX", "SEX"),
            ("sexga", "sexが"),
            ("sexが", "sexが"),
            ("sexyni", "sexyに"),
            ("rustde", "rustで"),
            ("omgが", "omgが"),
            ("n'na", "んな"),
            ("nあ", "んあ"),
        ] {
            let mut s = session();
            for ch in raw.chars() {
                s.event(Key::Text(&ch.to_string())).unwrap();
            }
            assert_eq!(s.display(), expected, "preedit {raw}");
            assert_eq!(
                s.event(Key::Enter).unwrap().committed.as_deref(),
                Some(expected),
                "commit {raw}"
            );
        }
        let mut s = session();
        s.event(Key::Text("sexgaseikide")).unwrap();
        assert_eq!(s.preedit(), "sexがせいきで");
        s.event(Key::Backspace).unwrap();
        assert!(s.preedit().starts_with("sexが"));
        // Ordinary Japanese remains romaji; no short-English prefix hijack.
        assert_eq!(
            s.engine.normalize_input("nihongonyuuryoku", false),
            "にほんごにゅうりょく"
        );
    }
    #[test]
    fn explicit_uppercase_is_preserved_with_ai_on_or_off() {
        for enabled in [true, false] {
            for text in ["A", "AI", "XYZ", "XYZによる", "XYZga"] {
                let mut s = session();
                if !enabled {
                    s.event(Key::ToggleAi).unwrap();
                }
                for ch in text.chars() {
                    s.event(Key::Text(&ch.to_string())).unwrap();
                }
                s.event(Key::Space).unwrap();
                let expected = if text == "XYZga" { "XYZが" } else { text };
                assert_eq!(s.display(), expected);
                assert_eq!(
                    s.event(Key::Enter).unwrap().committed.as_deref(),
                    Some(expected)
                );
            }
        }
        let mut s = session();
        s.event(Key::Text("XYZ")).unwrap();
        s.event(Key::Backspace).unwrap();
        assert_eq!(s.display(), "XY");
        s.event(Key::Cancel).unwrap();
        s.event(Key::Text("ai")).unwrap();
        assert_eq!(s.display(), "あい");
    }

    #[test]
    fn predictions_require_explicit_selection() {
        let mut s = session();
        s.event(Key::Text("nihon")).unwrap();
        assert_eq!(s.display(), "にほn");
        assert!(s.candidates().iter().any(|c| c.surface == "日本語"));
        assert_eq!(s.selected_index(), None);
        assert_eq!(
            s.event(Key::Enter).unwrap().committed.as_deref(),
            Some("にほん")
        );
        s.event(Key::Text("nihon")).unwrap();
        s.event(Key::Next).unwrap();
        assert!(s.selected_index().is_some());
        let chosen = s.display().to_owned();
        assert_eq!(s.event(Key::Enter).unwrap().committed, Some(chosen));
        s.event(Key::ToggleMode).unwrap();
        s.event(Key::Text("hel")).unwrap();
        assert!(s.candidates().iter().any(|c| c.surface == "hello"));
        assert_eq!(s.display(), "hel");
    }
    #[test]
    fn incremental_pending_and_undo() {
        let mut s = session();
        for (text, expected) in [
            ("n", "n"),
            ("i", "に"),
            ("h", "にh"),
            ("o", "にほ"),
            ("n", "にほn"),
            ("g", "にほんg"),
            ("o", "にほんご"),
        ] {
            assert_eq!(s.event(Key::Text(text)).unwrap().committed, None);
            assert_eq!(s.preedit(), expected);
        }
        s.event(Key::Backspace).unwrap();
        assert_eq!(s.preedit(), "にほんg");
        s.event(Key::Cancel).unwrap();
        for (text, expected) in [("k", "k"), ("y", "ky"), ("a", "きゃ")] {
            s.event(Key::Text(text)).unwrap();
            assert_eq!(s.preedit(), expected);
        }
        s.event(Key::Backspace).unwrap();
        assert_eq!(s.preedit(), "ky");
    }
    #[test]
    fn candidate_selection_requires_enter() {
        let mut s = session();
        s.event(Key::Text("hashi")).unwrap();
        assert_eq!(s.selected_index(), None);
        assert_eq!(s.event(Key::Convert).unwrap().committed, None);
        assert_eq!(s.display(), "橋");
        s.event(Key::Next).unwrap();
        assert_eq!(s.display(), "箸");
        s.event(Key::Previous).unwrap();
        assert_eq!(s.display(), "橋");
        s.event(Key::Previous).unwrap();
        assert_eq!(s.display(), "はし");
        s.event(Key::Convert).unwrap();
        assert_eq!(s.display(), "橋");
        s.event(Key::Next).unwrap();
        assert_eq!(
            s.event(Key::Enter).unwrap().committed.as_deref(),
            Some("箸")
        );
        assert!(s.raw().is_empty());
        assert_eq!(s.selected_index(), None);
        assert!(!s.event(Key::Enter).unwrap().handled);
    }
    #[test]
    fn limits_errors_and_cancel_preserve_contract() {
        let mut s = session();
        s.event(Key::Text("n")).unwrap();
        assert_eq!(
            s.event(Key::Enter).unwrap().committed.as_deref(),
            Some("ん")
        );
        s.event(Key::Text("🙂")).unwrap();
        s.event(Key::Backspace).unwrap();
        assert!(s.preedit().is_empty());
        s.event(Key::Text("は".repeat(128).as_str())).unwrap();
        assert!(s.event(Key::Text("あ")).is_err());
        assert_eq!(s.raw(), "は".repeat(128));
        assert!(s.event(Key::Text("\n")).is_err());
        assert!(s.event(Key::Text("")).is_err());
        s.event(Key::Cancel).unwrap();
        assert!(s.raw().is_empty());
        assert!(!s.event(Key::Backspace).unwrap().handled);
        assert!(Session::new(Engine::bundled(), 0).is_err());
    }
    #[test]
    fn editing_invalidates_candidates() {
        let mut s = session();
        s.event(Key::Text("hashi")).unwrap();
        s.event(Key::Convert).unwrap();
        s.event(Key::Text("🙂")).unwrap();
        assert!(s.candidates().is_empty());
        assert_eq!(s.display(), "はし🙂");
        assert_eq!(
            s.event(Key::Enter).unwrap().committed.as_deref(),
            Some("はし🙂")
        );
    }

    #[test]
    fn dictionary_failure_preserves_composition() {
        #[derive(Debug)]
        struct Broken;
        impl crate::DictionarySource for Broken {
            fn lookup(
                &self,
                _: &str,
                _: usize,
            ) -> Result<std::borrow::Cow<'_, [crate::Entry]>, crate::LookupError> {
                Err(crate::LookupError("broken block".into()))
            }
            fn len(&self) -> usize {
                1
            }
            fn max_reading_chars(&self) -> usize {
                4
            }
        }
        let mut s = Session::new(Engine::new(Broken), 8).unwrap();
        s.event(Key::Text("hashi")).unwrap();
        assert!(s.event(Key::Convert).is_err());
        assert_eq!(s.raw(), "hashi");
        assert_eq!(s.display(), "はし");
        assert!(s.candidates().is_empty());
        assert_eq!(
            s.event(Key::Enter).unwrap().committed.as_deref(),
            Some("はし")
        );
    }
}
