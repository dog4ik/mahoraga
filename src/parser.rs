use crate::{
    Error,
    lex::{Atom, Ident, Lexer, Punct, Tok, Token},
    punct_tok,
};

#[derive(Debug, PartialEq)]
pub enum Node {
    Op((Punct, (Box<Node>, Box<Node>))),
    Turnary {
        operand: Box<Node>,
        truth_node: Box<Node>,
        false_node: Box<Node>,
    },
    Path(Vec<Ident>),
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
                tok: Tok::Punct(p @ (Punct::Add | Punct::Sub)),
                ..
            }) => {
                let ((), r_bp) = p.prefix_binding_power().expect("add has prefix bp");
                let rhs = self.parse_expr(r_bp)?;
                Node::Op((p, (Box::new(Node::Atom(0.0.into())), Box::new(rhs))))
            }
            _ => return Err(Error::new("bad token, expected atom")),
        };

        loop {
            let punct = match self.lex.peek() {
                Some(Token {
                    tok: Tok::Punct(p), ..
                }) => p,
                Some(t) => return Err(Error::new(format!("expected punct token, got {t}"))),
                None => break,
            };

            if let Some((l_bp, r_bp)) = punct.infix_binding_power() {
                if l_bp < min_bind_power {
                    break;
                }
                self.lex.advance();

                if punct == Punct::Dot {
                    let ident = match self.lex.advance() {
                        Some(Token {
                            tok: Tok::Atom(Atom::Ident(ident)),
                            ..
                        }) => ident,
                        _ => {
                            return Err(Error::new("expected path or ident"));
                        }
                    };
                    match lhs {
                        Node::Atom(Atom::Ident(lhs_ident)) => {
                            lhs = Node::Path(vec![lhs_ident, ident])
                        }
                        Node::Path(ref mut path) => path.push(ident),
                        _ => return Err(Error::new("expected path or ident")),
                    }
                    continue;
                } else if punct == Punct::Question {
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
    parser.parse_expr(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_expr() -> crate::Result<()> {
        assert_eq!(parse_expr("12")?, Node::Atom(12.0.into()));
        Ok(())
    }

    #[test]
    fn basic_sum() -> crate::Result<()> {
        assert_eq!(
            parse_expr("12.4 + 13")?,
            Node::Op((
                Punct::Add,
                (
                    Box::new(Node::Atom(12.4.into())),
                    Box::new(Node::Atom(13.0.into()))
                )
            ))
        );
        Ok(())
    }

    #[test]
    fn basic_mul_presidence() -> crate::Result<()> {
        assert_eq!(
            parse_expr("12.4 + 13 * 2")?,
            Node::Op((
                Punct::Add,
                (
                    Box::new(Node::Atom(12.4.into())),
                    Box::new(Node::Op((
                        Punct::Mul,
                        (
                            Box::new(Node::Atom(13.0.into())),
                            Box::new(Node::Atom(2.0.into()))
                        )
                    )))
                )
            ))
        );
        Ok(())
    }

    #[test]
    fn basic_mul_presidence_rev() -> crate::Result<()> {
        assert_eq!(
            parse_expr("12.4 * 13 + 2")?,
            Node::Op((
                Punct::Add,
                (
                    Box::new(Node::Op((
                        Punct::Mul,
                        (
                            Box::new(Node::Atom(12.4.into())),
                            Box::new(Node::Atom(13.0.into()))
                        )
                    ))),
                    Box::new(Node::Atom(2.0.into())),
                )
            ))
        );
        Ok(())
    }

    #[test]
    fn basic_parents() -> crate::Result<()> {
        assert_eq!(
            parse_expr("12.4 * (13 + 2)")?,
            Node::Op((
                Punct::Mul,
                (
                    Box::new(Node::Atom(12.4.into())),
                    Box::new(Node::Op((
                        Punct::Add,
                        (
                            Box::new(Node::Atom(13.0.into())),
                            Box::new(Node::Atom(2.0.into()))
                        )
                    ))),
                )
            ))
        );
        Ok(())
    }

    #[test]
    fn turnary_expr() -> crate::Result<()> {
        assert_eq!(
            parse_expr("1 ? 10 + 2 : \"true\"")?,
            Node::Turnary {
                operand: Box::new(Node::Atom(1.0.into())),
                truth_node: Box::new(Node::Op((
                    Punct::Add,
                    (
                        Box::new(Node::Atom(10.0.into())),
                        Box::new(Node::Atom(2.0.into()))
                    )
                ))),
                false_node: Box::new(Node::Atom("true".into())),
            }
        );
        Ok(())
    }

    #[test]
    fn path() -> crate::Result<()> {
        assert_eq!(
            parse_expr("test.mail.ru")?,
            Node::Path(vec![
                Ident("test".into()),
                Ident("mail".into()),
                Ident("ru".into()),
            ])
        );
        Ok(())
    }
}
