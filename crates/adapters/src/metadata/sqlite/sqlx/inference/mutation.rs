use sqlay_core as core;
use sqlparser::ast::{
    Assignment, AssignmentTarget, Delete, FromTable, Insert, ObjectName, SetExpr, Statement,
    TableObject, Update,
};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;

use super::super::diagnostics::{mutation_error, mutation_param_usage_error};
use super::super::schema::SqliteSchema;
use super::expressions::{ColumnRef, collect_expr_param_contexts, is_placeholder};
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
    let (sources, contexts) = mutation_contexts(statement, mutation.param_usages().len());
    reject_unsupported_schema_qualifier(mutation, &sources)?;

    resolve_mutation_params(mutation, contexts, &sources, schema)
}

fn reject_unsupported_schema_qualifier(
    mutation: &core::RawMutation,
    sources: &TableSources,
) -> core::DiagnosticResult<()> {
    let Some(qualifier) = sources.unsupported_schema_qualifier() else {
        return Ok(());
    };

    Err(mutation_error(
        mutation,
        format!(
            "unsupported SQLite schema qualifier `{qualifier}`; only the main schema is supported, using `table` or `main.table` references"
        ),
    ))
}

fn mutation_contexts(
    statement: &Statement,
    expected_count: usize,
) -> (TableSources, Vec<Option<ColumnRef>>) {
    let (sources, mut contexts) = match statement {
        Statement::Insert(insert) => insert_contexts(insert),
        Statement::Update(update) => update_contexts(update),
        Statement::Delete(delete) => delete_contexts(delete),
        _ => (TableSources::default(), Vec::new()),
    };
    if contexts.len() != expected_count {
        contexts = vec![None; expected_count];
    }
    (sources, contexts)
}

fn insert_contexts(insert: &Insert) -> (TableSources, Vec<Option<ColumnRef>>) {
    let TableObject::TableName(table_name) = &insert.table else {
        return (TableSources::default(), Vec::new());
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
    let mut contexts = Vec::new();

    if let Some(source) = &insert.source
        && let SetExpr::Values(values) = source.body.as_ref()
    {
        for row in &values.rows {
            for (index, expr) in row.iter().enumerate() {
                if is_placeholder(expr) {
                    contexts.push(insert_column_context(
                        insert.columns.get(index),
                        qualifier.as_deref(),
                    ));
                } else {
                    collect_expr_param_contexts(expr, &mut contexts);
                }
            }
        }
    }

    (sources, contexts)
}

fn update_contexts(update: &Update) -> (TableSources, Vec<Option<ColumnRef>>) {
    let sources = single_table_sources(&update.table);
    let qualifier = table_with_joins_default_qualifier(&update.table);
    let mut contexts = Vec::new();
    for assignment in &update.assignments {
        collect_assignment_context(assignment, qualifier.as_deref(), &mut contexts);
    }
    if let Some(selection) = &update.selection {
        collect_expr_param_contexts(selection, &mut contexts);
    }
    (sources, contexts)
}

fn delete_contexts(delete: &Delete) -> (TableSources, Vec<Option<ColumnRef>>) {
    let table = match &delete.from {
        FromTable::WithFromKeyword(tables) | FromTable::WithoutKeyword(tables) => tables.first(),
    };
    let sources = table.map_or_else(TableSources::default, single_table_sources);
    let mut contexts = Vec::new();
    if let Some(selection) = &delete.selection {
        collect_expr_param_contexts(selection, &mut contexts);
    }
    (sources, contexts)
}

fn collect_assignment_context(
    assignment: &Assignment,
    default_qualifier: Option<&str>,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if is_placeholder(&assignment.value) {
        contexts.push(assignment_column_context(
            &assignment.target,
            default_qualifier,
        ));
    } else {
        collect_expr_param_contexts(&assignment.value, contexts);
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
    contexts: Vec<Option<ColumnRef>>,
    sources: &TableSources,
    schema: &SqliteSchema,
) -> core::DiagnosticResult<Vec<core::DbParamUsage>> {
    mutation
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
            if let Some(column) = schema_column {
                param = param.with_schema_column_reference(column.reference());
            }
            Ok(param)
        })
        .collect()
}
