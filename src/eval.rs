use crate::{
    Error,
    lex::{Atom, Ident},
    parser::Node,
    value::Value,
};

#[derive(Debug)]
pub struct PureFunction {
    name: &'static str,
    f: fn(val: Value) -> crate::Result<Value>,
}

#[derive(Debug, Default)]
pub struct Env {
    pub fns: Vec<PureFunction>,
}

mod std_fns {
    use crate::{Error, value::Value};

    pub fn to_uppercase(val: Value) -> crate::Result<Value> {
        match val {
            Value::String(s) => Ok(Value::String(s.to_uppercase())),
            _ => Err(Error::new("unexpected data type")),
        }
    }

    pub fn to_lowercase(val: Value) -> crate::Result<Value> {
        match val {
            Value::String(s) => Ok(Value::String(s.to_lowercase())),
            _ => Err(Error::new("unexpected data type")),
        }
    }
}

impl Env {
    fn std() -> Self {
        Self {
            fns: vec![
                PureFunction {
                    name: "upper",
                    f: std_fns::to_uppercase,
                },
                PureFunction {
                    name: "lower",
                    f: std_fns::to_lowercase,
                },
            ],
        }
    }
}

pub fn eval(node: Node) -> crate::Result<Value> {
    let env = Env::std();
    match node {
        Node::Op((punct, (lhs, rhs))) => match punct {
            crate::lex::Punct::Add => eval(*lhs)?.add(eval(*rhs)?),
            crate::lex::Punct::Mul => eval(*lhs)?.mul(eval(*rhs)?),
            crate::lex::Punct::Sub => eval(*lhs)?.sub(eval(*rhs)?),
            crate::lex::Punct::Div => eval(*lhs)?.div(eval(*rhs)?),
            crate::lex::Punct::Pipe => {
                let lhs = eval(*lhs)?;
                let ident = match *rhs {
                    Node::Atom(Atom::Ident(Ident(ident))) => ident,
                    _ => return Err(Error::new("rhs of pipe should be ident")),
                };
                let f = env
                    .fns
                    .iter()
                    .find(|n| ident == n.name)
                    .ok_or_else(|| Error::new("could not find function"))?;
                (f.f)(lhs)
            }
            _ => Err(Error::new(format!("can't eval {punct}"))),
        },
        Node::Atom(atom) => match atom {
            crate::lex::Atom::Ident(_) => Err(Error::new("idents are not supported yet")),
            crate::lex::Atom::StrLit(s) => Ok(Value::String(s)),
            crate::lex::Atom::NumLit(n) => Ok(Value::Number(n)),
            crate::lex::Atom::BoolLit(b) => Ok(Value::Bool(b)),
        },
        Node::Turnary {
            operand,
            truth_node,
            false_node,
        } => {
            let operand = eval(*operand)?;
            if operand.truthy() {
                eval(*truth_node)
            } else {
                eval(*false_node)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_expr;

    #[test]
    fn basic_add() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("2 + 2")?)?, Value::Number(4.0));
        Ok(())
    }

    #[test]
    fn add_and_mult() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("(2 + 2) * 3")?)?, Value::Number(12.));
        Ok(())
    }

    #[test]
    fn turnary_true() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("true ? 1 : 2")?)?, Value::Number(1.));
        Ok(())
    }

    #[test]
    fn turnary_false() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("false ? 1 : 2")?)?, Value::Number(2.));
        Ok(())
    }

    #[test]
    fn nested_turnary_false() -> crate::Result<()> {
        assert_eq!(
            eval(parse_expr(
                "false ? 1 ? 10 + 10 : 0 : \"test\" ? 100 * 125 : 0"
            )?)?,
            Value::Number(100. * 125.)
        );
        Ok(())
    }
}
