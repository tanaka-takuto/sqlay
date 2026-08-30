use sqlay_core as core;
use sqlparser::ast::{
    Assignment, AssignmentTarget, Delete, FromTable, Insert, ObjectName, SetExpr, Statement,
    TableObject, Update,
};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;

use super::super::diagnostics::{mutation_error, mutation_param_usage_error};
use super::super::schema::SqliteSchema;
use super::expressions::{ColumnRef, is_placeholder};
use super::param_contexts::{
    ParamContext, PendingParamColumns, collect_mutation_param_contexts, record_param_column,
};
use super::schema_qualifiers::unsupported_schema_qualifier;
use super::tables::{
    TableSources, named_table_sources, object_name_parts, single_table_sources,
    table_with_joins_default_qualifier,
};

pub(in crate::metadata::sqlite::sqlx) fn infer_mutation_params(
    mutation: &core::RawMutation,
    schema: &SqliteSchema,
) -> core::DiagnosticResult<Vec<core::DbParamUsage>> {
    let dialect = SQLiteDialect {};
    let statements = Parser::parse_sql(&dialect, mutation.analysis_sql()).map_err(|error| {
        mutation_error(mutation, format!("failed to parse SQLite SQL: {error}"))
    })?;
    let [statement] = statements.as_slice() else {
        return Err(mutation_error(
            mutation,
            "SQLite metadata inference requires exactly one mutation statement",
        ));
    };
    if let Some(qualifier) = unsupported_schema_qualifier(statement) {
        return Err(unsupported_schema_qualifier_error(mutation, &qualifier));
    }
    let (sources, pending_columns) = mutation_contexts(statement);
    let contexts = collect_mutation_param_contexts(
        statement,
        sources,
        pending_columns,
        mutation.param_usages().len(),
    );

    resolve_mutation_params(mutation, contexts, schema)
}

fn unsupported_schema_qualifier_error(
    mutation: &core::RawMutation,
    qualifier: &str,
) -> core::DiagnosticReport {
    mutation_error(
        mutation,
        format!(
            "unsupported SQLite schema qualifier `{qualifier}`; only the main schema is supported, using `table` or `main.table` references"
        ),
    )
}

fn mutation_contexts(statement: &Statement) -> (TableSources, PendingParamColumns) {
    match statement {
        Statement::Insert(insert) => insert_contexts(insert),
        Statement::Update(update) => update_contexts(update),
        Statement::Delete(delete) => delete_contexts(delete),
        _ => (TableSources::default(), PendingParamColumns::new()),
    }
}

fn insert_contexts(insert: &Insert) -> (TableSources, PendingParamColumns) {
    let TableObject::TableName(table_name) = &insert.table else {
        return (TableSources::default(), PendingParamColumns::new());
    };
    let alias = insert
        .table_alias
        .as_ref()
        .map(|alias| alias.alias.value.as_str());
    let sources = named_table_sources(table_name, alias);
    let qualifier = insert
        .table_alias
        .as_ref()
        .map(|alias| alias.alias.value.clone())
        .or_else(|| object_name_parts(table_name).last().cloned());
    let mut pending_columns = PendingParamColumns::new();

    if let Some(source) = &insert.source
        && let SetExpr::Values(values) = source.body.as_ref()
    {
        for row in &values.rows {
            for (index, expr) in row.iter().enumerate() {
                if is_placeholder(expr)
                    && let Some(column) =
                        insert_column_context(insert.columns.get(index), qualifier.as_deref())
                {
                    record_param_column(&mut pending_columns, expr, column);
                }
            }
        }
    }

    (sources, pending_columns)
}

fn update_contexts(update: &Update) -> (TableSources, PendingParamColumns) {
    let sources = single_table_sources(&update.table);
    let qualifier = table_with_joins_default_qualifier(&update.table);
    let mut pending_columns = PendingParamColumns::new();
    for assignment in &update.assignments {
        record_assignment_context(assignment, qualifier.as_deref(), &mut pending_columns);
    }
    (sources, pending_columns)
}

fn delete_contexts(delete: &Delete) -> (TableSources, PendingParamColumns) {
    let table = match &delete.from {
        FromTable::WithFromKeyword(tables) | FromTable::WithoutKeyword(tables) => tables.first(),
    };
    let sources = table.map_or_else(TableSources::default, single_table_sources);
    (sources, PendingParamColumns::new())
}

fn record_assignment_context(
    assignment: &Assignment,
    default_qualifier: Option<&str>,
    pending_columns: &mut PendingParamColumns,
) {
    if is_placeholder(&assignment.value)
        && let Some(column) = assignment_column_context(&assignment.target, default_qualifier)
    {
        record_param_column(pending_columns, &assignment.value, column);
    }
}

fn insert_column_context(
    column: Option<&ObjectName>,
    qualifier: Option<&str>,
) -> Option<ColumnRef> {
    let parts = object_name_parts(column?);
    let [column_name] = parts.as_slice() else {
        return None;
    };
    Some(ColumnRef::new(qualifier?, column_name.clone()))
}

fn assignment_column_context(
    target: &AssignmentTarget,
    default_qualifier: Option<&str>,
) -> Option<ColumnRef> {
    let AssignmentTarget::ColumnName(name) = target else {
        return None;
    };
    let parts = object_name_parts(name);
    match parts.as_slice() {
        [column] => Some(ColumnRef::new(default_qualifier?, column.clone())),
        [qualifier, column] => Some(ColumnRef::new(qualifier.clone(), column.clone())),
        [schema, table, column] => {
            Some(ColumnRef::new(format!("{schema}.{table}"), column.clone()))
        }
        _ => None,
    }
}

fn resolve_mutation_params(
    mutation: &core::RawMutation,
    contexts: Vec<ParamContext>,
    schema: &SqliteSchema,
) -> core::DiagnosticResult<Vec<core::DbParamUsage>> {
    mutation
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
                return Err(mutation_param_usage_error(
                    mutation,
                    usage,
                    format!(
                        "Param `{}` references main-schema column `{}.{}` with an ambiguous SQLite declared type; add `valueType` to override inference",
                        usage.id(), column.table_name, column.column_name
                    ),
                ));
            } else {
                return Err(mutation_param_usage_error(
                    mutation,
                    usage,
                    format!(
                        "Param `{}` has no supported main-schema SQLite column context; add `valueType` to override inference",
                        usage.id()
                    ),
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
