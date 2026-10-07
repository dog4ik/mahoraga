use std::collections::HashMap;

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct Paths(HashMap<String, Paths>);

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

pub fn collect_facts(node: &Node, out: &mut Facts) {
    if let Some(path) = static_path(node) {
        out.paths.push(path);
        return;
    }
    match &node.kind {
        NodeKind::Op((Punct::Pipe, (input, call))) => {
            collect_facts(input, out);
            match &call.kind {
                NodeKind::Atom(Atom::Ident(Ident(name))) => collect_call(name, call.span, &[], out),
                NodeKind::Call { callee, args } => collect_callee(callee, args, out),
                _ => collect_facts(call, out),
            }
        }
        NodeKind::Op((Punct::Coalesce | Punct::Or | Punct::And, (lhs, rhs))) => {
            collect_facts(lhs, out);
            collect_facts(rhs, out);
        }
        NodeKind::Op((_, (lhs, rhs))) => {
            collect_facts(lhs, out);
            collect_facts(rhs, out);
        }
        NodeKind::Turnary {
            operand,
            truth_node,
            false_node,
        } => {
            collect_facts(operand, out);
            collect_facts(truth_node, out);
            collect_facts(false_node, out);
        }
        NodeKind::Member { object, .. } | NodeKind::Negation(object) => collect_facts(object, out),
        NodeKind::Index { object, index } => {
            collect_facts(object, out);
            collect_facts(index, out);
        }
        // A string inside a literal is a part of the result, not the result.
        NodeKind::ArrayLit(items) => {
            for item in items {
                collect_facts(item, out);
            }
        }
        NodeKind::ObjectLit(fields) => {
            for value in fields.values() {
                collect_facts(value, out);
            }
        }
        NodeKind::Atom(_) => {}
        NodeKind::Call { callee, args } => match args.split_first() {
            Some((input, rest)) => {
                collect_facts(input, out);
                collect_callee(callee, rest, out);
            }
            None => collect_callee(callee, &[], out),
        },
    }
}

fn collect_callee(callee: &Node, args: &[Node], out: &mut Facts) {
    use {Atom, Ident, NodeKind};
    match &callee.kind {
        NodeKind::Atom(Atom::Ident(Ident(name))) => collect_call(name, callee.span, args, out),
        _ => {
            collect_facts(callee, out);
            for arg in args {
                collect_facts(arg, out);
            }
        }
    }
}

/// `is_result` carries through the arguments a builtin hands back untouched,
/// so `| map({"a": "approved"})` still exposes a status literal.
fn collect_call(name: &str, span: Span, args: &[Node], out: &mut Facts) {
    use NodeKind;
    out.calls.push(Call {
        name: FactIdent {
            name: name.to_string(),
            span,
        },
        args: args.len(),
    });
    for arg in args {
        match &arg.kind {
            NodeKind::ObjectLit(fields) => {
                for value in fields.values() {
                    collect_facts(value, out);
                }
            }
            _ => collect_facts(arg, out),
        }
    }
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
        let e = parse_expr(r#"payment.a["b"] ?? settings.c | trim"#)
            .unwrap()
            .collect_facts();
        assert_eq!(e.roots(), vec!["payment", "settings"]);
        assert_eq!(
            e.paths,
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
            e.calls,
            vec![Call {
                name: FactIdent::new("trim", 31..35),
                args: 0
            }]
        );
    }

    #[test]
    fn a_dynamic_index_reads_both_sides() {
        let e = parse_expr("steps.auth[params.key].token")
            .unwrap()
            .collect_facts();
        assert_eq!(
            facts_to_string(e.paths),
            vec![vec!["steps", "auth"], vec!["params", "key"]]
        );
    }

    #[test]
    fn literals_expose_the_paths_they_contain() {
        let e = parse_expr(r#"[params.first_name, "x", {"phone": params.customer.phone}]"#)
            .unwrap()
            .collect_facts();
        assert_eq!(
            facts_to_string(e.paths),
            vec![
                vec!["params", "first_name"],
                vec!["params", "customer", "phone"]
            ]
        );
    }
}
