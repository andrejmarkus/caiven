use crate::error::{LangError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(u32),
    StringLit(String),
    Ident(String),

    // Keywords (old compat)
    Const,
    Let,
    If,
    Then,
    Else,
    End,
    Fn,
    Return,
    // New Lua keywords
    And,
    Break,
    Do,
    Elseif,
    False,
    For,
    Function,
    In,
    Local,
    Nil,
    Not,
    Or,
    Repeat,
    True,
    Until,
    While,

    // Operators (old)
    Eq,
    PlusEq,
    MinusEq,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    LParen,
    RParen,
    Comma,
    Colon,

    // New operators
    TildeEq,
    Hash,
    Caret,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Dot,
    DotDot,
    DotDotDot,

    Newline,
    Eof,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Number(n) => write!(f, "{}", n),
            TokenKind::StringLit(s) => write!(f, "\"{}\"", s),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::Const => write!(f, "const"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Then => write!(f, "then"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::End => write!(f, "end"),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Do => write!(f, "do"),
            TokenKind::Elseif => write!(f, "elseif"),
            TokenKind::False => write!(f, "false"),
            TokenKind::For => write!(f, "for"),
            TokenKind::Function => write!(f, "function"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Local => write!(f, "local"),
            TokenKind::Nil => write!(f, "nil"),
            TokenKind::Not => write!(f, "not"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Repeat => write!(f, "repeat"),
            TokenKind::True => write!(f, "true"),
            TokenKind::Until => write!(f, "until"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::PlusEq => write!(f, "+="),
            TokenKind::MinusEq => write!(f, "-="),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::NotEq => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::LtEq => write!(f, "<="),
            TokenKind::GtEq => write!(f, ">="),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::TildeEq => write!(f, "~="),
            TokenKind::Hash => write!(f, "#"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::DotDotDot => write!(f, "..."),
            TokenKind::Newline => write!(f, "newline"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
}

pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    line: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            line: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek2(&self) -> Option<char> {
        let mut chars = self.src[self.pos..].chars();
        chars.next();
        chars.next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn skip_whitespace_inline(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t') | Some('\r')) {
            self.advance();
        }
    }

    fn read_number(&mut self) -> u32 {
        if self.peek() == Some('0') && matches!(self.peek2(), Some('x') | Some('X')) {
            self.advance(); // '0'
            self.advance(); // 'x'
            let start = self.pos;
            while matches!(
                self.peek(),
                Some('0'..='9') | Some('a'..='f') | Some('A'..='F')
            ) {
                self.advance();
            }
            u32::from_str_radix(&self.src[start..self.pos], 16).unwrap_or(0)
        } else {
            let start = self.pos;
            while matches!(self.peek(), Some('0'..='9')) {
                self.advance();
            }
            self.src[start..self.pos].parse::<u32>().unwrap_or(0)
        }
    }

    fn read_string(&mut self, quote: char) -> Result<String> {
        let line = self.line;
        let mut s = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => return Err(LangError::UnterminatedString { line }),
                Some(ch) if ch == quote => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        Some('\'') => s.push('\''),
                        Some(c) => {
                            s.push('\\');
                            s.push(c);
                        }
                        None => return Err(LangError::UnterminatedString { line }),
                    }
                }
                Some(ch) => {
                    self.advance();
                    s.push(ch);
                }
            }
        }
        Ok(s)
    }

    fn skip_block_comment(&mut self) {
        // Already consumed '--[['
        loop {
            match self.peek() {
                None => break,
                Some('\n') => {
                    self.line += 1;
                    self.advance();
                }
                Some(']') => {
                    self.advance();
                    if self.peek() == Some(']') {
                        self.advance();
                        break;
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_inline();
            let line = self.line;
            match self.peek() {
                None => {
                    tokens.push(Token {
                        kind: TokenKind::Eof,
                        line,
                    });
                    break;
                }
                Some('\n') => {
                    self.advance();
                    self.line += 1;
                    tokens.push(Token {
                        kind: TokenKind::Newline,
                        line,
                    });
                }
                Some('-') => {
                    self.advance();
                    if self.peek() == Some('-') {
                        self.advance();
                        // Check for block comment --[[
                        if self.peek() == Some('[') {
                            self.advance();
                            if self.peek() == Some('[') {
                                self.advance();
                                self.skip_block_comment();
                                continue;
                            }
                            // just --[ .. fall through to line comment
                        }
                        // Line comment: skip to end of line
                        while !matches!(self.peek(), None | Some('\n')) {
                            self.advance();
                        }
                    } else if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::MinusEq,
                            line,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::Minus,
                            line,
                        });
                    }
                }
                Some('+') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::PlusEq,
                            line,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::Plus,
                            line,
                        });
                    }
                }
                Some('*') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::Star,
                        line,
                    });
                }
                Some('/') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::Slash,
                        line,
                    });
                }
                Some('%') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::Percent,
                        line,
                    });
                }
                Some('^') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::Caret,
                        line,
                    });
                }
                Some('#') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::Hash,
                        line,
                    });
                }
                Some('(') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::LParen,
                        line,
                    });
                }
                Some(')') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::RParen,
                        line,
                    });
                }
                Some('{') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::LBrace,
                        line,
                    });
                }
                Some('}') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::RBrace,
                        line,
                    });
                }
                Some('[') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::LBracket,
                        line,
                    });
                }
                Some(']') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::RBracket,
                        line,
                    });
                }
                Some(',') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::Comma,
                        line,
                    });
                }
                Some(';') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::Semicolon,
                        line,
                    });
                }
                Some(':') => {
                    self.advance();
                    tokens.push(Token {
                        kind: TokenKind::Colon,
                        line,
                    });
                }
                Some('=') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::EqEq,
                            line,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::Eq,
                            line,
                        });
                    }
                }
                Some('!') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::NotEq,
                            line,
                        });
                    } else {
                        return Err(LangError::UnexpectedChar { line, ch: '!' });
                    }
                }
                Some('~') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::TildeEq,
                            line,
                        });
                    } else {
                        return Err(LangError::UnexpectedChar { line, ch: '~' });
                    }
                }
                Some('<') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::LtEq,
                            line,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::Lt,
                            line,
                        });
                    }
                }
                Some('>') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::GtEq,
                            line,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::Gt,
                            line,
                        });
                    }
                }
                Some('.') => {
                    self.advance();
                    if self.peek() == Some('.') {
                        self.advance();
                        if self.peek() == Some('.') {
                            self.advance();
                            tokens.push(Token {
                                kind: TokenKind::DotDotDot,
                                line,
                            });
                        } else {
                            tokens.push(Token {
                                kind: TokenKind::DotDot,
                                line,
                            });
                        }
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::Dot,
                            line,
                        });
                    }
                }
                Some('"') => {
                    self.advance();
                    let s = self.read_string('"')?;
                    tokens.push(Token {
                        kind: TokenKind::StringLit(s),
                        line,
                    });
                }
                Some('\'') => {
                    self.advance();
                    let s = self.read_string('\'')?;
                    tokens.push(Token {
                        kind: TokenKind::StringLit(s),
                        line,
                    });
                }
                Some(ch) if ch.is_ascii_digit() => {
                    let n = self.read_number();
                    tokens.push(Token {
                        kind: TokenKind::Number(n),
                        line,
                    });
                }
                Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {
                    let start = self.pos;
                    while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_') {
                        self.advance();
                    }
                    let word = &self.src[start..self.pos];
                    let kind = keyword(word);
                    tokens.push(Token { kind, line });
                }
                Some(ch) => {
                    return Err(LangError::UnexpectedChar { line, ch });
                }
            }
        }
        Ok(tokens)
    }
}

fn keyword(word: &str) -> TokenKind {
    match word {
        "const" => TokenKind::Const,
        "let" => TokenKind::Let,
        "if" => TokenKind::If,
        "then" => TokenKind::Then,
        "else" => TokenKind::Else,
        "end" => TokenKind::End,
        "fn" => TokenKind::Fn,
        "return" => TokenKind::Return,
        "and" => TokenKind::And,
        "break" => TokenKind::Break,
        "do" => TokenKind::Do,
        "elseif" => TokenKind::Elseif,
        "false" => TokenKind::False,
        "for" => TokenKind::For,
        "function" => TokenKind::Function,
        "in" => TokenKind::In,
        "local" => TokenKind::Local,
        "nil" => TokenKind::Nil,
        "not" => TokenKind::Not,
        "or" => TokenKind::Or,
        "repeat" => TokenKind::Repeat,
        "true" => TokenKind::True,
        "until" => TokenKind::Until,
        "while" => TokenKind::While,
        other => TokenKind::Ident(other.to_string()),
    }
}
