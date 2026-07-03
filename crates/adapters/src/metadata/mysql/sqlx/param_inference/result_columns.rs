use sqlay_core as core;
use sqlparser::ast::{
    Expr, Function, FunctionArg, FunctionArgExpr, FunctionArguments, Ident, ObjectNamePart,
    SelectItem,
};

use super::super::schema_columns::MysqlSchemaColumn;
use super::tables::{
    SelectTableSources, resolve_query_schema_table_ref, select_from_query, select_table_sources,
};
use super::{ResolvedSchemaTypeRef, SchemaColumnTypes, parse_query, single_select_query};

pub(in crate::metadata::mysql::sqlx) fn resolve_result_column_type_refs(
    query: &core::RawQuery,
    schema_columns: &[MysqlSchemaColumn],
) -> core::DiagnosticResult<Vec<Option<ResolvedSchemaTypeRef>>> {
    if schema_columns.is_empty() {
        return Ok(Vec::new());
    }

    let statements = parse_query(query)?;
    let parsed_query = single_select_query(query, &statements)?;
    let select = select_from_query(parsed_query)
        .expect("single_select_query verifies this is a top-level SELECT query");
    let table_sources = select_table_sources(parsed_query, select);
    let schema = SchemaColumnTypes::from_columns(schema_columns);
    let mut result_type_refs = Vec::with_capacity(select.projection.len());

    for item in &select.projection {
        let type_ref = match item {
            SelectItem::UnnamedExpr(expr)
            | SelectItem::ExprWithAlias { expr, .. }
            | SelectItem::ExprWithAliases { expr, .. } => {
                resolve_projection_expr_type_ref(expr, &table_sources, &schema)
            }
            SelectItem::QualifiedWildcard(_, _) | SelectItem::Wildcard(_) => return Ok(Vec::new()),
        };
        result_type_refs.push(type_ref);
    }

    Ok(result_type_refs)
}

pub(in crate::metadata::mysql::sqlx) fn resolve_json_derived_result_columns(
    query: &core::RawQuery,
) -> core::DiagnosticResult<Vec<bool>> {
    let statements = parse_query(query)?;
    let parsed_query = single_select_query(query, &statements)?;
    let select = select_from_query(parsed_query)
        .expect("single_select_query verifies this is a top-level SELECT query");
    let mut json_derived_columns = Vec::with_capacity(select.projection.len());

    for item in &select.projection {
        let is_json_derived = match item {
            SelectItem::UnnamedExpr(expr)
            | SelectItem::ExprWithAlias { expr, .. }
            | SelectItem::ExprWithAliases { expr, .. } => is_json_derived_projection_expr(expr),
            SelectItem::QualifiedWildcard(_, _) | SelectItem::Wildcard(_) => return Ok(Vec::new()),
        };
        json_derived_columns.push(is_json_derived);
    }

    Ok(json_derived_columns)
}

fn resolve_projection_expr_type_ref(
    expr: &Expr,
    table_sources: &SelectTableSources,
    schema: &SchemaColumnTypes,
) -> Option<ResolvedSchemaTypeRef> {
    match expr {
        Expr::Identifier(column) => {
            resolve_unqualified_projection_column_type_ref(&column.value, table_sources, schema)
        }
        Expr::CompoundIdentifier(parts) => {
            let (qualifier, column_name) = direct_projection_column_ref(parts.as_slice())?;
            let table_ref = resolve_query_schema_table_ref(table_sources, schema, &qualifier)?;
            let type_ref = schema.get(&table_ref, &column_name)?;

            Some(ResolvedSchemaTypeRef::schema_column(
                type_ref,
                &table_ref,
                &column_name,
            ))
        }
        _ => None,
    }
}

fn is_json_derived_projection_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Function(function) if function_name_eq(function, "JSON_EXTRACT") => true,
        Expr::Function(function) if function_name_eq(function, "JSON_UNQUOTE") => function
            .first_expr_arg()
            .is_some_and(is_json_derived_projection_expr),
        Expr::JsonAccess { .. } => true,
        Expr::Nested(expr) => is_json_derived_projection_expr(expr),
        _ => false,
    }
}

trait FunctionJsonArgs {
    fn first_expr_arg(&self) -> Option<&Expr>;
}

impl FunctionJsonArgs for Function {
    fn first_expr_arg(&self) -> Option<&Expr> {
        let FunctionArguments::List(args) = &self.args else {
            return None;
        };
        let FunctionArg::Unnamed(FunctionArgExpr::Expr(expr)) = args.args.first()? else {
            return None;
        };
        Some(expr)
    }
}

fn function_name_eq(function: &Function, expected_name: &str) -> bool {
    matches!(
        function.name.0.as_slice(),
        [ObjectNamePart::Identifier(ident)] if ident.value.eq_ignore_ascii_case(expected_name)
    )
}

fn resolve_unqualified_projection_column_type_ref(
    column_name: &str,
    table_sources: &SelectTableSources,
    schema: &SchemaColumnTypes,
) -> Option<ResolvedSchemaTypeRef> {
    if table_sources.has_unsupported_table_source() {
        return None;
    }

    let mut candidates = table_sources
        .schema_table_refs
        .iter()
        .filter_map(|table_ref| {
            schema
                .get(table_ref, column_name)
                .map(|type_ref| (table_ref, type_ref))
        });
    let (table_ref, type_ref) = candidates.next()?;
    if candidates.next().is_some() {
        return None;
    }

    Some(ResolvedSchemaTypeRef::schema_column(
        type_ref,
        table_ref,
        column_name,
    ))
}

fn direct_projection_column_ref(parts: &[Ident]) -> Option<(String, String)> {
    match parts {
        [qualifier, column] => Some((qualifier.value.clone(), column.value.clone())),
        [database, table, column] => Some((
            format!("{}.{}", database.value, table.value),
            column.value.clone(),
        )),
        _ => None,
    }
}
