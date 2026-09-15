use mahoraga::{Env, Value};
use ratatui::style::Style;
use ratatui_textarea::TextArea;

pub struct Entry {
    pub input: String,
    pub output: String,
}

pub struct App {
    pub textarea: TextArea<'static>,
    pub history: Vec<Entry>,
    /// Result of evaluating the current input, `None` when the input is empty.
    pub preview: Option<mahoraga::Result<Value>>,
    /// Index of the history entry shown in the input, `None` when editing a fresh line.
    history_idx: Option<usize>,
    /// Unsubmitted input saved while browsing history.
    draft: String,
    env: Env,
}

impl App {
    pub fn new() -> App {
        App {
            textarea: Self::new_textarea(),
            history: Vec::new(),
            preview: None,
            history_idx: None,
            draft: String::new(),
            env: Env::std(),
        }
    }

    fn new_textarea() -> TextArea<'static> {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        textarea.set_placeholder_text("Type an expression, e.g. 2 + 2");
        textarea
    }

    pub fn input(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Call after every edit of the input.
    pub fn on_edit(&mut self) {
        self.history_idx = None;
        self.update_preview();
    }

    pub fn history_prev(&mut self) {
        let idx = match self.history_idx {
            _ if self.history.is_empty() => return,
            None => {
                self.draft = self.input();
                self.history.len() - 1
            }
            Some(idx) => idx.saturating_sub(1),
        };
        self.history_idx = Some(idx);
        self.set_input(&self.history[idx].input.clone());
    }

    pub fn history_next(&mut self) {
        let Some(idx) = self.history_idx else {
            return;
        };
        if idx + 1 < self.history.len() {
            self.history_idx = Some(idx + 1);
            self.set_input(&self.history[idx + 1].input.clone());
        } else {
            self.history_idx = None;
            let draft = std::mem::take(&mut self.draft);
            self.set_input(&draft);
        }
    }

    fn set_input(&mut self, text: &str) {
        self.textarea = Self::new_textarea();
        self.textarea.insert_str(text);
        self.update_preview();
    }

    fn update_preview(&mut self) {
        let input = self.input();
        self.preview = (!input.trim().is_empty()).then(|| self.eval(&input));
    }

    pub fn submit(&mut self) {
        let Some(Ok(value)) = &self.preview else {
            return;
        };
        let output = value.to_string();
        let input = self.input();
        self.history.push(Entry { input, output });
        self.textarea = Self::new_textarea();
        self.preview = None;
        self.history_idx = None;
    }

    fn eval(&self, input: &str) -> mahoraga::Result<Value> {
        mahoraga::eval(mahoraga::parse_expr(input)?, &self.env)
    }
}
