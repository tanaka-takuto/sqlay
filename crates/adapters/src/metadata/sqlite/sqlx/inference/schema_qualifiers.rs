use std::collections::BTreeSet;
use std::ops::ControlFlow;

use sqlparser::ast::{ObjectName, Query as SqlQuery, Visit, Visitor};

use super::tables::object_name_parts;

pub(super) fn unsupported_schema_qualifier(node: &impl Visit) -> Option<String> {
    let mut visitor = SchemaQualifierVisitor::default();
    match node.visit(&mut visitor) {
        ControlFlow::Break(qualifier) => Some(qualifier),
        ControlFlow::Continue(()) => None,
    }
}

#[derive(Default)]
struct SchemaQualifierVisitor {
    visible_cte_names: Vec<BTreeSet<String>>,
}

impl Visitor for SchemaQualifierVisitor {
    type Break = String;

    fn pre_visit_query(&mut self, query: &SqlQuery) -> ControlFlow<Self::Break> {
        let mut visible_cte_names = self.visible_cte_names.last().cloned().unwrap_or_default();
        if let Some(with) = query.with.as_ref() {
            visible_cte_names.extend(
                with.cte_tables
                    .iter()
                    .map(|cte| cte.alias.name.value.to_ascii_lowercase()),
            );
        }
        self.visible_cte_names.push(visible_cte_names);
        ControlFlow::Continue(())
    }

    fn post_visit_query(&mut self, _query: &SqlQuery) -> ControlFlow<Self::Break> {
        self.visible_cte_names.pop();
        ControlFlow::Continue(())
    }

    fn pre_visit_relation(&mut self, relation: &ObjectName) -> ControlFlow<Self::Break> {
        let parts = object_name_parts(relation);
        match parts.as_slice() {
            [table]
                if is_temp_schema_alias(table)
                    && !self
                        .visible_cte_names
                        .iter()
                        .rev()
                        .any(|names| names.contains(&table.to_ascii_lowercase())) =>
            {
                ControlFlow::Break("temp".to_owned())
            }
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

const fn is_temp_schema_alias(table: &str) -> bool {
    table.eq_ignore_ascii_case("sqlite_temp_master")
        || table.eq_ignore_ascii_case("sqlite_temp_schema")
}
