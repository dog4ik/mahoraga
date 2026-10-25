use super::Result;
use crate::{
    Args, Atom, FunctionCallError, Ident, Node, NodeKind, Punct, RuntimeEnv, RuntimeError, Span,
    Spanned, Value,
    eval::closure::Closure,
    value::{Array, Number, Object, ValueType},
};

impl<'a> RuntimeEnv<'a> {
    fn eval_punct_node(
        &mut self,
        lhs: &Node,
        punct: Punct,
        rhs: &Node,
        span: &Span,
    ) -> Result<Value> {
        let spanitize = |e: RuntimeError| Spanned::new(e, *span);
        match punct {
            Punct::Add => self.eval(lhs)?.add(&self.eval(rhs)?).map_err(spanitize),
            Punct::Mul => self.eval(lhs)?.mul(&self.eval(rhs)?).map_err(spanitize),
            Punct::Sub => self.eval(lhs)?.sub(&self.eval(rhs)?).map_err(spanitize),
            Punct::Div => self.eval(lhs)?.div(&self.eval(rhs)?).map_err(spanitize),
            Punct::Less => self.eval(lhs)?.lt(&self.eval(rhs)?).map_err(spanitize),
            Punct::LessOrEq => self.eval(lhs)?.lte(&self.eval(rhs)?).map_err(spanitize),
            Punct::More => self.eval(lhs)?.gt(&self.eval(rhs)?).map_err(spanitize),
            Punct::MoreOrEq => self.eval(lhs)?.gte(&self.eval(rhs)?).map_err(spanitize),
            Punct::CmpEqual => self.eval(lhs)?.eq(&self.eval(rhs)?).map_err(spanitize),
            Punct::CmpNotEqual => self.eval(lhs)?.neq(&self.eval(rhs)?).map_err(spanitize),
            Punct::Or => {
                let lhs = self.eval(lhs)?;
                if lhs.truthy() {
                    Ok(lhs)
                } else {
                    self.eval(rhs)
                }
            }
            Punct::And => {
                let lhs = self.eval(lhs)?;
                if lhs.truthy() {
                    self.eval(rhs)
                } else {
                    Ok(lhs)
                }
            }
            Punct::Pipe => {
                let lhs = self.eval(lhs)?;
                // `x | f` and `x | f(a, b)` both mean "call f with x first".
                let (callee, rest) = match &rhs.kind {
                    NodeKind::Call { callee, args } => (&**callee, args),
                    _ => (rhs, &Vec::new()),
                };
                let name = callee_name(&callee.kind).map(String::from);
                let callee = self.eval(callee)?;
                let mut values = Vec::with_capacity(rest.len() + 1);
                values.push(lhs);
                for arg in rest {
                    values.push(self.eval(arg)?);
                }
                call_value(self, callee, Args(values), name.as_deref())
                    .map_err(|e| Spanned::new(e, rhs.span))
            }
            Punct::Coalesce => {
                let lhs = self.eval(lhs)?;
                if lhs.nullish() {
                    self.eval(rhs)
                } else {
                    Ok(lhs)
                }
            }
            _ => unreachable!("There is is no implementation to evaluate '{punct}'"),
        }
    }

    fn eval_atom(&mut self, atom: &Atom) -> Result<Value> {
        match atom {
            // Attached scope shadows the function registry, so a payload field never gets
            // swallowed by a function of the same name.
            Atom::Ident(Ident(i)) => match self.lookup_value(i) {
                // NOTE: This clone is baaad
                Some(v) => Ok(v.clone()),
                None => match self.env.fns.get(i.as_str()) {
                    Some(f) => Ok(Value::Function(f.clone())),
                    None => Ok(Value::Void),
                },
            },
            Atom::StrLit(s) => Ok(Value::String(s.to_owned())),
            Atom::FloatLit(n) => Ok(Value::Number(Number::Float(*n))),
            Atom::IntLit(n) => Ok(Value::Number(Number::Int(*n))),
            Atom::BoolLit(b) => Ok(Value::Bool(*b)),
            Atom::NullLit => Ok(Value::Null),
            Atom::VoidLit => Ok(Value::Void),
        }
    }

    fn eval_object_index(&mut self, Object(object): &Object, index: &Node) -> Result<Value> {
        let index_value = self.eval(index)?;
        match index_value {
            // TODO: this clone can be very expensive
            Value::String(s) => Ok(object.get(&s).cloned().unwrap_or(Value::Void)),
            Value::Null | Value::Void => Ok(Value::Void),
            _ => Err(Spanned::new(
                RuntimeError::InvalidObjectKey(index_value.value_type()),
                index.span,
            )),
        }
    }

    fn eval_array_index(&mut self, Array(array): &Array, index: &Node) -> Result<Value> {
        let index_value = self.eval(index)?;
        match index_value {
            Value::Number(Number::Float(n)) if n >= 0. && n.fract() == 0. => {
                Ok(array.get(n as usize).cloned().unwrap_or(Value::Void))
            }
            Value::Number(Number::Int(n)) if n >= 0 => {
                Ok(array.get(n as usize).cloned().unwrap_or(Value::Void))
            }
            Value::Null | Value::Void => Ok(Value::Void),
            _ => Err(Spanned::new(
                RuntimeError::InvalidArrayIndex(index_value.value_type()),
                index.span,
            )),
        }
    }

    fn eval(&mut self, Node { span, kind }: &Node) -> Result<Value> {
        self.current_depth += 1;
        if self.current_depth >= super::MAX_DEPTH {
            return Err(Spanned::new(RuntimeError::DepthLimitReached, *span));
        }
        let res = match kind {
            NodeKind::Op((punct, (lhs, rhs))) => self.eval_punct_node(lhs, *punct, rhs, span),
            NodeKind::Atom(atom) => self.eval_atom(atom),
            NodeKind::ArrayLit(arr) => Ok(Value::Array(Array(
                arr.iter()
                    .map(|v| self.eval(v))
                    .collect::<Result<Vec<_>>>()?,
            ))),
            NodeKind::ObjectLit(map) => Ok(Value::Object(Object(
                map.iter()
                    .map(|(k, v)| Ok((k.to_owned(), self.eval(v)?)))
                    .collect::<Result<_>>()?,
            ))),
            NodeKind::Turnary {
                operand,
                truth_node,
                false_node,
            } => {
                let operand = self.eval(operand)?;
                if operand.truthy() {
                    self.eval(truth_node)
                } else {
                    self.eval(false_node)
                }
            }
            NodeKind::Index { object, index } => {
                let index_target = self.eval(object)?;
                match index_target {
                    Value::Object(object) => self.eval_object_index(&object, index),
                    Value::Array(array) => self.eval_array_index(&array, index),
                    _ => Err(Spanned::new(RuntimeError::InvalidIndexOperand, index.span)),
                }
            }
            NodeKind::Member {
                object,
                field: Ident(field),
                ..
            } => {
                let object_value = self.eval(object)?;
                match object_value {
                    Value::Object(Object(obj)) => {
                        Ok(obj.get(field).cloned().unwrap_or(Value::Void))
                    }
                    Value::Null | Value::Void => Ok(Value::Void),
                    _ => Err(Spanned::new(RuntimeError::InvalidIndexOperand, object.span)),
                }
            }
            NodeKind::Call { callee, args } => {
                let name = callee_name(&callee.kind).map(String::from);
                let callee = self.eval(callee)?;
                let args = Args(
                    args.iter()
                        .map(|v| self.eval(v))
                        .collect::<Result<Vec<_>>>()?,
                );
                call_value(self, callee, args, name.as_deref()).map_err(|e| Spanned::new(e, *span))
            }
            NodeKind::Negation(statement) => {
                let value = self.eval(statement)?;
                Ok(Value::Bool(!value.truthy()))
            }

            NodeKind::NonNullishAssertion(statement) => {
                let value = self.eval(statement)?;
                if value.nullish() {
                    Err(Spanned::new(RuntimeError::NonNullishAssertion, *span))
                } else {
                    Ok(value)
                }
            }

            NodeKind::Closure { args, body } => {
                let closure = Closure::new(args.clone(), body.clone());
                Ok(Value::Closure(closure))
            }
        };
        self.current_depth -= 1;
        res
    }
}

pub fn eval(node: &Node, scope: &mut RuntimeEnv<'_>) -> Result<Value> {
    scope.eval(node)
}

/// The name a callee was written as, kept only so a failed call can say which one it was.
fn callee_name(node: &NodeKind) -> Option<&str> {
    match node {
        NodeKind::Atom(Atom::Ident(Ident(name))) => Some(name),
        NodeKind::Member {
            field: Ident(f), ..
        } => Some(f),
        _ => None,
    }
}

fn call_value(
    env: &mut RuntimeEnv<'_>,
    callee: Value,
    args: Args,
    name: Option<&str>,
) -> std::result::Result<Value, crate::RuntimeError> {
    match callee {
        Value::Function(f) => f.call(env, args),
        Value::Closure(closure) => closure.call(env, args).map_err(|e| e.inner),
        // An unresolved ident evaluates to void like any other missing lookup, so name it here
        // rather than reporting a bare type mismatch.
        Value::Void => Err(match name {
            Some(name) => FunctionCallError::NotFound {
                name: name.to_owned(),
            },
            None => FunctionCallError::UncallableType(ValueType::Void),
        })?,
        _ => Err(FunctionCallError::UncallableType(callee.value_type()))?,
    }
}
