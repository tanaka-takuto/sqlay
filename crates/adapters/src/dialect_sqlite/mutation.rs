use sqlay_app::MutationAnalyzer;
use sqlay_core as core;
use sqlparser::ast::{
    Delete, FromTable, Insert, OnInsert, Query, SetExpr, SqliteOnConflict, Statement, TableFactor,
    TableObject, TableWithJoins, Update,
};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;
use sqlparser::tokenizer::{Token, Tokenizer};

use super::{
    RAW_PLACEHOLDER_GUIDANCE, SqliteDialectAnalyzer, ends_with_statement_terminator,
    sqlite_param_placeholder_count, statement_keyword,
};
use crate::diagnostics::{mutation_error, mutation_param_usage_error};

impl MutationAnalyzer for SqliteDialectAnalyzer {
    fn analyze_mutation(
        &self,
        mutation: &core::RawMutation,
    ) -> core::DiagnosticResult<core::AnalyzedMutation> {
        let tokens = tokenize_mutation(mutation)?;
        validate_mutation_placeholders(mutation, &tokens)?;
        if begins_with_keyword(&tokens, "WITH") {
            return Err(cte_mutation_error(mutation));
        }

        let statements =
            Parser::parse_sql(&SQLiteDialect {}, mutation.analysis_sql()).map_err(|error| {
                unsupported_unparsed_mutation_error(mutation, &tokens).unwrap_or_else(|| {
                    mutation_error(mutation, format!("failed to parse SQLite SQL: {error}"))
                })
            })?;
        let [statement] = statements.as_slice() else {
            return Err(mutation_error(
                mutation,
                format!(
                    "expected exactly one SQL statement per mutation block; found {}",
                    statements.len()
                ),
            ));
        };
        if !ends_with_statement_terminator(&tokens) {
            return Err(mutation_error(mutation, "mutation block must end with `;`"));
        }
        validate_mutation_param_sample_expressions(mutation)?;

        let kind = analyze_mutation_statement(
            mutation,
            statement,
            begins_with_keyword(&tokens, "REPLACE"),
        )?;
        Ok(core::AnalyzedMutation::new(kind))
    }
}

fn analyze_mutation_statement(
    mutation: &core::RawMutation,
    statement: &Statement,
    uses_replace_keyword: bool,
) -> core::DiagnosticResult<core::MutationKind> {
    match statement {
        Statement::Insert(insert) if uses_replace_keyword => {
            validate_insert_or_replace(mutation, insert, core::MutationKind::Replace)?;
            Ok(core::MutationKind::Replace)
        }
        Statement::Insert(insert) => {
            validate_insert_or_replace(mutation, insert, core::MutationKind::Insert)?;
            Ok(core::MutationKind::Insert)
        }
        Statement::Update(update) => {
            validate_update(mutation, update)?;
            Ok(core::MutationKind::Update)
        }
        Statement::Delete(delete) => {
            validate_delete(mutation, delete)?;
            Ok(core::MutationKind::Delete)
        }
        _ => Err(mutation_error(
            mutation,
            format!(
                "unsupported SQLite mutation SQL statement `{}`; supported statement kinds are `INSERT`, `REPLACE`, `UPDATE`, and `DELETE`",
                statement_keyword(statement)
            ),
        )),
    }
}

fn validate_insert_or_replace(
    mutation: &core::RawMutation,
    insert: &Insert,
    kind: core::MutationKind,
) -> core::DiagnosticResult<()> {
    if insert.returning.is_some() {
        return Err(returning_error(mutation));
    }
    let replace_conflict_clause =
        kind == core::MutationKind::Replace && insert.or == Some(SqliteOnConflict::Replace);
    if (insert.or.is_some() && !replace_conflict_clause)
        || matches!(insert.on, Some(OnInsert::OnConflict(_)))
    {
        return Err(mutation_error(
            mutation,
            "unsupported SQLite INSERT upsert; `ON CONFLICT` is outside the supported mutation scope",
        ));
    }
    if !insert.assignments.is_empty() {
        return Err(insert_set_error(mutation));
    }
    if !matches!(insert.table, TableObject::TableName(_))
        || insert.table_alias.is_some()
        || insert.output.is_some()
        || insert.overwrite
        || insert.partitioned.is_some()
        || !insert.after_columns.is_empty()
        || insert.has_table_keyword
        || insert.settings.is_some()
        || insert.format_clause.is_some()
        || insert.multi_table_insert_type.is_some()
        || !insert.multi_table_into_clauses.is_empty()
        || !insert.multi_table_when_clauses.is_empty()
        || insert.multi_table_else_clause.is_some()
    {
        return Err(unsupported_insert_or_replace_form(mutation, kind));
    }

    if insert.source.as_deref().is_some_and(query_is_values) {
        return Ok(());
    }
    if insert.source.is_some() {
        return Err(insert_or_replace_select_error(mutation, kind));
    }

    Err(unsupported_insert_or_replace_form(mutation, kind))
}

fn validate_update(mutation: &core::RawMutation, update: &Update) -> core::DiagnosticResult<()> {
    if update.returning.is_some() {
        return Err(returning_error(mutation));
    }
    if update.selection.is_none() {
        return Err(mutation_error(
            mutation,
            "SQLite UPDATE mutation requires a WHERE clause",
        ));
    }
    if update.from.is_some() {
        return Err(mutation_error(
            mutation,
            "unsupported SQLite `UPDATE ... FROM`; supported form is single-table `UPDATE ... WHERE`",
        ));
    }
    if !is_single_table_with_optional_alias(&update.table) {
        return Err(mutation_error(
            mutation,
            "unsupported multi-table SQLite UPDATE; supported form is single-table `UPDATE ... WHERE`",
        ));
    }
    if update.output.is_some()
        || update.or.is_some()
        || !update.order_by.is_empty()
        || update.limit.is_some()
    {
        return Err(mutation_error(
            mutation,
            "unsupported SQLite UPDATE form; supported form is single-table `UPDATE ... WHERE`",
        ));
    }

    Ok(())
}

fn validate_delete(mutation: &core::RawMutation, delete: &Delete) -> core::DiagnosticResult<()> {
    if delete.returning.is_some() {
        return Err(returning_error(mutation));
    }
    if delete.selection.is_none() {
        return Err(mutation_error(
            mutation,
            "SQLite DELETE mutation requires a WHERE clause",
        ));
    }
    if !delete.tables.is_empty()
        || delete.using.is_some()
        || !single_delete_from_table(&delete.from)
    {
        return Err(mutation_error(
            mutation,
            "unsupported multi-table SQLite DELETE; supported form is single-table `DELETE ... WHERE`",
        ));
    }
    if delete.output.is_some() || !delete.order_by.is_empty() || delete.limit.is_some() {
        return Err(mutation_error(
            mutation,
            "unsupported SQLite DELETE form; supported form is single-table `DELETE ... WHERE`",
        ));
    }

    Ok(())
}

fn query_is_values(query: &Query) -> bool {
    match query.body.as_ref() {
        SetExpr::Values(_) => true,
        SetExpr::Query(query) => query_is_values(query),
        _ => false,
    }
}

fn single_delete_from_table(from: &FromTable) -> bool {
    match from {
        FromTable::WithFromKeyword(tables) | FromTable::WithoutKeyword(tables) => {
            let [table] = tables.as_slice() else {
                return false;
            };
            is_single_table_with_optional_alias(table)
        }
    }
}

const fn is_single_table_with_optional_alias(table: &TableWithJoins) -> bool {
    table.joins.is_empty() && matches!(&table.relation, TableFactor::Table { args: None, .. })
}

fn tokenize_mutation(mutation: &core::RawMutation) -> core::DiagnosticResult<Vec<Token>> {
    Tokenizer::new(&SQLiteDialect {}, mutation.analysis_sql())
        .tokenize()
        .map_err(|error| mutation_error(mutation, format!("failed to parse SQLite SQL: {error}")))
}

fn validate_mutation_placeholders(
    mutation: &core::RawMutation,
    tokens: &[Token],
) -> core::DiagnosticResult<()> {
    let placeholder_count = sqlite_param_placeholder_count(tokens)
        .map_err(|()| mutation_error(mutation, RAW_PLACEHOLDER_GUIDANCE))?;
    let param_usage_count = mutation.param_usages().len();
    if param_usage_count == 0 && placeholder_count != 0 {
        return Err(mutation_error(mutation, RAW_PLACEHOLDER_GUIDANCE));
    }
    if placeholder_count != param_usage_count {
        return Err(mutation_error(
            mutation,
            format!(
                "generated placeholder count {placeholder_count} does not match Param usage count {param_usage_count}"
            ),
        ));
    }

    Ok(())
}

fn validate_mutation_param_sample_expressions(
    mutation: &core::RawMutation,
) -> core::DiagnosticResult<()> {
    for usage in mutation.param_usages() {
        let trimmed = usage.sample_sql().trim();
        let mut parser = Parser::new(&SQLiteDialect {})
            .try_with_sql(trimmed)
            .map_err(|_| invalid_mutation_param_sample(mutation, usage))?;
        parser
            .parse_expr()
            .map_err(|_| invalid_mutation_param_sample(mutation, usage))?;
        if trimmed.is_empty() || parser.peek_token_ref().token != Token::EOF {
            return Err(invalid_mutation_param_sample(mutation, usage));
        }
    }

    Ok(())
}

fn invalid_mutation_param_sample(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
) -> core::DiagnosticReport {
    mutation_param_usage_error(
        mutation,
        usage,
        "Param range must contain exactly one SQL expression",
    )
}

fn begins_with_keyword(tokens: &[Token], expected: &str) -> bool {
    tokens
        .iter()
        .find_map(|token| match token {
            Token::Whitespace(_) => None,
            Token::Word(word) if word.quote_style.is_none() => Some(word.value.as_str()),
            _ => Some(""),
        })
        .is_some_and(|word| word.eq_ignore_ascii_case(expected))
}

fn unsupported_unparsed_mutation_error(
    mutation: &core::RawMutation,
    tokens: &[Token],
) -> Option<core::DiagnosticReport> {
    let keywords = unquoted_words(tokens);
    match keywords.first().map(String::as_str) {
        Some("INSERT" | "REPLACE") if keywords.iter().any(|word| word == "SET") => {
            Some(insert_set_error(mutation))
        }
        Some("UPDATE") if keywords.iter().any(|word| word == "JOIN") => Some(mutation_error(
            mutation,
            "unsupported multi-table SQLite UPDATE; supported form is single-table `UPDATE ... WHERE`",
        )),
        Some("DELETE") if keywords.iter().any(|word| word == "USING") => Some(mutation_error(
            mutation,
            "unsupported multi-table SQLite DELETE; supported form is single-table `DELETE ... WHERE`",
        )),
        _ => None,
    }
}

fn unquoted_words(tokens: &[Token]) -> Vec<String> {
    tokens
        .iter()
        .filter_map(|token| match token {
            Token::Word(word) if word.quote_style.is_none() => {
                Some(word.value.to_ascii_uppercase())
            }
            _ => None,
        })
        .collect()
}

fn cte_mutation_error(mutation: &core::RawMutation) -> core::DiagnosticReport {
    mutation_error(
        mutation,
        "unsupported SQLite CTE mutation; `WITH ... INSERT/UPDATE/DELETE/REPLACE` is outside the supported mutation scope",
    )
}

fn returning_error(mutation: &core::RawMutation) -> core::DiagnosticReport {
    mutation_error(
        mutation,
        "unsupported SQLite mutation `RETURNING`; mutation builders do not return rows",
    )
}

fn insert_set_error(mutation: &core::RawMutation) -> core::DiagnosticReport {
    mutation_error(
        mutation,
        "unsupported SQLite INSERT ... SET; supported form is `INSERT ... VALUES`",
    )
}

fn insert_or_replace_select_error(
    mutation: &core::RawMutation,
    kind: core::MutationKind,
) -> core::DiagnosticReport {
    let keyword = mutation_kind_keyword(kind);
    mutation_error(
        mutation,
        format!(
            "unsupported SQLite {keyword} ... SELECT; supported form is `{keyword} ... VALUES`"
        ),
    )
}

fn unsupported_insert_or_replace_form(
    mutation: &core::RawMutation,
    kind: core::MutationKind,
) -> core::DiagnosticReport {
    let keyword = mutation_kind_keyword(kind);
    mutation_error(
        mutation,
        format!("unsupported SQLite {keyword} form; supported form is `{keyword} ... VALUES`"),
    )
}

const fn mutation_kind_keyword(kind: core::MutationKind) -> &'static str {
    match kind {
        core::MutationKind::Insert => "INSERT",
        core::MutationKind::Replace => "REPLACE",
        core::MutationKind::Update | core::MutationKind::Delete => unreachable!(),
    }
}
