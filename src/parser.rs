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
    ArrayLit(Vec<Node>),
    Atom(Atom),
}

#[derive(Debug)]
struct Parser {
    lex: Lexer,
}

impl Parser {
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
            }) => {
                let mut args = Vec::new();
                if !matches!(
                    self.lex.peek(),
                    Some(Token {
                        tok: punct_tok!("]"),
                        ..
                    })
                ) {
                    args.push(self.parse_expr(0)?);
                    while let Some(Token {
                        tok: punct_tok!(","),
                        ..
                    }) = self.lex.peek()
                    {
                        self.lex.advance();
                        args.push(self.parse_expr(0)?);
                    }
                }
                self.lex.expect_next(Tok::Punct(Punct::CloseBracket))?;
                Node::ArrayLit(args)
            }
            Some(Token {
                tok: Tok::Punct(p @ (Punct::Add | Punct::Sub)),
                ..
            }) => {
                let ((), r_bp) = p.prefix_binding_power().expect("add has prefix bp");
                let rhs = self.parse_expr(r_bp)?;
                Node::Op((p, (Box::new(Node::Atom(0.0.into())), Box::new(rhs))))
            }
            rest => {
                return Err(Error::new(format!(
                    "bad token, expected atom, got {rest:?}"
                )));
            }
        };

        loop {
            let punct = match self.lex.peek() {
                Some(Token {
                    tok: Tok::Punct(p), ..
                }) => p,
                Some(t) => return Err(Error::new(format!("expected punct token, got {t}"))),
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
                            return Err(Error::new(format!(
                                "expected member accessor, got: {rest:?}"
                            )));
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
}
