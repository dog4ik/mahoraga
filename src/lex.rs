use std::{
    fmt::{Display, Write},
    str::FromStr,
};

use crate::{Error, span::Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Atom(Atom),
    Punct(Punct),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} in {}", self.tok, self.span)
    }
}

impl Display for Tok {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Atom(a) => write!(f, "{a}"),
            Self::Punct(p) => write!(f, "{p}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Punct {
    Add,
    Mul,
    Sub,
    Div,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    OpenBrace,
    CloseBrace,
    Coalesce,
    Question,
    Colon,
    Dot,
    Pipe,
    Less,
    LessOrEq,
    More,
    MoreOrEq,
    CmpEqual,
    Or,
    And,
    Comma,
}

#[macro_export]
macro_rules! punct_tok {
    ("+") => {
        Tok::Punct(Punct::Add)
    };
    ("*") => {
        Tok::Punct(Punct::Mul)
    };
    ("-") => {
        Tok::Punct(Punct::Sub)
    };
    ("/") => {
        Tok::Punct(Punct::Div)
    };
    ("(") => {
        Tok::Punct(Punct::OpenParen)
    };
    (")") => {
        Tok::Punct(Punct::CloseParen)
    };
    ("?") => {
        Tok::Punct(Punct::Question)
    };
    (":") => {
        Tok::Punct(Punct::Colon)
    };
    ("??") => {
        Tok::Punct(Punct::Coalesce)
    };
    (".") => {
        Tok::Punct(Punct::Dot)
    };
    ("|") => {
        Tok::Punct(Punct::Pipe)
    };
    (">") => {
        Tok::Punct(Punct::More)
    };
    (">=") => {
        Tok::Punct(Punct::MoreOrEq)
    };
    ("<") => {
        Tok::Punct(Punct::Less)
    };
    ("<=") => {
        Tok::Punct(Punct::LessOrEq)
    };
    ("==") => {
        Tok::Punct(Punct::CmpEqual)
    };
    ("||") => {
        Tok::Punct(Punct::Or)
    };
    ("&&") => {
        Tok::Punct(Punct::And)
    };
    ("[") => {
        Tok::Punct(Punct::OpenBracket)
    };
    ("]") => {
        Tok::Punct(Punct::CloseBracket)
    };
    ("{") => {
        Tok::Punct(Punct::OpenBrace)
    };
    ("}") => {
        Tok::Punct(Punct::CloseBrace)
    };
    (",") => {
        Tok::Punct(Punct::Comma)
    };
}

impl Punct {
    pub fn infix_binding_power(&self) -> Option<(u8, u8)> {
        match self {
            Punct::Pipe => Some((1, 2)),
            Punct::Question => Some((4, 3)),
            Punct::Coalesce | Punct::Or => Some((5, 6)),
            Punct::And => Some((7, 8)),
            Punct::More | Punct::MoreOrEq | Punct::Less | Punct::LessOrEq | Punct::CmpEqual => {
                Some((9, 10))
            }
            Punct::Add | Punct::Sub => Some((11, 12)),
            Punct::Mul | Punct::Div => Some((13, 14)),
            _ => None,
        }
    }

    pub fn postfix_binding_power(&self) -> Option<(u8, ())> {
        match self {
            Punct::Dot | Punct::OpenBracket => Some((16, ())),
            _ => None,
        }
    }

    pub fn prefix_binding_power(&self) -> Option<((), u8)> {
        match self {
            Punct::Add | Punct::Sub => Some(((), 15)),
            _ => None,
        }
    }
}

impl Display for Punct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Punct::Add => f.write_char('+'),
            Punct::Mul => f.write_char('*'),
            Punct::Sub => f.write_char('-'),
            Punct::Div => f.write_char('/'),
            Punct::OpenParen => f.write_char('('),
            Punct::CloseParen => f.write_char(')'),
            Punct::Coalesce => f.write_str("??"),
            Punct::Question => f.write_char('?'),
            Punct::Colon => f.write_char(':'),
            Punct::Dot => f.write_char('.'),
            Punct::Pipe => f.write_char('|'),
            Punct::More => f.write_char('>'),
            Punct::MoreOrEq => f.write_str(">="),
            Punct::Less => f.write_char('<'),
            Punct::LessOrEq => f.write_str("<="),
            Punct::CmpEqual => f.write_str("=="),
            Punct::Or => f.write_str("||"),
            Punct::And => f.write_str("&&"),
            Punct::OpenBracket => f.write_char('['),
            Punct::CloseBracket => f.write_char(']'),
            Punct::OpenBrace => f.write_char('{'),
            Punct::CloseBrace => f.write_char('}'),
            Punct::Comma => f.write_char(','),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Ident(Ident),
    StrLit(String),
    NumLit(f64),
    BoolLit(bool),
}

impl From<f64> for Atom {
    fn from(value: f64) -> Self {
        Self::NumLit(value)
    }
}

impl From<String> for Atom {
    fn from(value: String) -> Self {
        Self::StrLit(value)
    }
}

impl From<&str> for Atom {
    fn from(value: &str) -> Self {
        Self::StrLit(value.to_string())
    }
}

impl From<bool> for Atom {
    fn from(value: bool) -> Self {
        Self::BoolLit(value)
    }
}

impl Display for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Atom::Ident(ident) => write!(f, "{ident}"),
            Atom::StrLit(s) => write!(f, "{s}"),
            Atom::NumLit(n) => write!(f, "{n}"),
            Atom::BoolLit(b) => write!(f, "{b}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident(pub String);

impl Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Ident {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

const IMPLICIT_SEPARATORS: &[u8] = b")+-/*|&[]{},:?";

#[derive(Debug)]
pub struct Lexer {
    tokens: Vec<Token>,
    /// Lexer position in the tokens
    pos: usize,
}

fn tokenize(input: &str) -> crate::Result<Vec<Token>> {
    let mut out = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < input.len() {
        let start = i;
        let current_span = |end: usize| Span { start, end };
        let character = bytes[i];
        if character.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        let tok = match character {
            b'+' => {
                i += 1;
                Tok::Punct(Punct::Add)
            }
            b'-' => {
                i += 1;
                Tok::Punct(Punct::Sub)
            }
            b'*' => {
                i += 1;
                Tok::Punct(Punct::Mul)
            }
            b'/' => {
                i += 1;
                Tok::Punct(Punct::Div)
            }
            b'(' => {
                i += 1;
                Tok::Punct(Punct::OpenParen)
            }
            b')' => {
                i += 1;
                Tok::Punct(Punct::CloseParen)
            }
            b'[' => {
                i += 1;
                Tok::Punct(Punct::OpenBracket)
            }
            b']' => {
                i += 1;
                Tok::Punct(Punct::CloseBracket)
            }
            b'{' => {
                i += 1;
                Tok::Punct(Punct::OpenBrace)
            }
            b'}' => {
                i += 1;
                Tok::Punct(Punct::CloseBrace)
            }
            b':' => {
                i += 1;
                Tok::Punct(Punct::Colon)
            }
            b'.' => {
                i += 1;
                Tok::Punct(Punct::Dot)
            }
            b',' => {
                i += 1;
                Tok::Punct(Punct::Comma)
            }
            b'|' => {
                i += 1;
                if bytes.get(i).is_some_and(|&b| b == b'|') {
                    i += 1;
                    Tok::Punct(Punct::Or)
                } else {
                    Tok::Punct(Punct::Pipe)
                }
            }
            b'&' => {
                i += 1;
                if bytes.get(i).is_some_and(|&b| b == b'&') {
                    i += 1;
                    Tok::Punct(Punct::And)
                } else {
                    return Err(Error::new_with_span("expected second &", current_span(i)));
                }
            }
            b'>' => {
                i += 1;
                if bytes.get(i).is_some_and(|&b| b == b'=') {
                    i += 1;
                    Tok::Punct(Punct::MoreOrEq)
                } else {
                    Tok::Punct(Punct::More)
                }
            }
            b'<' => {
                i += 1;
                if bytes.get(i).is_some_and(|&b| b == b'=') {
                    i += 1;
                    Tok::Punct(Punct::LessOrEq)
                } else {
                    Tok::Punct(Punct::Less)
                }
            }
            b'=' => {
                i += 1;
                if bytes.get(i).is_some_and(|&b| b == b'=') {
                    i += 1;
                    Tok::Punct(Punct::CmpEqual)
                } else {
                    return Err(Error::new_with_span("expected second =", current_span(i)));
                }
            }
            b'?' => {
                if let Some(b'?') = bytes.get(i + 1) {
                    i += 2;
                    Tok::Punct(Punct::Coalesce)
                } else {
                    i += 1;
                    Tok::Punct(Punct::Question)
                }
            }
            b'0'..=b'9' => {
                i += 1;
                let mut after_dot = false;
                while let Some(next) = bytes
                    .get(i)
                    .filter(|b| !b.is_ascii_whitespace())
                    .filter(|b| !IMPLICIT_SEPARATORS.contains(b))
                {
                    match next {
                        b'0'..=b'9' => {
                            i += 1;
                        }
                        b'_' => {
                            i += 1;
                            match bytes.get(i) {
                                Some(b'_') => {
                                    return Err(Error::new_with_span(
                                        "only one underscore is allowed as numeric separator",
                                        current_span(i),
                                    ));
                                }
                                Some(b'0'..=b'9') => {}
                                None => {
                                    return Err(Error::new_with_span(
                                        "numeric separators are not allowed at the end of numeric literals",
                                        current_span(i),
                                    ));
                                }
                                _ => {
                                    return Err(Error::new_with_span(
                                        "invalid or unexpected token",
                                        current_span(i),
                                    ));
                                }
                            }
                        }
                        b'.' if !after_dot => {
                            i += 1;
                            after_dot = true;
                        }
                        _ => {
                            return Err(crate::Error::new_with_span(
                                format!(
                                    "expected number character, got '{}'",
                                    str::from_utf8(&[*next]).unwrap(),
                                ),
                                current_span(i),
                            ));
                        }
                    }
                }
                let slice = input.get(start..i).ok_or_else(|| {
                    crate::Error::new_with_span("failed to slice string", current_span(i))
                })?;
                let num = f64::from_str(&slice.replace("_", "")).map_err(|e| {
                    crate::Error::new_with_span(
                        format!("failed to parse number: {e}"),
                        current_span(i),
                    )
                })?;
                Tok::Atom(Atom::NumLit(num))
            }
            b'a'..=b'z' | b'A'..=b'Z' => {
                i += 1;
                while bytes.get(i).is_some_and(|next| {
                    !next.is_ascii_whitespace()
                        && !IMPLICIT_SEPARATORS.contains(next)
                        && *next != b'.'
                }) {
                    i += 1;
                }

                let slice = input.get(start..i).ok_or_else(|| {
                    crate::Error::new_with_span("failed to slice string", current_span(i))
                })?;

                if slice == "true" {
                    Tok::Atom(Atom::BoolLit(true))
                } else if slice == "false" {
                    Tok::Atom(Atom::BoolLit(false))
                } else {
                    Tok::Atom(Atom::Ident(Ident(slice.to_string())))
                }
            }
            b'"' | b'\'' => {
                let str_quote = character;
                i += 1;
                let mut s = Vec::new();
                loop {
                    let Some(&c) = bytes.get(i) else {
                        return Err(Error::new_with_span(
                            "unclosed string literal",
                            current_span(i),
                        ));
                    };
                    if c == str_quote {
                        i += 1;
                        break;
                    }
                    if c == b'\\' {
                        let Some(&next) = bytes.get(i + 1) else {
                            return Err(Error::new_with_span(
                                "unclosed string literal",
                                current_span(i),
                            ));
                        };

                        s.push(next);
                        i += 2;
                    } else {
                        s.push(c);
                        i += 1;
                    }
                }

                let s = String::from_utf8(s).map_err(|e| {
                    Error::new_with_span(
                        format!("failed to construct string literal: {e}"),
                        current_span(i),
                    )
                })?;
                Tok::Atom(Atom::StrLit(s))
            }
            _ => {
                let end = i;
                let c = char::from_u32(character.into()).unwrap();
                return Err(crate::Error::new_with_span(
                    format!("unrecognized token: {c}"),
                    Span { start, end },
                ));
            }
        };

        out.push(Token {
            tok,
            span: current_span(i),
        })
    }

    Ok(out)
}

impl Lexer {
    pub fn new(input: &str) -> crate::Result<Lexer> {
        Ok(Lexer {
            tokens: tokenize(input)?,
            pos: 0,
        })
    }

    pub fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos)?;
        self.pos += 1;
        Some(token.clone())
    }

    pub fn expect_next(&mut self, tok: Tok) -> crate::Result<Token> {
        let token = self
            .tokens
            .get(self.pos)
            .ok_or_else(|| Error::new(format!("unexpected EOF, expected {tok}")))?;
        self.pos += 1;
        if token.tok != tok {
            return Err(Error::new_with_span(
                format!("unexpected token {}, expected: {tok}", token.tok),
                token.span,
            ));
        }
        Ok(token.clone())
    }

    pub fn expect_eof(&mut self) -> crate::Result<()> {
        match self.advance() {
            Some(tok) => Err(Error::new(format!("expected eof, got {tok}"))),
            None => Ok(()),
        }
    }

    pub fn peek(&self) -> Option<Token> {
        self.tokens.get(self.pos).cloned()
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use crate::lex::Atom;

    use super::*;

    #[test]
    fn parse_number() -> crate::Result<()> {
        assert_eq!(
            tokenize("10")?,
            vec![Token {
                tok: Tok::Atom(Atom::NumLit(10.)),
                span: Span { start: 0, end: 2 }
            }]
        );
        Ok(())
    }

    #[test]
    fn parse_number_float() -> crate::Result<()> {
        assert_eq!(
            tokenize("10.0393")?,
            vec![Token {
                tok: Tok::Atom(Atom::NumLit(10.0393)),
                span: Span { start: 0, end: 7 }
            }]
        );
        Ok(())
    }

    #[test]
    fn number_with_trailing_zeros() -> crate::Result<()> {
        assert_eq!(
            tokenize("10.039300")?,
            vec![Token {
                tok: Tok::Atom(Atom::NumLit(10.0393)),
                span: Span { start: 0, end: 9 }
            }]
        );
        Ok(())
    }

    #[test]
    fn invalid_number() -> crate::Result<()> {
        assert_matches!(tokenize("10.039300."), Err(_));
        Ok(())
    }

    #[test]
    fn punctstream() -> crate::Result<()> {
        assert_eq!(
            tokenize("+-/*()??value 11")?,
            vec![
                Token {
                    tok: punct_tok!("+"),
                    span: Span { start: 0, end: 1 },
                },
                Token {
                    tok: punct_tok!("-"),
                    span: Span { start: 1, end: 2 },
                },
                Token {
                    tok: punct_tok!("/"),
                    span: Span { start: 2, end: 3 },
                },
                Token {
                    tok: punct_tok!("*"),
                    span: Span { start: 3, end: 4 },
                },
                Token {
                    tok: punct_tok!("("),
                    span: Span { start: 4, end: 5 },
                },
                Token {
                    tok: punct_tok!(")"),
                    span: Span { start: 5, end: 6 },
                },
                Token {
                    tok: punct_tok!("??"),
                    span: Span { start: 6, end: 8 },
                },
                Token {
                    tok: Tok::Atom(Atom::Ident(Ident("value".into()))),
                    span: Span { start: 8, end: 13 },
                },
                Token {
                    tok: Tok::Atom(Atom::NumLit(11.)),
                    span: Span { start: 14, end: 16 },
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn coalesce() -> crate::Result<()> {
        assert_eq!(
            tokenize("??")?,
            vec![Token {
                tok: Tok::Punct(Punct::Coalesce),
                span: Span { start: 0, end: 2 }
            }]
        );
        Ok(())
    }

    #[test]
    fn string_lit() -> crate::Result<()> {
        assert_eq!(
            tokenize(r#""Hello world!""#)?,
            vec![Token {
                tok: Tok::Atom(Atom::StrLit(String::from("Hello world!"))),
                span: Span { start: 0, end: 14 }
            }]
        );
        Ok(())
    }

    #[test]
    fn string_lit_escaped() -> crate::Result<()> {
        assert_eq!(
            tokenize(r#""\"Hello world!\"""#)?,
            vec![Token {
                tok: Tok::Atom(Atom::StrLit(String::from(r#""Hello world!""#))),
                span: Span { start: 0, end: 18 }
            }]
        );
        Ok(())
    }

    #[test]
    fn string_lit_double_escape() -> crate::Result<()> {
        assert_eq!(
            tokenize(r#""\\\"Hello world!\\\"""#)?,
            vec![Token {
                tok: Tok::Atom(Atom::StrLit(String::from(r#"\"Hello world!\""#))),
                span: Span { start: 0, end: 22 }
            }]
        );
        Ok(())
    }

    #[test]
    fn invalid_escape() -> crate::Result<()> {
        assert_matches!(tokenize(r#"\\\alter"#), Err(_));
        Ok(())
    }

    #[test]
    fn invalid_literal() -> crate::Result<()> {
        assert_matches!(tokenize(r#"""#), Err(_));
        Ok(())
    }

    #[test]
    fn single_quote_literal() -> crate::Result<()> {
        assert_eq!(
            tokenize(r#"'hello'"#)?,
            vec![Token {
                tok: Tok::Atom(Atom::StrLit(String::from("hello"))),
                span: Span { start: 0, end: 7 }
            }]
        );
        Ok(())
    }

    #[test]
    fn mixed_quote_literal_fails() -> crate::Result<()> {
        assert_matches!(tokenize(r#"'hello""#), Err(_));
        Ok(())
    }

    #[test]
    fn single_quote_is_allowed_in_double_quotes() -> crate::Result<()> {
        assert_eq!(
            tokenize(r#""he'' ''ll'o""#)?,
            vec![Token {
                tok: Tok::Atom(Atom::StrLit(String::from(r#"he'' ''ll'o"#))),
                span: Span { start: 0, end: 13 }
            }]
        );
        Ok(())
    }

    #[test]
    fn double_quote_is_allowed_in_single_quotes() -> crate::Result<()> {
        assert_eq!(
            tokenize(r#"'"test" " test"'"#)?,
            vec![Token {
                tok: Tok::Atom(Atom::StrLit(String::from(r#""test" " test""#))),
                span: Span { start: 0, end: 16 }
            }]
        );
        Ok(())
    }

    #[test]
    fn trailing_escape_fails_gracefully() -> crate::Result<()> {
        assert_matches!(tokenize(r#""abc\"#), Err(_));
        Ok(())
    }

    #[test]
    fn unclosed_invalid_literal() -> crate::Result<()> {
        assert_matches!(tokenize(r#""value"#), Err(_));
        Ok(())
    }

    #[test]
    fn num_in_parens() -> crate::Result<()> {
        assert_eq!(
            tokenize(r#"(3)"#)?,
            vec![
                Token {
                    tok: punct_tok!("("),
                    span: Span { start: 0, end: 1 }
                },
                Token {
                    tok: Tok::Atom(3.0.into()),
                    span: Span { start: 1, end: 2 }
                },
                Token {
                    tok: punct_tok!(")"),
                    span: Span { start: 2, end: 3 }
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn ternary() -> crate::Result<()> {
        assert_eq!(
            tokenize(r#"19 ? 1 : 2"#)?,
            vec![
                Token {
                    tok: Tok::Atom(19.0.into()),
                    span: Span { start: 0, end: 2 }
                },
                Token {
                    tok: punct_tok!("?"),
                    span: Span { start: 3, end: 4 }
                },
                Token {
                    tok: Tok::Atom(1.0.into()),
                    span: Span { start: 5, end: 6 }
                },
                Token {
                    tok: punct_tok!(":"),
                    span: Span { start: 7, end: 8 }
                },
                Token {
                    tok: Tok::Atom(2.0.into()),
                    span: Span { start: 9, end: 10 }
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn partial_ternary() -> crate::Result<()> {
        assert_matches!(tokenize(r#"19 ? 1 : "truee"#), Err(_));
        Ok(())
    }

    #[test]
    fn path_ident() -> crate::Result<()> {
        assert_eq!(
            tokenize("foo.bar.baz")?,
            vec![
                Token {
                    tok: Tok::Atom(Atom::Ident(Ident("foo".into()))),
                    span: Span { start: 0, end: 3 }
                },
                Token {
                    tok: punct_tok!("."),
                    span: Span { start: 3, end: 4 }
                },
                Token {
                    tok: Tok::Atom(Atom::Ident(Ident("bar".into()))),
                    span: Span { start: 4, end: 7 }
                },
                Token {
                    tok: punct_tok!("."),
                    span: Span { start: 7, end: 8 }
                },
                Token {
                    tok: Tok::Atom(Atom::Ident(Ident("baz".into()))),
                    span: Span { start: 8, end: 11 }
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn less_or_equal() -> crate::Result<()> {
        assert_eq!(
            tokenize("1 <= 10")?,
            vec![
                Token {
                    tok: Tok::Atom(1.0.into()),
                    span: Span { start: 0, end: 1 }
                },
                Token {
                    tok: punct_tok!("<="),
                    span: Span { start: 2, end: 4 }
                },
                Token {
                    tok: Tok::Atom(10.0.into()),
                    span: Span { start: 5, end: 7 }
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn cmp_equal() -> crate::Result<()> {
        assert_eq!(
            tokenize("1 == 10")?,
            vec![
                Token {
                    tok: Tok::Atom(1.0.into()),
                    span: Span { start: 0, end: 1 }
                },
                Token {
                    tok: punct_tok!("=="),
                    span: Span { start: 2, end: 4 }
                },
                Token {
                    tok: Tok::Atom(10.0.into()),
                    span: Span { start: 5, end: 7 }
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn cmp_invalid_equal() -> crate::Result<()> {
        assert_matches!(tokenize("1 = 10"), Err(_));
        Ok(())
    }

    #[test]
    fn walk_lexer() -> crate::Result<()> {
        let mut lexer = Lexer::new("3 + 5.3 - 9.0")?;
        assert_matches!(
            lexer.advance(),
            Some(Token {
                tok: Tok::Atom(Atom::NumLit(3.)),
                span: _
            })
        );

        assert_matches!(
            lexer.peek(),
            Some(Token {
                tok: punct_tok!("+"),
                span: _
            })
        );

        assert_matches!(
            lexer.advance(),
            Some(Token {
                tok: punct_tok!("+"),
                span: _
            })
        );

        assert_matches!(
            lexer.advance(),
            Some(Token {
                tok: Tok::Atom(Atom::NumLit(5.3)),
                span: _
            })
        );

        assert_matches!(
            lexer.advance(),
            Some(Token {
                tok: punct_tok!("-"),
                span: _
            })
        );

        assert_matches!(
            lexer.advance(),
            Some(Token {
                tok: Tok::Atom(Atom::NumLit(9.)),
                span: _
            })
        );

        assert_matches!(lexer.advance(), None);
        assert_matches!(lexer.advance(), None);
        Ok(())
    }
}
