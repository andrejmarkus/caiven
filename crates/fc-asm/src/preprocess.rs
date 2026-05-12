use crate::error::AsmError;
use crate::expr::eval_expr;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineSection {
    Program,
    SpriteSheet,
}

#[derive(Debug, Clone)]
pub struct SourceLine {
    pub text: String,
    pub file: String,
    pub line_number: usize,
    pub section: LineSection,
}

#[derive(Debug, Clone)]
pub struct MacroDef {
    pub params: Vec<String>,
    pub body: Vec<String>,
}

pub struct Preprocessor {
    pub constants: HashMap<String, u16>,
    pub macros: HashMap<String, MacroDef>,
    include_stack: Vec<PathBuf>,
    current_section: LineSection,
}

impl Default for Preprocessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Preprocessor {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
            macros: HashMap::new(),
            include_stack: Vec::new(),
            current_section: LineSection::Program,
        }
    }

    pub fn process_str(&mut self, source: &str) -> Result<Vec<SourceLine>, AsmError> {
        let lines = source
            .lines()
            .enumerate()
            .map(|(i, l)| (i + 1, l.to_string()))
            .collect();
        self.process_lines(lines, "<input>")
    }

    pub fn process_file(&mut self, path: &Path) -> Result<Vec<SourceLine>, AsmError> {
        let canonical = path.canonicalize().map_err(|e| {
            AsmError::syntax(0, "", format!("cannot open '{}': {}", path.display(), e))
        })?;
        if self.include_stack.contains(&canonical) {
            return Err(AsmError::syntax(
                0,
                "",
                format!("include cycle: {}", path.display()),
            ));
        }
        let source = std::fs::read_to_string(path).map_err(|e| {
            AsmError::syntax(0, "", format!("cannot read '{}': {}", path.display(), e))
        })?;
        self.include_stack.push(canonical);
        let lines = source
            .lines()
            .enumerate()
            .map(|(i, l)| (i + 1, l.to_string()))
            .collect();
        let result = self.process_lines(lines, &path.display().to_string());
        self.include_stack.pop();
        result
    }

    fn process_lines(
        &mut self,
        lines: Vec<(usize, String)>,
        file: &str,
    ) -> Result<Vec<SourceLine>, AsmError> {
        let mut output = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            let (line_number, raw) = &lines[i];
            let line_number = *line_number;
            let stripped = strip_comment(raw);
            let trimmed = stripped.trim();

            if trimmed.is_empty() {
                i += 1;
                continue;
            }

            let tokens = tokenize(trimmed);
            if tokens.is_empty() {
                i += 1;
                continue;
            }

            let first = tokens[0].to_uppercase();

            // Section markers
            if first == ".BEGIN_SPRITE_SHEET" {
                self.current_section = LineSection::SpriteSheet;
                i += 1;
                continue;
            }
            if first == ".END_SPRITE_SHEET" {
                self.current_section = LineSection::Program;
                i += 1;
                continue;
            }

            if first == ".CONST" || first == "CONST" {
                if tokens.len() < 4 || tokens[2] != "=" {
                    return Err(AsmError::syntax(
                        line_number,
                        trimmed,
                        ".CONST requires: .CONST NAME = EXPR",
                    ));
                }
                let name = tokens[1].to_string();
                let expr_str = tokens[3..].join(" ");
                let val = eval_expr(&expr_str, &self.constants).map_err(|e| {
                    AsmError::syntax(line_number, trimmed, format!(".CONST eval error: {}", e))
                })?;
                self.constants.insert(name, val);
                i += 1;
                continue;
            }

            if first == ".INCLUDE" || first == "INCLUDE" {
                if tokens.len() < 2 {
                    return Err(AsmError::syntax(
                        line_number,
                        trimmed,
                        ".INCLUDE requires a filename",
                    ));
                }
                let filename = unquote(&tokens[1]).ok_or_else(|| {
                    AsmError::syntax(line_number, trimmed, ".INCLUDE filename must be quoted")
                })?;
                let include_path = if let Some(parent) = self.include_stack.last() {
                    parent.parent().unwrap_or(Path::new(".")).join(&filename)
                } else {
                    PathBuf::from(&filename)
                };
                let included = self.process_file(&include_path)?;
                output.extend(included);
                i += 1;
                continue;
            }

            if first == ".MACRO" || first == "MACRO" {
                if tokens.len() < 2 {
                    return Err(AsmError::syntax(
                        line_number,
                        trimmed,
                        ".MACRO requires a name",
                    ));
                }
                let macro_name = tokens[1].to_uppercase();
                let params: Vec<String> = tokens[2..].iter().map(|s| s.to_string()).collect();
                let mut body = Vec::new();
                i += 1;
                loop {
                    if i >= lines.len() {
                        return Err(AsmError::syntax(
                            line_number,
                            trimmed,
                            "unterminated .MACRO (missing .ENDM)",
                        ));
                    }
                    let (_, body_raw) = &lines[i];
                    let body_stripped = strip_comment(body_raw);
                    let body_trimmed = body_stripped.trim();
                    let body_tokens = tokenize(body_trimmed);
                    if !body_tokens.is_empty() && body_tokens[0].to_uppercase() == ".ENDM" {
                        i += 1;
                        break;
                    }
                    body.push(body_trimmed.to_string());
                    i += 1;
                }
                self.macros.insert(macro_name, MacroDef { params, body });
                continue;
            }

            // Macro invocation
            if let Some(mac) = self.macros.get(&first).cloned() {
                let args: Vec<String> = tokens[1..].iter().map(|s| s.to_string()).collect();
                if args.len() != mac.params.len() {
                    return Err(AsmError::syntax(
                        line_number,
                        trimmed,
                        format!(
                            "macro '{}' expects {} args, got {}",
                            first,
                            mac.params.len(),
                            args.len()
                        ),
                    ));
                }
                let section = self.current_section;
                for body_line in &mac.body {
                    let mut expanded = body_line.clone();
                    for (param, arg) in mac.params.iter().zip(args.iter()) {
                        expanded = expanded.replace(param.as_str(), arg.as_str());
                    }
                    if !expanded.trim().is_empty() {
                        output.push(SourceLine {
                            text: expanded,
                            file: file.to_string(),
                            line_number,
                            section,
                        });
                    }
                }
                i += 1;
                continue;
            }

            // Pass through
            output.push(SourceLine {
                text: trimmed.to_string(),
                file: file.to_string(),
                line_number,
                section: self.current_section,
            });
            i += 1;
        }
        Ok(output)
    }
}

pub fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_quote = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => in_quote = !in_quote,
            b';' if !in_quote => return &line[..i],
            _ => {}
        }
        i += 1;
    }
    line
}

pub fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();
    loop {
        while chars
            .peek()
            .map(|c| c.is_whitespace() || *c == ',')
            .unwrap_or(false)
        {
            chars.next();
        }
        let Some(&c) = chars.peek() else { break };
        if c == '"' {
            let mut s = String::from('"');
            chars.next();
            for ch in chars.by_ref() {
                s.push(ch);
                if ch == '"' {
                    break;
                }
            }
            tokens.push(s);
        } else {
            let mut s = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || c == ',' {
                    break;
                }
                s.push(c);
                chars.next();
            }
            if !s.is_empty() {
                tokens.push(s);
            }
        }
    }
    tokens
}

pub fn resolve_local_refs(expr: &str, scope: &str) -> String {
    if scope.is_empty() || !expr.contains('@') {
        return expr.to_string();
    }
    let mut result = String::with_capacity(expr.len() + 16);
    let bytes = expr.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'@' {
                result.push('@');
                result.push('@');
                i += 2;
            } else {
                result.push_str(scope);
                result.push_str("@@");
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

fn unquote(s: &str) -> Option<String> {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        Some(s[1..s.len() - 1].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_define_and_use() {
        let mut pp = Preprocessor::new();
        let lines = pp.process_str(".CONST W = 128\nMOV R0 W").unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "MOV R0 W");
        assert_eq!(pp.constants["W"], 128);
    }

    #[test]
    fn macro_expand() {
        let src = ".MACRO ZERO reg\nMOV reg 0\n.ENDM\nZERO R0";
        let mut pp = Preprocessor::new();
        let lines = pp.process_str(src).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "MOV R0 0");
    }

    #[test]
    fn strip_comment_quoted() {
        assert_eq!(strip_comment(r#"DB "he;y" ; comment"#), r#"DB "he;y" "#);
        assert_eq!(strip_comment("MOV R0 5 ; move"), "MOV R0 5 ");
    }

    #[test]
    fn tokenize_quoted() {
        let tokens = tokenize(r#".DB "hello""#);
        assert_eq!(tokens, vec![".DB", "\"hello\""]);
    }

    #[test]
    fn local_ref_resolution() {
        assert_eq!(resolve_local_refs("@loop", "main"), "main@@loop");
        assert_eq!(resolve_local_refs("main@@loop", "main"), "main@@loop");
        assert_eq!(resolve_local_refs("no_at", "main"), "no_at");
    }

    #[test]
    fn section_tagging() {
        let src = "MOV R0 0\n.BEGIN_SPRITE_SHEET\n.DB 1 2\n.END_SPRITE_SHEET\nMOV R1 0";
        let mut pp = Preprocessor::new();
        let lines = pp.process_str(src).unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].section, LineSection::Program);
        assert_eq!(lines[1].section, LineSection::SpriteSheet);
        assert_eq!(lines[2].section, LineSection::Program);
    }
}
