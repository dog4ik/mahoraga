use std::{
    collections::HashMap,
    fmt::{Display, Write},
};

use crate::{
    Error, Span, facts,
    lex::{Atom, Ident, Lexer, Punct, Tok, Token},
    punct_tok,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Full node span rega
    pub span: Span,
    pub kind: NodeKind,
}

impl Node {
    pub fn collect_facts(&self) -> facts::Facts {
        let mut facts = facts::Facts::default();
        facts::collect_facts(self, &mut facts);
        facts
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            NodeKind::Op((op, (lhs, rhs))) => write!(f, "{lhs} {op} {rhs}"),
            NodeKind::Turnary {
                operand,
                truth_node,
                false_node,
            } => {
                write!(f, "{operand} ? {truth_node} : {false_node}")
            }
            NodeKind::Member { object, field } => {
                write!(f, "{object}.{field}")
            }
            NodeKind::Index { object, index } => {
                write!(f, "{object}[{index}]")
            }
            NodeKind::Call { callee, args } => {
                write!(f, "{callee}(",)?;
                if args.is_empty() {
                    f.write_char(')')?;
                }
                for arg in args.iter().take(args.len() - 1) {
                    write!(f, "{arg}, ")?;
                }
                write!(f, "{})", args.last().expect("length checked above"))
            }
            NodeKind::Negation(node) => write!(f, "!{node}"),
            NodeKind::ArrayLit(nodes) => {
                f.write_char('[')?;
                if nodes.is_empty() {
                    f.write_char(']')?;
                }
                for arg in nodes.iter().take(nodes.len() - 1) {
                    write!(f, "{arg}, ")?;
                }
                write!(f, "{}]", nodes.last().expect("length checked above"))
            }
            NodeKind::ObjectLit(hash_map) => {
                f.write_char('{')?;
                if hash_map.is_empty() {
                    f.write_char('}')?;
                }
                let mut iter = hash_map.iter().take(hash_map.len() - 1);
                for (key, value) in &mut iter {
                    write!(f, "\"{key}\": {value}, ")?;
                }
                let (last_key, last_value) = iter.next().expect("length checked above");
                write!(f, "\"{last_key}\": {last_value}}}")
            }
            NodeKind::Atom(atom) => write!(f, "{atom}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Op((Punct, (Box<Node>, Box<Node>))),
    Turnary {
        operand: Box<Node>,
        truth_node: Box<Node>,
        false_node: Box<Node>,
    },
    Member {
        object: Box<Node>,
        field: Ident,
    },
    Index {
        object: Box<Node>,
        index: Box<Node>,
    },
    Call {
        callee: Box<Node>,
        args: Vec<Node>,
    },
    Negation(Box<Node>),
    ArrayLit(Vec<Node>),
    ObjectLit(HashMap<String, Node>),
    Atom(Atom),
}

#[derive(Debug)]
struct Parser {
    lex: Lexer,
}

impl Parser {
    /// Parse comma separated list.
    ///
    /// Returns parsed nodes and terminator token
    fn parse_comma_separated(&mut self, terminator: Punct) -> crate::Result<(Vec<Node>, Token)> {
        let mut items = Vec::new();
        if !matches!(self.lex.peek(), Some(Token { tok: Tok::Punct(p), .. }) if p == terminator) {
            items.push(self.parse_expr(0)?);
            while let Some(Token {
                tok: punct_tok!(","),
                ..
            }) = self.lex.peek()
            {
                self.lex.advance();
                items.push(self.parse_expr(0)?);
            }
        }
        let terminator_token = self.lex.expect_next(Tok::Punct(terminator))?;
        Ok((items, terminator_token))
    }

    /// Parse object literal.
    ///
    /// Returns Object node and terminator token
    fn parse_object(&mut self) -> crate::Result<(NodeKind, Token)> {
        let mut entries = HashMap::new();
        if !matches!(
            self.lex.peek(),
            Some(Token {
                tok: punct_tok!("}"),
                ..
            })
        ) {
            let key = match self.lex.advance() {
                Some(Token {
                    tok: Tok::Atom(Atom::StrLit(s)),
                    ..
                }) => s,
                rest => {
                    return Err(Error::new_from_parts(
                        format!("map key must be string literal, got {rest:?}"),
                        rest.map(|v| v.span),
                    ));
                }
            };
            self.lex.expect_next(punct_tok!(":"))?;
            let value = self.parse_expr(0)?;
            entries.insert(key, value);
            while let Some(Token {
                tok: punct_tok!(","),
                ..
            }) = self.lex.peek()
            {
                self.lex.advance();
                let key = match self.lex.advance() {
                    Some(Token {
                        tok: Tok::Atom(Atom::StrLit(s)),
                        ..
                    }) => s,
                    rest => {
                        return Err(Error::new_from_parts(
                            format!("map key must be string literal, got {rest:?}"),
                            rest.map(|v| v.span),
                        ));
                    }
                };
                self.lex.expect_next(punct_tok!(":"))?;
                let value = self.parse_expr(0)?;
                entries.insert(key, value);
            }
        }
        let end = self.lex.expect_next(Tok::Punct(Punct::CloseBrace))?;
        Ok((NodeKind::ObjectLit(entries), end))
    }

    pub fn parse_expr(&mut self, min_bind_power: u8) -> crate::Result<Node> {
        let Some(Token {
            tok,
            span: start_span,
        }) = self.lex.advance()
        else {
            return Err(Error::new_with_span(
                "expected atom, got EOF".to_string(),
                Span {
                    start: self.lex.pos(),
                    end: self.lex.pos(),
                },
            ));
        };

        let mut lhs = match tok {
            Tok::Atom(at) => Node {
                kind: NodeKind::Atom(at),
                span: start_span,
            },
            Tok::Punct(Punct::OpenParen) => {
                let mut lhs = self.parse_expr(0)?;
                let close = self.lex.expect_next(Tok::Punct(Punct::CloseParen))?;
                lhs.span.start = start_span.start;
                lhs.span.end = close.span.end;
                lhs
            }
            Tok::Punct(Punct::OpenBracket) => {
                let (array_elements, terminator) =
                    self.parse_comma_separated(Punct::CloseBracket)?;
                Node {
                    kind: NodeKind::ArrayLit(array_elements),
                    span: start_span.merge(terminator.span),
                }
            }

            Tok::Punct(Punct::OpenBrace) => {
                let (object, terminator) = self.parse_object()?;
                Node {
                    kind: object,
                    span: start_span.merge(terminator.span),
                }
            }
            Tok::Punct(p @ (Punct::Add | Punct::Sub | Punct::Bang)) => {
                let ((), r_bp) = p
                    .prefix_binding_power()
                    .expect("add/sub/negation has prefix bp");
                let rhs = self.parse_expr(r_bp)?;
                if p == Punct::Bang {
                    Node {
                        span: start_span.merge(rhs.span),
                        kind: NodeKind::Negation(Box::new(rhs)),
                    }
                } else {
                    Node {
                        span: start_span.merge(rhs.span),
                        kind: NodeKind::Op((
                            p,
                            (
                                Box::new(Node {
                                    span: start_span.merge(rhs.span),
                                    kind: NodeKind::Atom(0.into()),
                                }),
                                Box::new(rhs),
                            ),
                        )),
                    }
                }
            }
            _ => {
                return Err(Error::new_with_span(
                    format!("bad token, expected atom, got {tok:?}"),
                    start_span,
                ));
            }
        };

        loop {
            let (punct, _start_span) = match self.lex.peek() {
                Some(Token {
                    tok: Tok::Punct(p),
                    span,
                }) => (p, span),
                Some(t) => {
                    return Err(Error::new_with_span(
                        format!("expected punct token, got {t}"),
                        t.span,
                    ));
                }
                None => break,
            };

            if let Some((l_bp, ())) = punct.postfix_binding_power() {
                if l_bp < min_bind_power {
                    break;
                }
                self.lex.advance();
                match punct {
                    Punct::OpenBracket => {
                        let index = self.parse_expr(0)?;
                        let end_token = self.lex.expect_next(punct_tok!("]"))?;

                        lhs = Node {
                            span: lhs.span.merge(end_token.span),
                            kind: NodeKind::Index {
                                object: Box::new(lhs),
                                index: Box::new(index),
                            },
                        }
                    }

                    Punct::OpenParen => {
                        let (args, terminator) = self.parse_comma_separated(Punct::CloseParen)?;
                        lhs = Node {
                            span: lhs.span.merge(terminator.span),
                            kind: NodeKind::Call {
                                callee: Box::new(lhs),
                                args,
                            },
                        }
                    }

                    Punct::Dot => match self.lex.advance() {
                        Some(Token {
                            tok: Tok::Atom(Atom::Ident(field)),
                            span: ident_span,
                        }) => {
                            lhs = Node {
                                span: lhs.span.merge(ident_span),
                                kind: NodeKind::Member {
                                    object: Box::new(lhs),
                                    field,
                                },
                            }
                        }
                        rest => {
                            return Err(Error::new_from_parts(
                                format!("expected member accessor, got: {rest:?}"),
                                rest.map(|v| v.span),
                            ));
                        }
                    },
                    _ => unreachable!("all postfix operators must be handled, got '{punct}'"),
                }
                continue;
            }

            if let Some((l_bp, r_bp)) = punct.infix_binding_power() {
                if l_bp < min_bind_power {
                    break;
                }
                self.lex.advance();

                if punct == Punct::Question {
                    let mhs = self.parse_expr(0)?;
                    self.lex.expect_next(punct_tok!(":"))?;
                    let rhs = self.parse_expr(r_bp)?;
                    lhs = Node {
                        span: lhs.span.merge(rhs.span),
                        kind: NodeKind::Turnary {
                            operand: Box::new(lhs),
                            truth_node: Box::new(mhs),
                            false_node: Box::new(rhs),
                        },
                    }
                } else {
                    let rhs = self.parse_expr(r_bp)?;
                    lhs = Node {
                        span: lhs.span.merge(rhs.span),
                        kind: NodeKind::Op((punct, (Box::new(lhs), Box::new(rhs)))),
                    };
                }
                continue;
            }
            break;
        }
        Ok(lhs)
    }
}

pub fn parse_expr(input: &str) -> crate::Result<Node> {
    let lex = Lexer::new(input)?;
    let mut parser = Parser { lex };
    let node = parser.parse_expr(0)?;
    parser.lex.expect_eof()?;
    Ok(node)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Default)]
    struct Spanner {
        pos: usize,
    }

    impl Spanner {
        pub fn new(initial: &str) -> Self {
            Self { pos: initial.len() }
        }

        pub fn span_for(&mut self, s: &str) -> Span {
            let start = self.pos;
            self.pos += s.len();
            Span {
                start,
                end: self.pos,
            }
        }

        pub fn span_with_offset(&mut self, s: &str, offset: &str) -> Span {
            let start = self.pos;
            self.pos += s.len();
            let span = Span {
                start,
                end: self.pos,
            };
            self.pos += offset.len();
            span
        }
    }

    fn op(punct: Punct, lhs: Node, rhs: Node, span: impl Into<Span>) -> Node {
        Node {
            span: span.into(),
            kind: NodeKind::Op((punct, (Box::new(lhs), Box::new(rhs)))),
        }
    }

    fn ident(name: &str, span: impl Into<Span>) -> Node {
        Node {
            span: span.into(),
            kind: NodeKind::Atom(Atom::Ident(Ident(name.into()))),
        }
    }

    fn atom(atom: impl Into<Atom>, span: impl Into<Span>) -> Node {
        Node {
            span: span.into(),
            kind: NodeKind::Atom(atom.into()),
        }
    }

    fn member(object: Node, field: &str, span: impl Into<Span>) -> Node {
        Node {
            span: span.into(),
            kind: NodeKind::Member {
                object: Box::new(object),
                field: Ident(field.into()),
            },
        }
    }

    fn index(object: Node, index: Node, span: impl Into<Span>) -> Node {
        Node {
            span: span.into(),
            kind: NodeKind::Index {
                object: Box::new(object),
                index: Box::new(index),
            },
        }
    }

    fn turnary(operand: Node, truth_node: Node, false_node: Node, span: impl Into<Span>) -> Node {
        Node {
            span: span.into(),
            kind: NodeKind::Turnary {
                operand: Box::new(operand),
                truth_node: Box::new(truth_node),
                false_node: Box::new(false_node),
            },
        }
    }

    fn array(items: Vec<Node>, span: impl Into<Span>) -> Node {
        Node {
            span: span.into(),
            kind: NodeKind::ArrayLit(items),
        }
    }

    fn call(callee: Node, args: Vec<Node>, span: impl Into<Span>) -> Node {
        Node {
            span: span.into(),
            kind: NodeKind::Call {
                callee: Box::new(callee),
                args,
            },
        }
    }

    fn negation(node: Node, span: impl Into<Span>) -> Node {
        Node {
            span: span.into(),
            kind: NodeKind::Negation(Box::new(node)),
        }
    }

    #[test]
    fn basic_expr() -> crate::Result<()> {
        assert_eq!(
            parse_expr("12")?,
            Node {
                kind: NodeKind::Atom(12.into()),
                span: Span::new(0..2)
            }
        );
        Ok(())
    }

    #[test]
    fn basic_sum() -> crate::Result<()> {
        let mut spanner = Spanner::default();
        let expr = "12.4 + 13";
        assert_eq!(
            parse_expr(expr)?,
            Node {
                kind: NodeKind::Op((
                    Punct::Add,
                    (
                        Box::new(Node {
                            kind: NodeKind::Atom(12.4.into()),
                            span: spanner.span_with_offset("12.4", " + ")
                        }),
                        Box::new(Node {
                            kind: NodeKind::Atom(13.into()),
                            span: spanner.span_for("13")
                        })
                    )
                )),
                span: Spanner::default().span_for(expr),
            }
        );
        Ok(())
    }

    #[test]
    fn basic_mul_presidence() -> crate::Result<()> {
        let mut spanner = Spanner::default();
        let expr = "12.4 + 13 * 2";
        assert_eq!(
            parse_expr(expr)?,
            op(
                Punct::Add,
                Node {
                    kind: NodeKind::Atom(12.4.into()),
                    span: spanner.span_with_offset("12.4", " + ")
                },
                op(
                    Punct::Mul,
                    Node {
                        kind: NodeKind::Atom(13.into()),
                        span: spanner.span_with_offset("13", " * ")
                    },
                    Node {
                        kind: NodeKind::Atom(2.into()),
                        span: spanner.span_for("2")
                    },
                    Spanner::new("12.4 + ").span_for("13 * 2")
                ),
                Spanner::default().span_for(expr)
            )
        );
        Ok(())
    }

    #[test]
    fn basic_mul_presidence_rev() -> crate::Result<()> {
        assert_eq!(
            parse_expr("12.4 * 13 + 2")?,
            op(
                Punct::Add,
                op(
                    Punct::Mul,
                    Node {
                        kind: NodeKind::Atom(12.4.into()),
                        span: (0..4).into()
                    },
                    Node {
                        kind: NodeKind::Atom(13.into()),
                        span: (7..9).into()
                    },
                    Span::new(0..9)
                ),
                Node {
                    kind: NodeKind::Atom(2.into()),
                    span: (12..13).into()
                },
                Span::new(0..13),
            )
        );
        Ok(())
    }

    #[test]
    fn basic_parents() -> crate::Result<()> {
        assert_eq!(
            parse_expr("12.4 * (13 + 2)")?,
            op(
                Punct::Mul,
                atom(12.4, 0..4),
                op(Punct::Add, atom(13, 8..10), atom(2, 13..14), 7..15),
                0..15
            )
        );
        Ok(())
    }

    #[test]
    fn turnary_expr() -> crate::Result<()> {
        assert_eq!(
            parse_expr("1 ? 10 + 2 : \"true\"")?,
            turnary(
                atom(1, 0..1),
                op(Punct::Add, atom(10, 4..6), atom(2, 9..10), 4..10),
                atom("true", 13..19),
                0..19
            )
        );
        Ok(())
    }

    #[test]
    fn path() -> crate::Result<()> {
        assert_eq!(
            parse_expr("test.mail.ru")?,
            member(member(ident("test", 0..4), "mail", 0..9), "ru", 0..12)
        );
        Ok(())
    }

    #[test]
    fn cmp_equal() -> crate::Result<()> {
        assert_eq!(
            parse_expr("1 + 1 == 2")?,
            op(
                Punct::CmpEqual,
                op(Punct::Add, atom(1, 0..1), atom(1, 4..5), 0..5),
                atom(2, 9..10),
                0..10
            )
        );
        Ok(())
    }

    #[test]
    fn and_binds_tighter_than_or() -> crate::Result<()> {
        assert_eq!(
            parse_expr("a || b && c")?,
            op(
                Punct::Or,
                ident("a", 0..1),
                op(Punct::And, ident("b", 5..6), ident("c", 10..11), 5..11),
                0..11
            )
        );
        Ok(())
    }

    #[test]
    fn comparison_binds_tighter_than_and() -> crate::Result<()> {
        assert_eq!(
            parse_expr("1 < 2 && 3 == 3")?,
            op(
                Punct::And,
                op(Punct::Less, atom(1, 0..1), atom(2, 4..5), 0..5),
                op(Punct::CmpEqual, atom(3, 9..10), atom(3, 14..15), 9..15),
                0..15
            )
        );
        Ok(())
    }

    #[test]
    fn pipes_chain_left_to_right() -> crate::Result<()> {
        assert_eq!(
            parse_expr("a | trim | upper")?,
            op(
                Punct::Pipe,
                op(Punct::Pipe, ident("a", 0..1), ident("trim", 4..8), 0..8),
                ident("upper", 11..16),
                0..16
            )
        );
        Ok(())
    }

    #[test]
    fn pipes_have_low_binding_power() -> crate::Result<()> {
        assert_eq!(
            parse_expr("5 + 6 | trim")?,
            op(
                Punct::Pipe,
                op(Punct::Add, atom(5, 0..1), atom(6, 4..5), 0..5),
                ident("trim", 8..12),
                0..12
            )
        );
        Ok(())
    }

    #[test]
    fn pipes_with_parents() -> crate::Result<()> {
        assert_eq!(
            parse_expr("5 + (6 | trim)")?,
            op(
                Punct::Add,
                atom(5, 0..1),
                op(Punct::Pipe, atom(6, 5..6), ident("trim", 9..13), 4..14),
                0..14
            )
        );
        Ok(())
    }

    #[test]
    fn coalesce_parses() -> crate::Result<()> {
        assert_eq!(
            parse_expr("a ?? b")?,
            op(Punct::Coalesce, ident("a", 0..1), ident("b", 5..6), 0..6)
        );
        Ok(())
    }

    #[test]
    fn path_index() -> crate::Result<()> {
        assert_eq!(
            parse_expr("a[b.test]")?,
            index(
                ident("a", 0..1),
                member(ident("b", 2..3), "test", 2..8),
                0..9
            )
        );
        Ok(())
    }

    #[test]
    fn path_member() -> crate::Result<()> {
        assert_eq!(
            parse_expr("a.b.c")?,
            member(member(ident("a", 0..1), "b", 0..3), "c", 0..5)
        );
        Ok(())
    }

    #[test]
    fn array_literal_elements_are_full_expressions() -> crate::Result<()> {
        assert_eq!(
            parse_expr("[1, a.b, c ? 2 : 3, (1 + 2) * 3]")?,
            array(
                vec![
                    atom(1, 1..2),
                    member(ident("a", 4..5), "b", 4..7),
                    turnary(ident("c", 9..10), atom(2, 13..14), atom(3, 17..18), 9..18),
                    op(
                        Punct::Mul,
                        op(Punct::Add, atom(1, 21..22), atom(2, 25..26), 20..27),
                        atom(3, 30..31),
                        20..31
                    ),
                ],
                0..32
            )
        );
        Ok(())
    }

    #[test]
    fn empty_array_literal() -> crate::Result<()> {
        assert_eq!(parse_expr("[]")?, array(vec![], 0..2));
        assert_eq!(
            parse_expr("[[], []]")?,
            array(vec![array(vec![], 1..3), array(vec![], 5..7)], 0..8)
        );
        assert_eq!(
            parse_expr("[][0]")?,
            index(array(vec![], 0..2), atom(0, 3..4), 0..5)
        );
        Ok(())
    }

    #[test]
    fn malformed_array_literals_error() {
        for src in [
            "[", "[,", "[1, 2", "[1 2]", "[1, 2]]", "[,]", "1, 2", "a[1, 2]",
        ] {
            assert!(parse_expr(src).is_err(), "{src} should not parse");
        }
    }

    #[test]
    fn multiline_object_lit() -> crate::Result<()> {
        assert_eq!(
            parse_expr(
                r#"{
"foo": "bar",
"baz": 43
}"#
            )?,
            Node {
                span: Span::new(0..27),
                kind: NodeKind::ObjectLit(HashMap::from_iter([
                    ("foo".into(), atom("bar", 9..14)),
                    ("baz".into(), atom(43, 23..25)),
                ]))
            }
        );
        Ok(())
    }

    #[test]
    fn fn_call() -> crate::Result<()> {
        assert_eq!(
            parse_expr(r#"foo(1.0, "baz")"#)?,
            call(
                ident("foo", 0..3),
                vec![atom(1.0, 4..7), atom("baz", 9..14)],
                0..15
            )
        );
        Ok(())
    }

    #[test]
    fn empty_arguments_list() -> crate::Result<()> {
        assert_eq!(parse_expr("foo()")?, call(ident("foo", 0..3), vec![], 0..5));
        assert_eq!(
            parse_expr("foo(bar())")?,
            call(
                ident("foo", 0..3),
                vec![call(ident("bar", 4..7), vec![], 4..9)],
                0..10
            )
        );
        Ok(())
    }

    #[test]
    fn arguments_are_full_expressions() -> crate::Result<()> {
        assert_eq!(
            parse_expr("foo(a.b, c ? 1 : 2, (1 + 2) * 3, [4], bar(5))")?,
            call(
                ident("foo", 0..3),
                vec![
                    member(ident("a", 4..5), "b", 4..7),
                    turnary(ident("c", 9..10), atom(1, 13..14), atom(2, 17..18), 9..18),
                    op(
                        Punct::Mul,
                        op(Punct::Add, atom(1, 21..22), atom(2, 25..26), 20..27),
                        atom(3, 30..31),
                        20..31
                    ),
                    array(vec![atom(4, 34..35)], 33..36),
                    call(ident("bar", 38..41), vec![atom(5, 42..43)], 38..44),
                ],
                0..45
            )
        );
        Ok(())
    }

    #[test]
    fn calls_chain_with_member_and_index() -> crate::Result<()> {
        assert_eq!(
            parse_expr("a.b(1)[0](2)")?,
            call(
                index(
                    call(
                        member(ident("a", 0..1), "b", 0..3),
                        vec![atom(1, 4..5)],
                        0..6
                    ),
                    atom(0, 7..8),
                    0..9
                ),
                vec![atom(2, 10..11)],
                0..12
            )
        );
        Ok(())
    }

    #[test]
    fn call_binds_tighter_than_operators() -> crate::Result<()> {
        assert_eq!(
            parse_expr("1 + foo(2) * 3")?,
            op(
                Punct::Add,
                atom(1, 0..1),
                op(
                    Punct::Mul,
                    call(ident("foo", 4..7), vec![atom(2, 8..9)], 4..10),
                    atom(3, 13..14),
                    4..14
                ),
                0..14
            )
        );
        assert_eq!(
            parse_expr("-foo(2)")?,
            op(
                Punct::Sub,
                // synthetic zero operand of the unary minus spans the whole expression
                atom(0, 0..7),
                call(ident("foo", 1..4), vec![atom(2, 5..6)], 1..7),
                0..7
            )
        );
        Ok(())
    }

    #[test]
    fn call_is_piped_into() -> crate::Result<()> {
        assert_eq!(
            parse_expr("a | join(' ') | trim")?,
            op(
                Punct::Pipe,
                op(
                    Punct::Pipe,
                    ident("a", 0..1),
                    call(ident("join", 4..8), vec![atom(" ", 9..12)], 4..13),
                    0..13
                ),
                ident("trim", 16..20),
                0..20
            )
        );
        Ok(())
    }

    #[test]
    fn malformed_arguments_list_errors() {
        for src in [
            "foo(", "foo(1", "foo(1 2)", "foo(1,)", "foo(,)", "foo(,1)", "foo(1))", "foo(1;2)",
        ] {
            assert!(parse_expr(src).is_err(), "{src} should not parse");
        }
    }

    #[test]
    fn negation_of_object_field() -> crate::Result<()> {
        assert_eq!(
            parse_expr("!test.value")?,
            negation(member(ident("test", 1..5), "value", 1..11), 0..11)
        );
        Ok(())
    }

    #[test]
    fn negation_of_array() -> crate::Result<()> {
        assert_eq!(
            parse_expr("![1, 2, 3]")?,
            negation(
                array(vec![atom(1, 2..3), atom(2, 5..6), atom(3, 8..9)], 1..10),
                0..10
            )
        );
        Ok(())
    }
}
