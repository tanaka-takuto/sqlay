use std::ops::ControlFlow;

use sqlparser::ast::{ObjectName, Visit, Visitor};

use super::tables::object_name_parts;

pub(super) fn unsupported_schema_qualifier(node: &impl Visit) -> Option<String> {
    let mut visitor = SchemaQualifierVisitor;
    match node.visit(&mut visitor) {
        ControlFlow::Break(qualifier) => Some(qualifier),
        ControlFlow::Continue(()) => None,
    }
}

struct SchemaQualifierVisitor;

impl Visitor for SchemaQualifierVisitor {
    type Break = String;

    fn pre_visit_relation(&mut self, relation: &ObjectName) -> ControlFlow<Self::Break> {
        let parts = object_name_parts(relation);
        match parts.as_slice() {
            [_table] => ControlFlow::Continue(()),
            [schema, _table] if schema.eq_ignore_ascii_case("main") => ControlFlow::Continue(()),
            [schema, _table] => ControlFlow::Break(schema.clone()),
            [qualifier @ .., _table] if qualifier.len() > 1 => {
                ControlFlow::Break(qualifier.join("."))
            }
            _ => ControlFlow::Continue(()),
        }
    }
}
