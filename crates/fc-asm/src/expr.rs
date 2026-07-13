use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EvalError {
    #[error("empty expression")]
    EmptyExpression,

    #[error("unexpected '{0}' in expression")]
    UnexpectedChar(char),

    #[error("unexpected end of expression")]
    UnexpectedEnd,

    #[error("expected ')'")]
    ExpectedCloseParen,

    #[error("unterminated char literal")]
    UnterminatedCharLiteral,

    #[error("division by zero")]
    DivisionByZero,

    #[error("empty numeric literal")]
    EmptyNumericLiteral,

    #[error("invalid number: {0}")]
    InvalidNumber(String),

    #[error("undefined symbol: {0}")]
    UndefinedSymbol(String),
}

pub fn eval_expr(s: &str, symbols: &HashMap<String, u16>) -> Result<u16, EvalError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(EvalError::EmptyExpression);
    }
    let bytes = s.as_bytes();
    let mut parser = Parser {
        src: bytes,
        pos: 0,
        symbols,
    };
    let val = parser.parse_expr()?;
    if parser.pos != bytes.len() {
        return Err(EvalError::UnexpectedChar(bytes[parser.pos] as char));
    }
    Ok(val)
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
    symbols: &'a HashMap<String, u16>,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn parse_expr(&mut self) -> Result<u16, EvalError> {
        self.parse_add()
    }

    fn parse_add(&mut self) -> Result<u16, EvalError> {
        let mut left = self.parse_shift()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'+') => {
                    self.pos += 1;
                    left = left.wrapping_add(self.parse_shift()?);
                }
                Some(b'-') => {
                    self.pos += 1;
                    left = left.wrapping_sub(self.parse_shift()?);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<u16, EvalError> {
        let mut left = self.parse_mul()?;
        loop {
            self.skip_ws();
            if self.pos + 1 < self.src.len()
                && self.src[self.pos] == b'<'
                && self.src[self.pos + 1] == b'<'
            {
                self.pos += 2;
                let r = self.parse_mul()?;
                left = left.wrapping_shl(r as u32);
            } else if self.pos + 1 < self.src.len()
                && self.src[self.pos] == b'>'
                && self.src[self.pos + 1] == b'>'
            {
                self.pos += 2;
                let r = self.parse_mul()?;
                left >>= r;
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<u16, EvalError> {
        let mut left = self.parse_unary()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'*') => {
                    self.pos += 1;
                    left = left.wrapping_mul(self.parse_unary()?);
                }
                Some(b'/') => {
                    self.pos += 1;
                    let r = self.parse_unary()?;
                    if r == 0 {
                        return Err(EvalError::DivisionByZero);
                    }
                    left /= r;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<u16, EvalError> {
        self.skip_ws();
        if self.peek() == Some(b'-') {
            self.pos += 1;
            let v = self.parse_atom()?;
            Ok(0u16.wrapping_sub(v))
        } else if self.peek() == Some(b'+') {
            self.pos += 1;
            self.parse_atom()
        } else {
            self.parse_atom()
        }
    }

    fn parse_atom(&mut self) -> Result<u16, EvalError> {
        self.skip_ws();
        match self.peek() {
            Some(b'(') => {
                self.pos += 1;
                let v = self.parse_expr()?;
                self.skip_ws();
                if self.peek() != Some(b')') {
                    return Err(EvalError::ExpectedCloseParen);
                }
                self.pos += 1;
                Ok(v)
            }
            Some(b'\'') => self.parse_char_lit(),
            Some(b'0')
                if self.pos + 1 < self.src.len()
                    && (self.src[self.pos + 1] == b'x' || self.src[self.pos + 1] == b'X') =>
            {
                self.pos += 2;
                self.parse_radix(16)
            }
            Some(b'0')
                if self.pos + 1 < self.src.len()
                    && (self.src[self.pos + 1] == b'b' || self.src[self.pos + 1] == b'B') =>
            {
                self.pos += 2;
                self.parse_radix(2)
            }
            Some(c) if c.is_ascii_digit() => self.parse_decimal(),
            Some(c) if c.is_ascii_alphabetic() || c == b'_' || c == b'@' => self.parse_ident(),
            Some(c) => Err(EvalError::UnexpectedChar(c as char)),
            None => Err(EvalError::UnexpectedEnd),
        }
    }

    fn parse_char_lit(&mut self) -> Result<u16, EvalError> {
        self.pos += 1; // skip '
        if self.pos >= self.src.len() {
            return Err(EvalError::UnterminatedCharLiteral);
        }
        let ch = self.src[self.pos];
        self.pos += 1;
        if self.peek() != Some(b'\'') {
            return Err(EvalError::UnterminatedCharLiteral);
        }
        self.pos += 1;
        Ok(ch as u16)
    }

    /// The scanned range is ASCII by construction (the scanners above only
    /// advance over ASCII bytes), so this cannot fail on well-formed input.
    fn scanned_str(&self, start: usize) -> Result<&str, EvalError> {
        std::str::from_utf8(&self.src[start..self.pos])
            .map_err(|_| EvalError::InvalidNumber("non-ASCII bytes".to_string()))
    }

    fn parse_decimal(&mut self) -> Result<u16, EvalError> {
        let start = self.pos;
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let s = self.scanned_str(start)?;
        s.parse::<u16>()
            .map_err(|e| EvalError::InvalidNumber(e.to_string()))
    }

    fn parse_radix(&mut self, radix: u32) -> Result<u16, EvalError> {
        let start = self.pos;
        while self.pos < self.src.len() && (self.src[self.pos] as char).is_digit(radix) {
            self.pos += 1;
        }
        let s = self.scanned_str(start)?;
        if s.is_empty() {
            return Err(EvalError::EmptyNumericLiteral);
        }
        u16::from_str_radix(s, radix).map_err(|e| EvalError::InvalidNumber(e.to_string()))
    }

    fn parse_ident(&mut self) -> Result<u16, EvalError> {
        let start = self.pos;
        while self.pos < self.src.len() && {
            let c = self.src[self.pos];
            c.is_ascii_alphanumeric() || c == b'_' || c == b'@'
        } {
            self.pos += 1;
        }
        let name = self.scanned_str(start)?;
        self.symbols
            .get(name)
            .copied()
            .ok_or_else(|| EvalError::UndefinedSymbol(name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn syms(pairs: &[(&str, u16)]) -> HashMap<String, u16> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn literals() {
        let s = syms(&[]);
        assert_eq!(eval_expr("42", &s), Ok(42));
        assert_eq!(eval_expr("0xFF", &s), Ok(255));
        assert_eq!(eval_expr("0b1010", &s), Ok(10));
        assert_eq!(eval_expr("'A'", &s), Ok(65));
    }

    #[test]
    fn arithmetic() {
        let s = syms(&[]);
        assert_eq!(eval_expr("2 + 3", &s), Ok(5));
        assert_eq!(eval_expr("10 - 3", &s), Ok(7));
        assert_eq!(eval_expr("3 * 4", &s), Ok(12));
        assert_eq!(eval_expr("8 / 2", &s), Ok(4));
        assert_eq!(eval_expr("1 << 3", &s), Ok(8));
        assert_eq!(eval_expr("16 >> 2", &s), Ok(4));
        assert_eq!(eval_expr("(2 + 3) * 4", &s), Ok(20));
    }

    #[test]
    fn symbols() {
        let s = syms(&[("FOO", 10), ("BAR", 5)]);
        assert_eq!(eval_expr("FOO + BAR", &s), Ok(15));
        assert_eq!(eval_expr("FOO * 2", &s), Ok(20));
    }
}
