use std::collections::{BTreeMap, BTreeSet};
use std::ops::ControlFlow;

use sqlparser::ast::{BinaryOperator, Expr, Query as SqlQuery, Select, Statement, Visit, Visitor};

use super::expressions::{ColumnRef, is_placeholder, qualified_column_ref};
use super::tables::{TableSources, direct_select_query, select_table_sources_with_cte_names};

pub(super) type PendingParamColumns = BTreeMap<usize, ColumnRef>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct ParamContext {
    column: Option<ColumnRef>,
    sources: Vec<TableSources>,
}

impl ParamContext {
    pub(super) fn resolve_column(
        &self,
        schema: &super::super::schema::SqliteSchema,
    ) -> Option<super::super::schema::SqliteSchemaColumn> {
        let column = self.column.as_ref()?;
        for sources in &self.sources {
            if sources.qualifier_is_known(&column.qualifier) {
                return sources.resolve_column(schema, column);
            }
        }
        None
    }

    pub(super) const fn column(&self) -> Option<&ColumnRef> {
        self.column.as_ref()
    }

    pub(super) fn qualifier_is_known(&self) -> bool {
        let Some(column) = self.column.as_ref() else {
            return false;
        };
        self.sources
            .iter()
            .any(|sources| sources.qualifier_is_known(&column.qualifier))
    }
}

pub(super) fn collect_query_param_contexts(
    query: &SqlQuery,
    expected_count: usize,
) -> Vec<ParamContext> {
    collect_param_contexts(query, None, PendingParamColumns::new(), expected_count)
}

pub(super) fn collect_mutation_param_contexts(
    statement: &Statement,
    mutation_sources: TableSources,
    pending_columns: PendingParamColumns,
    expected_count: usize,
) -> Vec<ParamContext> {
    collect_param_contexts(
        statement,
        Some(mutation_sources),
        pending_columns,
        expected_count,
    )
}

pub(super) fn record_param_column(
    pending_columns: &mut PendingParamColumns,
    expression: &Expr,
    column: ColumnRef,
) {
    pending_columns.insert(expr_key(expression), column);
}

fn collect_param_contexts(
    node: &impl Visit,
    fallback_sources: Option<TableSources>,
    pending_columns: PendingParamColumns,
    expected_count: usize,
) -> Vec<ParamContext> {
    let mut collector = ParamContextCollector {
        fallback_sources,
        pending_columns,
        ..ParamContextCollector::default()
    };
    let _ = node.visit(&mut collector);
    if collector.contexts.len() == expected_count {
        collector.contexts
    } else {
        vec![ParamContext::default(); expected_count]
    }
}

#[derive(Default)]
struct ParamContextCollector {
    contexts: Vec<ParamContext>,
    fallback_sources: Option<TableSources>,
    pending_columns: PendingParamColumns,
    query_cte_names: Vec<BTreeSet<String>>,
    query_sources: Vec<Option<TableSources>>,
    select_sources: Vec<TableSources>,
}

impl Visitor for ParamContextCollector {
    type Break = ();

    fn pre_visit_query(&mut self, query: &SqlQuery) -> ControlFlow<Self::Break> {
        let mut cte_names = self.query_cte_names.last().cloned().unwrap_or_default();
        if let Some(with) = query.with.as_ref() {
            cte_names.extend(
                with.cte_tables
                    .iter()
                    .map(|cte| cte.alias.name.value.to_ascii_lowercase()),
            );
        }
        let query_sources = direct_select_query(query)
            .map(|(_direct_query, select)| select_table_sources_with_cte_names(select, &cte_names));
        self.query_cte_names.push(cte_names);
        self.query_sources.push(query_sources);
        ControlFlow::Continue(())
    }

    fn post_visit_query(&mut self, _query: &SqlQuery) -> ControlFlow<Self::Break> {
        self.query_sources.pop();
        self.query_cte_names.pop();
        ControlFlow::Continue(())
    }

    fn pre_visit_select(&mut self, select: &Select) -> ControlFlow<Self::Break> {
        let cte_names = self.query_cte_names.last().cloned().unwrap_or_default();
        self.select_sources
            .push(select_table_sources_with_cte_names(select, &cte_names));
        ControlFlow::Continue(())
    }

    fn post_visit_select(&mut self, _select: &Select) -> ControlFlow<Self::Break> {
        self.select_sources.pop();
        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &Expr) -> ControlFlow<Self::Break> {
        record_direct_param_columns(expr, &mut self.pending_columns);
        if is_placeholder(expr) {
            let mut sources = Vec::new();
            if let Some(query_sources) = self.query_sources.last().and_then(Option::as_ref)
                && self.select_sources.last() != Some(query_sources)
            {
                sources.push(query_sources.clone());
            }
            for select_sources in self.select_sources.iter().rev() {
                if !sources.contains(select_sources) {
                    sources.push(select_sources.clone());
                }
            }
            if let Some(fallback_sources) = &self.fallback_sources
                && !sources.contains(fallback_sources)
            {
                sources.push(fallback_sources.clone());
            }
            self.contexts.push(ParamContext {
                column: self.pending_columns.remove(&expr_key(expr)),
                sources,
            });
        }
        ControlFlow::Continue(())
    }
}

fn record_direct_param_columns(expr: &Expr, pending: &mut PendingParamColumns) {
    match expr {
        Expr::BinaryOp { left, op, right } if is_supported_comparison_operator(op) => {
            if let Some(column) = qualified_column_ref(left)
                && is_placeholder(right)
            {
                record_param_column(pending, right, column);
            } else if is_placeholder(left)
                && let Some(column) = qualified_column_ref(right)
            {
                record_param_column(pending, left, column);
            }
        }
        Expr::InList {
            expr,
            list,
            negated: false,
        } => {
            if let Some(column) = qualified_column_ref(expr) {
                for item in list {
                    if is_placeholder(item) {
                        record_param_column(pending, item, column.clone());
                    }
                }
            }
        }
        _ => {}
    }
}

const fn is_supported_comparison_operator(operator: &BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Eq
            | BinaryOperator::NotEq
            | BinaryOperator::Lt
            | BinaryOperator::LtEq
            | BinaryOperator::Gt
            | BinaryOperator::GtEq
    )
}

fn expr_key(expr: &Expr) -> usize {
    std::ptr::from_ref(expr) as usize
}
