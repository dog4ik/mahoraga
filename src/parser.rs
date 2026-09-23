use std::collections::HashMap;

use crate::{
    Error,
    lex::{Atom, Ident, Lexer, Punct, Tok, Token},
    punct_tok,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
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
    ArrayLit(Vec<Node>),
    ObjectLit(HashMap<String, Node>),
    Atom(Atom),
}

#[derive(Debug)]
struct Parser {
    lex: Lexer,
}

impl Parser {
    fn parse_comma_separated(&mut self, terminator: Punct) -> crate::Result<Vec<Node>> {
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
        self.lex.expect_next(Tok::Punct(terminator))?;
        Ok(items)
    }

    fn parse_object(&mut self) -> crate::Result<Node> {
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
        self.lex.expect_next(Tok::Punct(Punct::CloseBrace))?;
        Ok(Node::ObjectLit(entries))
    }

    pub fn parse_expr(&mut self, min_bind_power: u8) -> crate::Result<Node> {
        let mut lhs = match self.lex.advance() {
            Some(Token {
                tok: Tok::Atom(at), ..
            }) => Node::Atom(at),
            Some(Token {
                tok: Tok::Punct(Punct::OpenParen),
                ..
            }) => {
                let lhs = self.parse_expr(0)?;
                self.lex.expect_next(Tok::Punct(Punct::CloseParen))?;
                lhs
            }
            Some(Token {
                tok: Tok::Punct(Punct::OpenBracket),
                ..
            }) => Node::ArrayLit(self.parse_comma_separated(Punct::CloseBracket)?),

            Some(Token {
                tok: Tok::Punct(Punct::OpenBrace),
                ..
            }) => self.parse_object()?,
            Some(Token {
                tok: Tok::Punct(p @ (Punct::Add | Punct::Sub)),
                ..
            }) => {
                let ((), r_bp) = p.prefix_binding_power().expect("add/sub has prefix bp");
                let rhs = self.parse_expr(r_bp)?;
                Node::Op((p, (Box::new(Node::Atom(0.0.into())), Box::new(rhs))))
            }
            rest => {
                return Err(Error::new_from_parts(
                    format!("bad token, expected atom, got {rest:?}"),
                    rest.map(|v| v.span),
                ));
            }
        };

        loop {
            let punct = match self.lex.peek() {
                Some(Token {
                    tok: Tok::Punct(p), ..
                }) => p,
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
                        self.lex.expect_next(punct_tok!("]"))?;

                        lhs = Node::Index {
                            object: Box::new(lhs),
                            index: Box::new(index),
                        }
                    }

                    Punct::OpenParen => {
                        lhs = Node::Call {
                            callee: Box::new(lhs),
                            args: self.parse_comma_separated(Punct::CloseParen)?,
                        }
                    }

                    Punct::Dot => match self.lex.advance() {
                        Some(Token {
                            tok: Tok::Atom(Atom::Ident(field)),
                            ..
                        }) => {
                            lhs = Node::Member {
                                object: Box::new(lhs),
                                field,
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
                    lhs = Node::Turnary {
                        operand: Box::new(lhs),
                        truth_node: Box::new(mhs),
                        false_node: Box::new(rhs),
                    }
                } else {
                    let rhs = self.parse_expr(r_bp)?;
                    lhs = Node::Op((punct, (Box::new(lhs), Box::new(rhs))));
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

    fn op(punct: Punct, lhs: Node, rhs: Node) -> Node {
        Node::Op((punct, (Box::new(lhs), Box::new(rhs))))
    }

    fn ident(name: &str) -> Node {
        Node::Atom(Atom::Ident(Ident(name.into())))
    }

    #[test]
    fn basic_expr() -> crate::Result<()> {
        assert_eq!(parse_expr("12")?, Node::Atom(12.0.into()));
        Ok(())
    }

    #[test]
    fn basic_sum() -> crate::Result<()> {
        assert_eq!(
            parse_expr("12.4 + 13")?,
            op(Punct::Add, Node::Atom(12.4.into()), Node::Atom(13.0.into()))
        );
        Ok(())
    }

    #[test]
    fn basic_mul_presidence() -> crate::Result<()> {
        assert_eq!(
            parse_expr("12.4 + 13 * 2")?,
            op(
                Punct::Add,
                Node::Atom(12.4.into()),
                op(Punct::Mul, Node::Atom(13.0.into()), Node::Atom(2.0.into()))
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
                op(Punct::Mul, Node::Atom(12.4.into()), Node::Atom(13.0.into())),
                Node::Atom(2.0.into())
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
                Node::Atom(12.4.into()),
                op(Punct::Add, Node::Atom(13.0.into()), Node::Atom(2.0.into()))
            )
        );
        Ok(())
    }

    #[test]
    fn turnary_expr() -> crate::Result<()> {
        assert_eq!(
            parse_expr("1 ? 10 + 2 : \"true\"")?,
            Node::Turnary {
                operand: Box::new(Node::Atom(1.0.into())),
                truth_node: Box::new(op(
                    Punct::Add,
                    Node::Atom(10.0.into()),
                    Node::Atom(2.0.into())
                )),
                false_node: Box::new(Node::Atom("true".into())),
            }
        );
        Ok(())
    }

    #[test]
    fn path() -> crate::Result<()> {
        assert_eq!(
            parse_expr("test.mail.ru")?,
            Node::Member {
                object: Box::new(Node::Member {
                    object: Box::new(Node::Atom(Atom::Ident(Ident("test".into())))),
                    field: Ident("mail".into())
                }),
                field: Ident("ru".into()),
            }
        );
        Ok(())
    }

    #[test]
    fn cmp_equal() -> crate::Result<()> {
        assert_eq!(
            parse_expr("1 + 1 == 2")?,
            op(
                Punct::CmpEqual,
                op(Punct::Add, Node::Atom(1.0.into()), Node::Atom(1.0.into())),
                Node::Atom(2.0.into())
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
                ident("a"),
                op(Punct::And, ident("b"), ident("c"))
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
                op(Punct::Less, Node::Atom(1.0.into()), Node::Atom(2.0.into())),
                op(
                    Punct::CmpEqual,
                    Node::Atom(3.0.into()),
                    Node::Atom(3.0.into())
                )
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
                op(Punct::Pipe, ident("a"), ident("trim")),
                ident("upper")
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
                op(Punct::Add, Node::Atom(5.0.into()), Node::Atom(6.0.into()),),
                ident("trim")
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
                Node::Atom(5.0.into()),
                op(Punct::Pipe, Node::Atom(6.0.into()), ident("trim")),
            )
        );
        Ok(())
    }

    #[test]
    fn coalesce_parses() -> crate::Result<()> {
        assert_eq!(
            parse_expr("a ?? b")?,
            op(Punct::Coalesce, ident("a"), ident("b"))
        );
        Ok(())
    }

    #[test]
    fn path_index() -> crate::Result<()> {
        assert_eq!(
            parse_expr("a[b.test]")?,
            Node::Index {
                object: Box::new(ident("a")),
                index: Box::new(Node::Member {
                    object: Box::new(ident("b")),
                    field: Ident("test".into())
                })
            }
        );
        Ok(())
    }

    #[test]
    fn path_member() -> crate::Result<()> {
        assert_eq!(
            parse_expr("a.b.c")?,
            Node::Member {
                object: Box::new(Node::Member {
                    object: Box::new(ident("a")),
                    field: Ident("b".into())
                }),
                field: Ident("c".into())
            }
        );
        Ok(())
    }

    #[test]
    fn array_literal_elements_are_full_expressions() -> crate::Result<()> {
        assert_eq!(
            parse_expr("[1, a.b, c ? 2 : 3, (1 + 2) * 3]")?,
            Node::ArrayLit(vec![
                Node::Atom(1.0.into()),
                Node::Member {
                    object: Box::new(ident("a")),
                    field: Ident("b".into())
                },
                Node::Turnary {
                    operand: Box::new(ident("c")),
                    truth_node: Box::new(Node::Atom(2.0.into())),
                    false_node: Box::new(Node::Atom(3.0.into())),
                },
                op(
                    Punct::Mul,
                    op(Punct::Add, Node::Atom(1.0.into()), Node::Atom(2.0.into())),
                    Node::Atom(3.0.into())
                ),
            ])
        );
        Ok(())
    }

    #[test]
    fn empty_array_literal() -> crate::Result<()> {
        assert_eq!(parse_expr("[]")?, Node::ArrayLit(vec![]));
        assert_eq!(
            parse_expr("[[], []]")?,
            Node::ArrayLit(vec![Node::ArrayLit(vec![]), Node::ArrayLit(vec![])])
        );
        assert_eq!(
            parse_expr("[][0]")?,
            Node::Index {
                object: Box::new(Node::ArrayLit(vec![])),
                index: Box::new(Node::Atom(0.0.into())),
            }
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
            Node::ObjectLit(HashMap::from_iter([
                ("foo".into(), Node::Atom(Atom::StrLit("bar".into()))),
                ("baz".into(), Node::Atom(Atom::NumLit(43.)))
            ]))
        );
        Ok(())
    }

    fn call(callee: Node, args: Vec<Node>) -> Node {
        Node::Call {
            callee: Box::new(callee),
            args,
        }
    }

    #[test]
    fn fn_call() -> crate::Result<()> {
        assert_eq!(
            parse_expr(r#"foo(1.0, "baz")"#)?,
            call(
                ident("foo"),
                vec![Node::Atom(1.0.into()), Node::Atom("baz".into())]
            )
        );
        Ok(())
    }

    #[test]
    fn empty_arguments_list() -> crate::Result<()> {
        assert_eq!(parse_expr("foo()")?, call(ident("foo"), vec![]));
        assert_eq!(
            parse_expr("foo(bar())")?,
            call(ident("foo"), vec![call(ident("bar"), vec![])])
        );
        Ok(())
    }

    #[test]
    fn arguments_are_full_expressions() -> crate::Result<()> {
        assert_eq!(
            parse_expr("foo(a.b, c ? 1 : 2, (1 + 2) * 3, [4], bar(5))")?,
            call(
                ident("foo"),
                vec![
                    Node::Member {
                        object: Box::new(ident("a")),
                        field: Ident("b".into())
                    },
                    Node::Turnary {
                        operand: Box::new(ident("c")),
                        truth_node: Box::new(Node::Atom(1.0.into())),
                        false_node: Box::new(Node::Atom(2.0.into())),
                    },
                    op(
                        Punct::Mul,
                        op(Punct::Add, Node::Atom(1.0.into()), Node::Atom(2.0.into())),
                        Node::Atom(3.0.into())
                    ),
                    Node::ArrayLit(vec![Node::Atom(4.0.into())]),
                    call(ident("bar"), vec![Node::Atom(5.0.into())]),
                ]
            )
        );
        Ok(())
    }

    #[test]
    fn calls_chain_with_member_and_index() -> crate::Result<()> {
        assert_eq!(
            parse_expr("a.b(1)[0](2)")?,
            call(
                Node::Index {
                    object: Box::new(call(
                        Node::Member {
                            object: Box::new(ident("a")),
                            field: Ident("b".into())
                        },
                        vec![Node::Atom(1.0.into())]
                    )),
                    index: Box::new(Node::Atom(0.0.into()))
                },
                vec![Node::Atom(2.0.into())]
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
                Node::Atom(1.0.into()),
                op(
                    Punct::Mul,
                    call(ident("foo"), vec![Node::Atom(2.0.into())]),
                    Node::Atom(3.0.into())
                )
            )
        );
        assert_eq!(
            parse_expr("-foo(2)")?,
            op(
                Punct::Sub,
                Node::Atom(0.0.into()),
                call(ident("foo"), vec![Node::Atom(2.0.into())])
            )
        );
        Ok(())
    }

    #[test]
    fn call_is_piped_into() -> crate::Result<()> {
        assert_eq!(
            parse_expr("a | join(\' \') | trim")?,
            op(
                Punct::Pipe,
                op(
                    Punct::Pipe,
                    ident("a"),
                    call(ident("join"), vec![Node::Atom(" ".into())])
                ),
                ident("trim")
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
}
