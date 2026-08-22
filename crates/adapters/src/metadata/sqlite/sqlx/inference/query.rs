use sqlay_core as core;
use sqlparser::ast::{
    Expr, JoinConstraint, JoinOperator, Select, SelectItem, Statement, TableWithJoins,
};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;

use super::super::diagnostics::{param_usage_error, query_error};
use super::super::schema::{SqliteSchema, SqliteSchemaColumn};
use super::expressions::{ColumnRef, collect_expr_param_contexts, qualified_column_ref};
use super::tables::{TableSources, select_from_query, select_table_sources};

pub(in crate::metadata::sqlite::sqlx) struct QueryInference {
    pub(in crate::metadata::sqlite::sqlx) result_columns: Vec<Option<SqliteSchemaColumn>>,
    pub(in crate::metadata::sqlite::sqlx) param_usages: Vec<core::DbParamUsage>,
    pub(in crate::metadata::sqlite::sqlx) requires_prepare_only: bool,
}

pub(in crate::metadata::sqlite::sqlx) fn infer_query(
    query: &core::RawQuery,
    schema: &SqliteSchema,
) -> core::DiagnosticResult<QueryInference> {
    let dialect = SQLiteDialect {};
    let statements = Parser::parse_sql(&dialect, query.analysis_sql())
        .map_err(|error| query_error(query, format!("failed to parse SQLite SQL: {error}")))?;
    let [Statement::Query(sql_query)] = statements.as_slice() else {
        return Err(query_error(
            query,
            "SQLite metadata inference requires exactly one query statement",
        ));
    };
    let Some(select) = select_from_query(sql_query) else {
        return Err(query_error(
            query,
            "SQLite metadata inference requires one direct SELECT body",
        ));
    };
    let sources = select_table_sources(sql_query, select);
    reject_unsupported_schema_qualifier(query, &sources)?;
    let result_columns: Vec<Option<SqliteSchemaColumn>> = select
        .projection
        .iter()
        .map(|item| resolve_projection(item, &sources, schema).cloned())
        .collect();
    let requires_prepare_only = result_columns.iter().any(|column| {
        column
            .as_ref()
            .is_none_or(|column| column.ty == core::CoreType::Unknown)
    });
    let contexts = collect_query_param_contexts(select, query.param_usages().len());
    let param_usages = resolve_query_params(query, contexts, &sources, schema)?;

    Ok(QueryInference {
        result_columns,
        param_usages,
        requires_prepare_only,
    })
}

fn reject_unsupported_schema_qualifier(
    query: &core::RawQuery,
    sources: &TableSources,
) -> core::DiagnosticResult<()> {
    let Some(qualifier) = sources.unsupported_schema_qualifier() else {
        return Ok(());
    };

    Err(query_error(
        query,
        format!(
            "unsupported SQLite schema qualifier `{qualifier}`; only the main schema is supported, using `table` or `main.table` references"
        ),
    ))
}

fn resolve_projection<'a>(
    item: &SelectItem,
    sources: &TableSources,
    schema: &'a SqliteSchema,
) -> Option<&'a SqliteSchemaColumn> {
    let expr = match item {
        SelectItem::UnnamedExpr(expr)
        | SelectItem::ExprWithAlias { expr, .. }
        | SelectItem::ExprWithAliases { expr, .. } => expr,
        SelectItem::QualifiedWildcard(_, _) | SelectItem::Wildcard(_) => return None,
    };

    if let Some(column) = qualified_column_ref(expr) {
        return sources.resolve_column(schema, &column);
    }
    let Expr::Identifier(identifier) = expr else {
        return None;
    };
    sources.resolve_unqualified_column(schema, &identifier.value)
}

fn collect_query_param_contexts(select: &Select, expected_count: usize) -> Vec<Option<ColumnRef>> {
    let mut contexts = Vec::new();
    for item in &select.projection {
        if let SelectItem::UnnamedExpr(expr)
        | SelectItem::ExprWithAlias { expr, .. }
        | SelectItem::ExprWithAliases { expr, .. } = item
        {
            collect_expr_param_contexts(expr, &mut contexts);
        }
    }
    for table in &select.from {
        collect_join_param_contexts(table, &mut contexts);
    }
    if let Some(selection) = &select.selection {
        collect_expr_param_contexts(selection, &mut contexts);
    }
    if let Some(having) = &select.having {
        collect_expr_param_contexts(having, &mut contexts);
    }

    if contexts.len() == expected_count {
        contexts
    } else {
        vec![None; expected_count]
    }
}

fn collect_join_param_contexts(table: &TableWithJoins, contexts: &mut Vec<Option<ColumnRef>>) {
    for join in &table.joins {
        let constraint = match &join.join_operator {
            JoinOperator::Join(constraint)
            | JoinOperator::Inner(constraint)
            | JoinOperator::Left(constraint)
            | JoinOperator::LeftOuter(constraint)
            | JoinOperator::Right(constraint)
            | JoinOperator::RightOuter(constraint)
            | JoinOperator::FullOuter(constraint)
            | JoinOperator::CrossJoin(constraint)
            | JoinOperator::Semi(constraint)
            | JoinOperator::LeftSemi(constraint)
            | JoinOperator::RightSemi(constraint)
            | JoinOperator::Anti(constraint)
            | JoinOperator::LeftAnti(constraint)
            | JoinOperator::RightAnti(constraint)
            | JoinOperator::StraightJoin(constraint)
            | JoinOperator::AsOf { constraint, .. } => Some(constraint),
            _ => None,
        };
        if let Some(JoinConstraint::On(expr)) = constraint {
            collect_expr_param_contexts(expr, contexts);
        }
    }
}

fn resolve_query_params(
    query: &core::RawQuery,
    contexts: Vec<Option<ColumnRef>>,
    sources: &TableSources,
    schema: &SqliteSchema,
) -> core::DiagnosticResult<Vec<core::DbParamUsage>> {
    query
        .param_usages()
        .iter()
        .zip(contexts)
        .map(|(usage, context)| {
            let schema_column = context
                .as_ref()
                .and_then(|column| sources.resolve_column(schema, column));
            let ty = if let Some(ty) = usage.value_type_override() {
                ty
            } else if let Some(column) = schema_column
                && column.ty != core::CoreType::Unknown
            {
                column.ty
            } else if let Some(column) = schema_column {
                return Err(param_usage_error(
                    query,
                    usage,
                    format!(
                        "Param `{}` references main-schema column `{}.{}` with an ambiguous SQLite declared type; add `valueType` to override inference",
                        usage.id(), column.table_name, column.column_name
                    ),
                ));
            } else {
                return Err(param_usage_error(
                    query,
                    usage,
                    unresolved_param_message(usage.id(), context.as_ref(), sources),
                ));
            };
            let mut param = core::DbParamUsage::new(usage.id().to_owned(), ty);
            if let Some(column) = schema_column {
                param = param.with_schema_column_reference(column.reference());
            }
            Ok(param)
        })
        .collect()
}

fn unresolved_param_message(
    id: &str,
    context: Option<&ColumnRef>,
    sources: &TableSources,
) -> String {
    match context {
        Some(column) if sources.qualifier_is_known(&column.qualifier) => format!(
            "Param `{id}` references unknown main-schema column `{}.{}`; add `valueType` to override inference",
            column.qualifier, column.column
        ),
        Some(column) => format!(
            "Param `{id}` qualifier `{}` is not a supported main-schema table; add `valueType` to override inference",
            column.qualifier
        ),
        None => format!(
            "Param `{id}` has no supported qualified SQLite column context; add `valueType` to override inference"
        ),
    }
}
