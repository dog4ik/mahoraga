use crate::{
    Atom::{self},
    Ident, NodeKind, Punct, Span,
    parser::Node,
    value::Number,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactIdent {
    pub name: String,
    pub span: Span,
}

impl FactIdent {
    pub fn new(name: impl Into<String>, span: impl Into<Span>) -> Self {
        Self {
            span: span.into(),
            name: name.into(),
        }
    }
}

impl From<FactIdent> for String {
    fn from(value: FactIdent) -> Self {
        value.name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub name: FactIdent,
    pub args: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathComponent {
    MapIndex(String),
    Object(Box<PathComponent>),
    ArrayIdx(Number),
}

#[derive(Debug, Clone, Default)]
pub struct Facts {
    pub paths: Vec<Vec<FactIdent>>,
    pub calls: Vec<Call>,
}

impl Facts {
    pub fn roots(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for FactIdent { name: root, .. } in self.paths.iter().filter_map(|p| p.iter().next()) {
            if !out.contains(root) {
                out.push(root.into());
            }
        }
        out
    }

    pub fn find_sub_path(&self, needle: &[&str]) -> Option<&Vec<FactIdent>> {
        self.paths.iter().find(|path| {
            path.len() >= needle.len()
                && path
                    .iter()
                    .map(|v| &v.name)
                    .zip(needle)
                    .all(|(a, b)| a == b)
        })
    }
}

#[derive(Debug)]
struct FactsCollector<'a> {
    out: Facts,
    arg_scope: Vec<&'a str>,
}

impl<'a> FactsCollector<'a> {
    fn new() -> Self {
        Self {
            out: Facts::default(),
            arg_scope: Vec::new(),
        }
    }

    fn collect_facts(&mut self, node: &'a Node) {
        if let Some(path) = static_path(node) {
            if !self.arg_scope.contains(&path[0].name.as_str()) {
                self.out.paths.push(path);
                return;
            }
        }
        match &node.kind {
            NodeKind::Op((Punct::Pipe, (input, call))) => {
                self.collect_facts(input);
                match &call.kind {
                    NodeKind::Atom(Atom::Ident(Ident(name))) => {
                        self.collect_call(name, call.span, &[])
                    }
                    NodeKind::Call { callee, args } => self.collect_callee(callee, args),
                    _ => self.collect_facts(call),
                }
            }
            NodeKind::Op((Punct::Coalesce | Punct::Or | Punct::And, (lhs, rhs))) => {
                self.collect_facts(lhs);
                self.collect_facts(rhs);
            }
            NodeKind::Op((_, (lhs, rhs))) => {
                self.collect_facts(lhs);
                self.collect_facts(rhs);
            }
            NodeKind::Turnary {
                operand,
                truth_node,
                false_node,
            } => {
                self.collect_facts(operand);
                self.collect_facts(truth_node);
                self.collect_facts(false_node);
            }
            NodeKind::Member { object, .. }
            | NodeKind::Negation(object)
            | NodeKind::NonNullishAssertion(object) => self.collect_facts(object),
            NodeKind::Index { object, index } => {
                self.collect_facts(object);
                self.collect_facts(index);
            }
            // A string inside a literal is a part of the result, not the result.
            NodeKind::ArrayLit(items) => {
                for item in items {
                    self.collect_facts(item);
                }
            }
            NodeKind::ObjectLit(fields) => {
                for value in fields.values() {
                    self.collect_facts(value);
                }
            }
            NodeKind::Atom(_) => {}
            NodeKind::Call { callee, args } => match args.split_first() {
                Some((input, rest)) => {
                    self.collect_facts(input);
                    self.collect_callee(callee, rest);
                }
                None => self.collect_callee(callee, &[]),
            },
            NodeKind::Closure { args, body } => {
                let old_len = self.arg_scope.len();
                self.arg_scope.extend(args.iter().map(|v| v.0.as_str()));
                self.collect_facts(body);
                self.arg_scope.truncate(old_len);
            }
        }
    }

    fn collect_callee(&mut self, callee: &'a Node, args: &'a [Node]) {
        use {Atom, Ident, NodeKind};
        match &callee.kind {
            NodeKind::Atom(Atom::Ident(Ident(name))) => {
                self.collect_call(name, callee.span, args);
            }
            _ => {
                self.collect_facts(callee);
                for arg in args {
                    self.collect_facts(arg);
                }
            }
        }
    }

    fn collect_call(&mut self, name: &str, span: Span, args: &'a [Node]) {
        if !self.arg_scope.contains(&name) {
            self.out.calls.push(Call {
                name: FactIdent {
                    name: name.to_string(),
                    span,
                },
                args: args.len(),
            });
        }
        for arg in args {
            self.collect_facts(arg);
        }
    }
}

pub fn collect_facts(node: &Node) -> Facts {
    let mut collector = FactsCollector::new();
    collector.collect_facts(node);
    collector.out
}

/// The segments of a chain of identifier, `.field` and `["key"]` accesses.
fn static_path(Node { kind, span }: &Node) -> Option<Vec<FactIdent>> {
    let span = *span;
    match kind {
        NodeKind::Atom(Atom::Ident(Ident(root))) => Some(vec![FactIdent {
            name: root.clone(),
            span,
        }]),
        NodeKind::Member {
            object,
            field: Ident(field),
            field_span,
        } => {
            let mut path = static_path(object)?;
            path.push(FactIdent {
                name: field.clone(),
                span: *field_span,
            });
            Some(path)
        }
        NodeKind::Index { object, index } => match &index.kind {
            NodeKind::Atom(Atom::StrLit(key)) => {
                let mut path = static_path(object)?;
                path.push(FactIdent {
                    name: key.clone(),
                    span: index.span,
                });
                Some(path)
            }
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_expr;

    use super::*;

    fn facts_to_string(facts: Vec<Vec<FactIdent>>) -> Vec<Vec<String>> {
        facts
            .into_iter()
            .map(|v| v.into_iter().map(|v| String::from(v)).collect())
            .collect()
    }

    #[test]
    fn exposes_roots_paths_and_calls() {
        let facts = parse_expr(r#"payment.a["b"] ?? settings.c | trim"#)
            .unwrap()
            .collect_facts();
        assert_eq!(facts.roots(), vec!["payment", "settings"]);
        assert_eq!(
            facts.paths,
            vec![
                vec![
                    FactIdent::new("payment", 0..7),
                    FactIdent::new("a", 8..9),
                    FactIdent::new("b", 10..13)
                ],
                vec![
                    FactIdent::new("settings", 18..26),
                    FactIdent::new("c", 27..28)
                ]
            ]
        );
        assert_eq!(
            facts.calls,
            vec![Call {
                name: FactIdent::new("trim", 31..35),
                args: 0
            }]
        );
    }

    #[test]
    fn a_dynamic_index_reads_both_sides() {
        let facts = parse_expr("steps.auth[params.key].token")
            .unwrap()
            .collect_facts();
        assert_eq!(
            facts_to_string(facts.paths),
            vec![vec!["steps", "auth"], vec!["params", "key"]]
        );
    }

    #[test]
    fn literals_expose_the_paths_they_contain() {
        let facts = parse_expr(r#"[params.first_name, "x", {"phone": params.customer.phone}]"#)
            .unwrap()
            .collect_facts();
        assert_eq!(
            facts_to_string(facts.paths),
            vec![
                vec!["params", "first_name"],
                vec!["params", "customer", "phone"]
            ]
        );
    }

    #[test]
    fn closure_arguments_are_not_roots() {
        let facts = parse_expr(r#"[params.first_name, top_level, nested, |x, name| [x + name, |nested_arg| nested_arg, |nested| nested], nested_arg]"#)
            .unwrap()
            .collect_facts();
        assert_eq!(
            facts.roots(),
            vec!["params", "top_level", "nested", "nested_arg"]
        );
    }

    #[test]
    fn closure_argument_not_counted() {
        let facts = parse_expr(r#"|v| v(foo, bar)"#).unwrap().collect_facts();
        assert_eq!(facts.roots(), vec!["foo", "bar"]);

        assert!(
            facts
                .calls
                .iter()
                .find(|Call { name, .. }| name.name == "v")
                .is_none()
        )
    }

    #[test]
    fn nested_closure_does_not_cleanup_capture() {
        let facts = parse_expr(r#"|x| [|x| x, x.y] "#).unwrap().collect_facts();
        assert!(facts.roots().is_empty());
    }
}
