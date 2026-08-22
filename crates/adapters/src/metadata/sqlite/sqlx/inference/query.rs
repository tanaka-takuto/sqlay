use sqlay_core as core;
use sqlparser::ast::{Expr, SelectItem, Statement};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;

use super::super::diagnostics::{param_usage_error, query_error};
use super::super::schema::{SqliteSchema, SqliteSchemaColumn};
use super::expressions::qualified_column_ref;
use super::param_contexts::{ParamContext, collect_query_param_contexts};
use super::schema_qualifiers::unsupported_schema_qualifier;
use super::tables::{TableSources, direct_select_query, select_table_sources};

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
    if let Some(qualifier) = unsupported_schema_qualifier(sql_query) {
        return Err(unsupported_schema_qualifier_error(query, &qualifier));
    }

    let direct_select = direct_select_query(sql_query);
    let result_columns = direct_select.map_or_else(Vec::new, |(direct_query, select)| {
        let sources = select_table_sources(direct_query, select);
        select
            .projection
            .iter()
            .map(|item| resolve_projection(item, &sources, schema))
            .collect()
    });
    let requires_prepare_only = direct_select.is_none()
        || result_columns.iter().any(|column| {
            column
                .as_ref()
                .is_none_or(|column| column.ty == core::CoreType::Unknown)
        });
    let contexts = collect_query_param_contexts(sql_query, query.param_usages().len());
    let param_usages = resolve_query_params(query, contexts, schema)?;

    Ok(QueryInference {
        result_columns,
        param_usages,
        requires_prepare_only,
    })
}

fn unsupported_schema_qualifier_error(
    query: &core::RawQuery,
    qualifier: &str,
) -> core::DiagnosticReport {
    query_error(
        query,
        format!(
            "unsupported SQLite schema qualifier `{qualifier}`; only the main schema is supported, using `table` or `main.table` references"
        ),
    )
}

fn resolve_projection(
    item: &SelectItem,
    sources: &TableSources,
    schema: &SqliteSchema,
) -> Option<SqliteSchemaColumn> {
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

fn resolve_query_params(
    query: &core::RawQuery,
    contexts: Vec<ParamContext>,
    schema: &SqliteSchema,
) -> core::DiagnosticResult<Vec<core::DbParamUsage>> {
    query
        .param_usages()
        .iter()
        .zip(contexts)
        .map(|(usage, context)| {
            let schema_column = context.resolve_column(schema);
            let ty = if let Some(ty) = usage.value_type_override() {
                ty
            } else if let Some(column) = schema_column.as_ref()
                && column.ty != core::CoreType::Unknown
            {
                column.ty
            } else if let Some(column) = schema_column.as_ref() {
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
                    unresolved_param_message(usage.id(), &context),
                ));
            };
            let mut param = core::DbParamUsage::new(usage.id().to_owned(), ty);
            if ty == core::CoreType::Bool {
                param = param.with_encoding(core::ParamEncoding::BooleanAsInteger);
            }
            if let Some(column) = schema_column.as_ref() {
                param = param.with_schema_column_reference(column.reference());
            }
            Ok(param)
        })
        .collect()
}

fn unresolved_param_message(id: &str, context: &ParamContext) -> String {
    match context.column() {
        Some(column) if context.qualifier_is_known() => format!(
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
