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
    ai: bool,
    context: String,
    personal: nextime_ai::PersonalModel,
    learn_selection: bool,
    text_learning: bool,
    text_model: nextime_ai::Model,
    local_ai: Option<nextime_ai::LocalLlm>,
    local_ai_error: Option<nextime_ai::LocalConfigError>,
}

impl Session {
    pub fn new(engine: Engine, limit: usize) -> Result<Self, SessionError> {
        if !(1..=MAX_CANDIDATES).contains(&limit) {
            return Err(ConvertError::InvalidLimit.into());
        }
        let local_ai = Self::optional_llm();
        let local_ai_error = local_ai.as_ref().err().copied();
        Ok(Self {
            engine,
            limit,
            raw: String::new(),
            preedit: String::new(),
            candidates: Vec::new(),
            selected: 0,
            selecting: false,
            english: false,
            ai: true,
            context: String::new(),
            personal: nextime_ai::PersonalModel::default(),
            learn_selection: false,
            text_learning: true,
            text_model: nextime_ai::Model::default(),
            local_ai: local_ai.ok(),
            local_ai_error,
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
        self.candidates.get(index).map(|candidate| {
            if self.ai && self.selecting && !self.english {
                nextime_ai::Model::bundled().native_bonus(&self.context, &candidate.surface)
            } else {
                0
            }
        })
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
        if key == Key::PollAi {
            let pending = self.local_ai_pending();
            let changed = self.ai
                && self.selecting
                && self.selected == 0
                && self
                    .local_ai
                    .as_mut()
                    .is_some_and(|llm| llm.poll(&mut self.candidates));
            return Ok(Outcome {
                handled: changed || (pending && !self.local_ai_pending()),
                committed: None,
            });
        }
        if let Some(llm) = self.local_ai.as_mut() {
            llm.cancel();
        }
        if key == Key::ToggleTraining {
            self.text_learning = !self.text_learning;
            self.clear_context();
            self.refresh();
            return Ok(Outcome {
                handled: true,
                committed: None,
            });
        }
        if key == Key::ClearLearning {
            self.personal.clear();
            self.text_model = nextime_ai::Model::default();
            self.clear_context();
            self.refresh();
            return Ok(Outcome {
                handled: true,
                committed: None,
            });
        }
        if key == Key::ClearContext {
            self.clear_context();
            return Ok(Outcome {
                handled: true,
                committed: None,
            });
        }
        if key == Key::ToggleAi {
            self.ai = !self.ai;
            if self.ai {
                self.refresh_local_ai();
            }
            self.context.clear();
            self.refresh();
            return Ok(Outcome {
                handled: true,
                committed: None,
            });
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
                self.remember(&committed);
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
            if self.ai && self.raw.is_empty() {
                self.refresh_local_ai();
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
                if self.ai {
                    self.refresh_local_ai();
                }
                self.candidates = if self.english {
                    vec![Candidate {
                        surface: self.raw.clone(),
                        score: 0,
                        segments: 1,
                    }]
                } else {
                    if self.ai {
                        self.engine
                            .convert_with_context(&self.raw, &self.context, self.limit)?
                            .candidates
                    } else {
                        self.engine.convert(&self.raw, self.limit)?.candidates
                    }
                };
                self.rerank_personal();
                self.selected = 0;
                self.selecting = true;
                self.learn_selection = !self.english;
                if self.ai {
                    if let Some(llm) = self.local_ai.as_mut() {
                        nextime_ai::case_candidates(&mut self.candidates, self.limit);
                        let reading = if self.english {
                            self.raw.clone()
                        } else {
                            self.engine.normalize_input(&self.raw, false)
                        };
                        llm.begin(&self.context, &reading, &self.candidates);
                    }
                }
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
                if self.ai
                    && self.selecting
                    && self.learn_selection
                    && self
                        .candidates
                        .get(self.selected)
                        .is_some_and(|c| c.segments == 1)
                {
                    let reading = self.engine.normalize_input(&self.raw, false);
                    self.personal
                        .learn_with_context(&reading, &committed, &self.context);
                }
                self.remember(&committed);
                self.clear();
                return Ok(Outcome {
                    handled: true,
                    committed: Some(committed),
                });
            }
            Key::Cancel => self.clear(),
            Key::Literal => {
                self.learn_selection = false;
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
                self.learn_selection = false;
                if let Some(llm) = self.local_ai.as_mut() {
                    llm.cancel();
                }
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
        self.learn_selection = false;
    }
    pub fn ai_enabled(&self) -> bool {
        self.ai
    }
    pub fn local_ai_pending(&self) -> bool {
        self.ai
            && self.selecting
            && self.selected == 0
            && self
                .local_ai
                .as_ref()
                .is_some_and(nextime_ai::LocalLlm::pending)
    }
    pub fn local_ai_enabled(&self) -> bool {
        self.ai && self.local_ai.is_some()
    }
    pub fn local_ai_status(&self) -> &'static str {
        if !self.ai {
            "local:off"
        } else if std::env::var("LUQNEXA_AI_BACKEND").as_deref() != Ok("qwen") {
            "local:original"
        } else if self.local_ai_pending() {
            "local:pending"
        } else if self.local_ai.is_some() {
            "local:on"
        } else {
            self.local_ai_error
                .map_or("local:off", nextime_ai::LocalConfigError::status)
        }
    }
    fn refresh_local_ai(&mut self) {
        let configured = Self::optional_llm();
        self.local_ai_error = configured.as_ref().err().copied();
        self.local_ai = configured.ok();
    }
    fn optional_llm() -> Result<nextime_ai::LocalLlm, nextime_ai::LocalConfigError> {
        // Native inference has no settings, helper, sockets or credentials.
        if std::env::var("LUQNEXA_AI_BACKEND").as_deref() == Ok("qwen") {
            nextime_ai::LocalLlm::try_configured()
        } else {
            Err(nextime_ai::LocalConfigError::Disabled)
        }
    }
    pub fn learned_entries(&self) -> usize {
        self.personal.entry_count()
    }
    pub fn text_learning_enabled(&self) -> bool {
        self.text_learning
    }
    pub fn learned_text_features(&self) -> usize {
        self.text_model.feature_count()
    }
    fn rerank_personal(&mut self) {
        if self.ai {
            if !self.english {
                let reading = self.engine.normalize_input(&self.raw, false);
                self.personal.rerank_with_context(
                    &reading,
                    &self.context,
                    &mut self.candidates,
                    self.limit,
                );
            }
            if self.text_learning {
                self.text_model
                    .rerank_online(&self.context, &mut self.candidates, self.limit);
            }
        }
    }
    pub fn clear_context(&mut self) {
        self.context.clear();
        if let Some(llm) = self.local_ai.as_mut() {
            llm.cancel();
        }
    }
    fn remember(&mut self, text: &str) {
        if !self.ai {
            return;
        }
        if self.text_learning {
            self.text_model.learn_confirmed(&self.context, text);
        }
        self.context.push_str(text);
        let chars = self.context.chars().count();
        if chars > 96 {
            let start = self
                .context
                .char_indices()
                .nth(chars - 96)
                .map_or(0, |(i, _)| i);
            self.context.drain(..start);
        }
    }
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
            .predict(
                &self.raw,
                if self.ai { 32 } else { self.limit },
                self.english,
            )
            .unwrap_or_default();
        if self.ai {
            nextime_ai::Model::bundled().rerank(&self.context, &mut self.candidates, self.limit);
        }
        self.rerank_personal();
        self.selected = 0;
        self.selecting = false;
        self.learn_selection = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> Session {
        Session::new(Engine::bundled(), 8).unwrap()
    }
    #[test]
    fn human_character_input_learns_final_text_only() {
        let mut s = session();
        assert!(s.text_learning_enabled());
        s.event(Key::Text("打ち間違い")).unwrap();
        for _ in 0..5 {
            s.event(Key::Backspace).unwrap();
        }
        for c in "食事に箸を使います。".chars() {
            s.event(Key::Text(&c.to_string())).unwrap();
            assert_eq!(s.learned_text_features(), 0);
        }
        let committed = s.event(Key::Enter).unwrap().committed.unwrap();
        assert_eq!(committed, "食事に箸を使います。");
        assert!(s.learned_text_features() > 0);
        assert!(s.text_model.score("食事に", "箸") > s.text_model.score("食事に", "橋"));
        assert_eq!(s.text_model.score("", "打ち間違い"), 0);
        let before = s.learned_text_features();
        s.event(Key::Text("取り消した文章")).unwrap();
        s.event(Key::Cancel).unwrap();
        assert_eq!(s.learned_text_features(), before);
        s.event(Key::ToggleTraining).unwrap();
        s.event(Key::Text("新しい文章")).unwrap();
        s.event(Key::Enter).unwrap();
        assert_eq!(s.learned_text_features(), before);
        s.event(Key::ClearLearning).unwrap();
        assert_eq!(s.learned_text_features(), 0);
        assert!(!s.text_learning_enabled());
        s.event(Key::ToggleTraining).unwrap();
        s.event(Key::ToggleMode).unwrap();
        for c in "hello".chars() {
            s.event(Key::Text(&c.to_string())).unwrap();
        }
        s.event(Key::Space).unwrap();
        assert!(s.learned_text_features() > 0);
        s.event(Key::ClearLearning).unwrap();
        s.event(Key::ToggleAi).unwrap();
        s.event(Key::Text("doNotLearn")).unwrap();
        s.event(Key::Enter).unwrap();
        assert_eq!(s.learned_text_features(), 0);
    }
    #[test]
    fn learns_explicit_conversion_only_and_can_forget() {
        let mut s = session();
        s.event(Key::Text("hashi")).unwrap();
        s.event(Key::Space).unwrap();
        for _ in 0..s.candidates.len() {
            if s.display() == "箸" {
                break;
            }
            s.event(Key::Next).unwrap();
        }
        assert_eq!(s.display(), "箸");
        assert_eq!(s.learned_entries(), 0);
        s.event(Key::Enter).unwrap();
        assert_eq!(s.learned_entries(), 1);
        s.clear_context();
        s.event(Key::Text("はし")).unwrap();
        s.event(Key::Space).unwrap();
        assert_eq!(s.display(), "箸");
        s.event(Key::Cancel).unwrap();
        s.event(Key::ClearLearning).unwrap();
        assert_eq!(s.learned_entries(), 0);
        s.event(Key::Text("hashi")).unwrap();
        s.event(Key::Space).unwrap();
        assert_eq!(s.display(), "橋");
        s.event(Key::ToggleAi).unwrap();
        s.event(Key::Space).unwrap();
        s.event(Key::Enter).unwrap();
        assert_eq!(s.learned_entries(), 0);
        s.event(Key::ToggleAi).unwrap();
        s.event(Key::Text("hashi")).unwrap();
        s.event(Key::Literal).unwrap();
        s.event(Key::Enter).unwrap();
        assert_eq!(s.learned_entries(), 0);
        s.event(Key::Text("yoroshi")).unwrap();
        s.event(Key::Next).unwrap();
        s.event(Key::Enter).unwrap();
        assert_eq!(s.learned_entries(), 0);
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
    fn ai_context_is_transient_bounded_and_can_be_disabled() {
        let mut s = session();
        s.event(Key::Text("食事に")).unwrap();
        s.event(Key::Enter).unwrap();
        s.event(Key::Text("hashi")).unwrap();
        assert!(s.event(Key::Space).unwrap().committed.is_none());
        assert_eq!(s.display(), "箸");
        s.event(Key::ToggleAi).unwrap();
        assert!(!s.ai_enabled());
        assert!(s.context.is_empty());
        s.event(Key::Space).unwrap();
        assert_eq!(s.display(), "橋");
        s.event(Key::Cancel).unwrap();
        s.event(Key::ToggleAi).unwrap();
        s.remember(&"🙂".repeat(200));
        assert_eq!(s.context.chars().count(), 96);
        s.event(Key::ClearContext).unwrap();
        assert!(s.context.is_empty());
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
