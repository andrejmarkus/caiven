use thiserror::Error;

#[derive(Debug, Error)]
pub enum LangError {
    #[error("line {line}: unexpected character '{ch}'")]
    UnexpectedChar { line: usize, col: usize, ch: char },

    #[error("line {line}: unexpected token '{got}', expected {expected}")]
    UnexpectedToken {
        line: usize,
        col: usize,
        got: String,
        expected: String,
    },

    #[error("line {line}: unterminated string")]
    UnterminatedString { line: usize, col: usize },

    #[error("line {line}: undefined variable '{name}'")]
    UndefinedVariable { line: usize, name: String },

    #[error("line {line}: undefined function '{name}'")]
    UndefinedFunction { line: usize, name: String },

    #[error("line {line}: wrong arg count for '{name}': expected {expected}, got {got}")]
    ArgCount {
        line: usize,
        name: String,
        expected: usize,
        got: usize,
    },

    #[error("line {line}: '{name}' requires a literal argument")]
    RequiresLiteral { line: usize, name: String },

    #[error("line {line}: return outside function")]
    ReturnOutsideFunction { line: usize },

    #[error("line {line}: too many local variables (max 2044)")]
    TooManyLocals { line: usize },

    #[error("line {line}: break outside loop")]
    BreakOutsideLoop { line: usize },

    #[error("line {line}: '{feature}' not yet implemented")]
    NotImplemented { line: usize, feature: String },

    #[error("unresolved label '{label}'")]
    UnresolvedLabel { label: String },
}

impl LangError {
    /// 1-based source line the error points at, if known.
    pub fn line(&self) -> Option<usize> {
        match self {
            LangError::UnexpectedChar { line, .. }
            | LangError::UnexpectedToken { line, .. }
            | LangError::UnterminatedString { line, .. }
            | LangError::UndefinedVariable { line, .. }
            | LangError::UndefinedFunction { line, .. }
            | LangError::ArgCount { line, .. }
            | LangError::RequiresLiteral { line, .. }
            | LangError::ReturnOutsideFunction { line }
            | LangError::TooManyLocals { line }
            | LangError::BreakOutsideLoop { line }
            | LangError::NotImplemented { line, .. } => Some(*line),
            LangError::UnresolvedLabel { .. } => None,
        }
    }

    /// 1-based column plus caret width, when the error carries an exact position.
    fn caret(&self) -> Option<(usize, usize)> {
        match self {
            LangError::UnexpectedChar { col, .. } => Some((*col, 1)),
            LangError::UnexpectedToken { col, got, .. } => {
                Some((*col, got.chars().count().max(1)))
            }
            LangError::UnterminatedString { col, .. } => Some((*col, 1)),
            _ => None,
        }
    }

    /// Identifier to search for in the source line when there is no exact column.
    fn name_hint(&self) -> Option<&str> {
        match self {
            LangError::UndefinedVariable { name, .. }
            | LangError::UndefinedFunction { name, .. }
            | LangError::ArgCount { name, .. }
            | LangError::RequiresLiteral { name, .. } => Some(name),
            _ => None,
        }
    }

    /// Render the error with the offending source line and a caret marker:
    ///
    /// ```text
    /// line 3: undefined variable 'foo'
    ///    3 | x = foo + 1
    ///      |     ^^^
    /// ```
    pub fn render(&self, src: &str) -> String {
        let mut out = self.to_string();
        let Some(line_no) = self.line() else {
            return out;
        };
        let Some(text) = src.lines().nth(line_no.saturating_sub(1)) else {
            return out;
        };
        out.push_str(&format!("\n{:>4} | {}", line_no, text));
        let span = self.caret().or_else(|| {
            self.name_hint()
                .and_then(|n| text.find(n).map(|i| (i + 1, n.chars().count())))
        });
        if let Some((col, len)) = span {
            let pad = col.saturating_sub(1);
            out.push_str(&format!(
                "\n{:>4} | {}{}",
                "",
                " ".repeat(pad),
                "^".repeat(len.max(1))
            ));
        }
        out
    }
}

pub type Result<T> = std::result::Result<T, LangError>;

#[cfg(test)]
mod tests {
    #[test]
    fn caret_on_unexpected_token() {
        let src = "let x = 0\nloop:\n  if x then then end\nend";
        let err = crate::compile(src).unwrap_err();
        let r = err.render(src);
        let lines: Vec<&str> = r.lines().collect();
        assert!(lines[0].starts_with("line 3:"), "got: {r}");
        assert_eq!(lines[1], "   3 |   if x then then end");
        let caret_at = lines[2].find('^').unwrap();
        let then2_at = lines[1].rfind("then").unwrap();
        assert_eq!(caret_at, then2_at, "caret under 2nd 'then': {r}");
        assert!(lines[2].contains("^^^^"), "4-wide caret: {r}");
    }

    #[test]
    fn caret_on_undefined_variable() {
        let src = "let x = 0\nloop:\n  x = foo + 1";
        let err = crate::compile(src).unwrap_err();
        let r = err.render(src);
        assert!(r.contains("undefined variable 'foo'"), "got: {r}");
        assert!(r.contains("   3 |   x = foo + 1"), "got: {r}");
        let lines: Vec<&str> = r.lines().collect();
        let caret_at = lines[2].find('^').unwrap();
        let foo_at = lines[1].find("foo").unwrap();
        assert_eq!(caret_at, foo_at, "caret must sit under 'foo': {r}");
        assert!(lines[2].contains("^^^"), "3-wide caret: {r}");
    }

    #[test]
    fn caret_on_unexpected_char() {
        let src = "loop:\n  x = 1 ` 2\nend";
        let err = crate::compile(src).unwrap_err();
        let r = err.render(src);
        let lines: Vec<&str> = r.lines().collect();
        let caret_at = lines[2].find('^').unwrap();
        let tick_at = lines[1].find('`').unwrap();
        assert_eq!(caret_at, tick_at, "caret must sit under backtick: {r}");
    }

    #[test]
    fn render_without_position_is_plain() {
        let err = crate::error::LangError::UnresolvedLabel {
            label: "x".into(),
        };
        assert_eq!(err.render("src"), "unresolved label 'x'");
    }
}
