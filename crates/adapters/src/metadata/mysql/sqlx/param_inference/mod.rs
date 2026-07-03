mod contexts;
mod mutations;
mod result_columns;
mod schema_type_ref;
mod tables;
mod unsupported_contexts;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use sqlay_core as core;
use sqlparser::ast::{Query as SqlQuery, Statement};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;

use super::diagnostics::{param_usage_error, query_error};
use super::schema_columns::{MysqlSchemaColumn, MysqlSchemaTableRef};
use contexts::{ColumnRef, collect_query_param_contexts};
pub(super) use mutations::{mutation_schema_table_refs, resolve_mutation_param_usage_metadata};
pub(super) use result_columns::resolve_result_column_type_refs;
pub(in crate::metadata::mysql::sqlx) use schema_type_ref::ResolvedSchemaTypeRef;
use tables::{
    QuerySchemaTableRefResolution, SelectTableSources, resolve_query_schema_table_ref_status,
    select_from_query, select_table_sources,
};

const SUPPORTED_PARAM_VALUE_TYPES_MESSAGE: &str = "`bool`, `int32`, `int64`, `float64`, `decimal`, `string`, `bytes`, `date`, `time`, `datetime`, and `json`";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SchemaColumnTypes {
    columns: BTreeMap<(MysqlSchemaTableRef, String), core::CoreTypeRef>,
    tables: BTreeSet<MysqlSchemaTableRef>,
}

impl SchemaColumnTypes {
    fn from_columns(columns: &[MysqlSchemaColumn]) -> Self {
        let mut schema = Self::default();
        for column in columns {
            schema.tables.insert(column.table_ref.clone());
            schema.columns.insert(
                (column.table_ref.clone(), column.column_name.clone()),
                column.type_ref.clone(),
            );
        }

        schema
    }

    fn get(&self, table_ref: &MysqlSchemaTableRef, column_name: &str) -> Option<core::CoreTypeRef> {
        self.columns
            .get(&(table_ref.clone(), column_name.to_owned()))
            .cloned()
    }

    fn has_table(&self, table_ref: &MysqlSchemaTableRef) -> bool {
        self.tables.contains(table_ref)
    }
}

pub(super) fn resolve_param_usage_metadata(
    query: &core::RawQuery,
    schema_columns: &[MysqlSchemaColumn],
) -> core::DiagnosticResult<Vec<core::DbParamUsage>> {
    if query.param_usages().is_empty() {
        return Ok(Vec::new());
    }

    let statements = parse_query(query)?;
    let parsed_query = single_select_query(query, &statements)?;
    let select = select_from_query(parsed_query)
        .expect("single_select_query verifies this is a top-level SELECT query");
    let mut contexts = collect_query_param_contexts(parsed_query, select);
    if contexts.len() > query.param_usages().len() {
        return Err(query_error(
            query,
            format!(
                "resolved Param context count {} does not match source Param usage count {}",
                contexts.len(),
                query.param_usages().len()
            ),
        ));
    }
    contexts.resize(query.param_usages().len(), None);

    let table_sources = select_table_sources(parsed_query, select);
    let schema = SchemaColumnTypes::from_columns(schema_columns);
    let mut params = Vec::with_capacity(query.param_usages().len());

    for (usage, context) in query.param_usages().iter().zip(contexts) {
        let resolved = if let Some(value_type) = usage.value_type_override() {
            let schema_column_reference = context.as_ref().and_then(|column| {
                resolve_param_schema_type_ref(column, &table_sources, &schema)?
                    .schema_column_reference
            });
            ResolvedSchemaTypeRef::new(core::CoreTypeRef::from(value_type), schema_column_reference)
        } else {
            resolve_inferred_param_type(query, usage, context.as_ref(), &table_sources, &schema)?
        };
        let mut param = core::DbParamUsage::new_type_ref(usage.id().to_owned(), resolved.type_ref);
        if let Some(reference) = resolved.schema_column_reference {
            param = param.with_schema_column_reference(reference);
        }
        params.push(param);
    }

    Ok(params)
}

fn resolve_param_schema_type_ref(
    column: &ColumnRef,
    table_sources: &SelectTableSources,
    schema: &SchemaColumnTypes,
) -> Option<ResolvedSchemaTypeRef> {
    let QuerySchemaTableRefResolution::Resolved(table_ref) =
        resolve_query_schema_table_ref_status(table_sources, schema, &column.qualifier)
    else {
        return None;
    };
    let type_ref = schema.get(&table_ref, &column.column)?;
    Some(ResolvedSchemaTypeRef::schema_column(
        type_ref,
        &table_ref,
        &column.column,
    ))
}

fn resolve_inferred_param_type(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    context: Option<&ColumnRef>,
    table_sources: &SelectTableSources,
    schema: &SchemaColumnTypes,
) -> core::DiagnosticResult<ResolvedSchemaTypeRef> {
    let Some(column) = context else {
        return Err(param_usage_error(
            query,
            usage,
            param_value_type_required_message(
                usage.id(),
                "no supported qualified column context was found",
            ),
        ));
    };

    if let Some(resolved) = resolve_param_schema_type_ref(column, table_sources, schema) {
        return Ok(resolved);
    }

    let table_ref =
        match resolve_query_schema_table_ref_status(table_sources, schema, &column.qualifier) {
            QuerySchemaTableRefResolution::Resolved(table_ref) => table_ref,
            QuerySchemaTableRefResolution::Unsupported => {
                return Err(param_usage_error(
                    query,
                    usage,
                    param_value_type_required_message(
                        usage.id(),
                        format!(
                            "table alias `{}` does not resolve to a supported schema-backed table",
                            column.qualifier
                        ),
                    ),
                ));
            }
            QuerySchemaTableRefResolution::Unknown => {
                return Err(param_usage_error(
                    query,
                    usage,
                    format!(
                        "Param `{}` references unknown table alias `{}`",
                        usage.id(),
                        column.qualifier
                    ),
                ));
            }
        };

    if !schema.has_table(&table_ref) {
        return Err(param_usage_error(
            query,
            usage,
            format!(
                "Param `{}` references unknown {}",
                usage.id(),
                table_ref.table_description()
            ),
        ));
    }

    Err(param_usage_error(
        query,
        usage,
        table_ref.unknown_column_message(usage.id(), &column.column),
    ))
}

fn param_value_type_required_message(id: &str, reason: impl AsRef<str>) -> String {
    let reason = reason.as_ref();
    format!(
        "Param `{id}` requires `valueType` because {reason}; use an inline `valueType` such as `valueType: string` or compare the Param directly with a qualified column; supported values are {SUPPORTED_PARAM_VALUE_TYPES_MESSAGE}; use `nullable: true` for nullable inputs"
    )
}

pub(super) fn schema_table_refs(
    query: &core::RawQuery,
) -> core::DiagnosticResult<Vec<MysqlSchemaTableRef>> {
    let statements = parse_query(query)?;
    let parsed_query = single_select_query(query, &statements)?;
    let select = select_from_query(parsed_query)
        .expect("single_select_query verifies this is a top-level SELECT query");
    Ok(select_table_sources(parsed_query, select)
        .schema_table_refs
        .into_iter()
        .collect())
}

fn parse_query(query: &core::RawQuery) -> core::DiagnosticResult<Vec<Statement>> {
    let dialect = MySqlDialect {};
    Parser::parse_sql(&dialect, query.analysis_sql())
        .map_err(|error| query_error(query, format!("failed to parse MySQL SQL: {error}")))
}

fn single_select_query<'a>(
    query: &core::RawQuery,
    statements: &'a [Statement],
) -> core::DiagnosticResult<&'a SqlQuery> {
    let [Statement::Query(parsed_query)] = statements else {
        return Err(query_error(
            query,
            "Param type inference requires exactly one SELECT query",
        ));
    };

    if select_from_query(parsed_query).is_none() {
        return Err(query_error(
            query,
            "Param type inference requires a top-level SELECT query",
        ));
    }

    Ok(parsed_query)
}
