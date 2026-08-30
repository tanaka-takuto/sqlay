//! `SQLite` dialect analysis adapter.

mod mutation;

#[cfg(test)]
mod tests;

use sqlay_app::DialectAnalyzer;
use sqlay_core as core;
use sqlparser::ast::{Expr, LimitClause, Query, SetExpr, Statement};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;
use sqlparser::tokenizer::{Token, Tokenizer};

use crate::diagnostics::{param_usage_error, query_error};

pub(super) const RAW_PLACEHOLDER_GUIDANCE: &str = "raw SQLite parameter placeholders are not supported in source SQL; use paired `@sqlay` Param markers around a sample expression";

/// `SQLite` dialect analyzer backed by `sqlparser-rs`.
#[derive(Clone, Copy, Debug, Default)]
pub struct SqliteDialectAnalyzer;

impl DialectAnalyzer for SqliteDialectAnalyzer {
    fn analyze(&self, query: &core::RawQuery) -> core::DiagnosticResult<core::AnalyzedQuery> {
        let tokens = tokenize_query(query)?;
        validate_query_placeholders(query, &tokens)?;
        let statements = Parser::parse_sql(&SQLiteDialect {}, query.analysis_sql())
            .map_err(|error| query_error(query, format!("failed to parse SQLite SQL: {error}")))?;

        let [statement] = statements.as_slice() else {
            return Err(query_error(
                query,
                format!(
                    "expected exactly one SQL statement per query block; found {}",
                    statements.len()
                ),
            ));
        };
        if !ends_with_statement_terminator(&tokens) {
            return Err(query_error(query, "query block must end with `;`"));
        }
        validate_query_param_sample_expressions(query)?;

        let Statement::Query(parsed_query) = statement else {
            return Err(unsupported_statement_error(query, statement));
        };
        if !is_select_query(parsed_query) {
            return Err(unsupported_statement_error(query, statement));
        }

        Ok(core::AnalyzedQuery::new(infer_cardinality(parsed_query)))
    }
}

fn tokenize_query(query: &core::RawQuery) -> core::DiagnosticResult<Vec<Token>> {
    Tokenizer::new(&SQLiteDialect {}, query.analysis_sql())
        .tokenize()
        .map_err(|error| query_error(query, format!("failed to parse SQLite SQL: {error}")))
}

fn validate_query_placeholders(
    query: &core::RawQuery,
    tokens: &[Token],
) -> core::DiagnosticResult<()> {
    let placeholder_count = sqlite_param_placeholder_count(tokens)
        .map_err(|()| query_error(query, RAW_PLACEHOLDER_GUIDANCE))?;
    let param_usage_count = query.param_usages().len();
    if param_usage_count == 0 && placeholder_count != 0 {
        return Err(query_error(query, RAW_PLACEHOLDER_GUIDANCE));
    }
    if placeholder_count != param_usage_count {
        return Err(query_error(
            query,
            format!(
                "generated placeholder count {placeholder_count} does not match Param usage count {param_usage_count}"
            ),
        ));
    }

    Ok(())
}

pub(super) fn sqlite_param_placeholder_count(tokens: &[Token]) -> Result<usize, ()> {
    let mut count = 0;
    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Placeholder(value) if value == "?" => count += 1,
            Token::Placeholder(_) => return Err(()),
            Token::Colon | Token::AtSign
                if tokens
                    .get(index + 1)
                    .is_some_and(|next| !matches!(next, Token::Whitespace(_))) =>
            {
                return Err(());
            }
            _ => {}
        }
    }

    Ok(count)
}

pub(super) fn ends_with_statement_terminator(tokens: &[Token]) -> bool {
    matches!(
        tokens
            .iter()
            .rev()
            .find(|token| !matches!(token, Token::Whitespace(_))),
        Some(Token::SemiColon)
    )
}

fn validate_query_param_sample_expressions(query: &core::RawQuery) -> core::DiagnosticResult<()> {
    for usage in query.param_usages() {
        validate_query_param_sample_expression(query, usage)?;
    }

    Ok(())
}

fn validate_query_param_sample_expression(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
) -> core::DiagnosticResult<()> {
    let trimmed = usage.sample_sql().trim();
    if trimmed.is_empty() {
        return Err(param_usage_error(
            query,
            usage,
            "Param range must contain exactly one SQL expression",
        ));
    }
    let mut parser = Parser::new(&SQLiteDialect {})
        .try_with_sql(trimmed)
        .map_err(|_| invalid_query_param_sample(query, usage))?;
    parser
        .parse_expr()
        .map_err(|_| invalid_query_param_sample(query, usage))?;
    if parser.peek_token_ref().token != Token::EOF {
        return Err(invalid_query_param_sample(query, usage));
    }

    Ok(())
}

fn invalid_query_param_sample(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
) -> core::DiagnosticReport {
    param_usage_error(
        query,
        usage,
        "Param range must contain exactly one SQL expression",
    )
}

fn is_select_query(query: &Query) -> bool {
    match query.body.as_ref() {
        SetExpr::Select(_) => true,
        SetExpr::Query(query) => is_select_query(query),
        SetExpr::SetOperation { left, right, .. } => {
            is_select_set_expression(left) && is_select_set_expression(right)
        }
        _ => false,
    }
}

fn is_select_set_expression(expression: &SetExpr) -> bool {
    match expression {
        SetExpr::Select(_) => true,
        SetExpr::Query(query) => is_select_query(query),
        SetExpr::SetOperation { left, right, .. } => {
            is_select_set_expression(left) && is_select_set_expression(right)
        }
        _ => false,
    }
}

fn infer_cardinality(query: &Query) -> core::Cardinality {
    if query.limit_clause.as_ref().is_some_and(limit_clause_is_one) {
        core::Cardinality::One
    } else {
        core::Cardinality::Many
    }
}

fn limit_clause_is_one(limit_clause: &LimitClause) -> bool {
    match limit_clause {
        LimitClause::LimitOffset {
            limit: Some(limit), ..
        }
        | LimitClause::OffsetCommaLimit { limit, .. } => expression_is_one(limit),
        LimitClause::LimitOffset { limit: None, .. } => false,
    }
}

fn expression_is_one(expression: &Expr) -> bool {
    matches!(expression, Expr::Value(value) if value.to_string() == "1")
}

fn unsupported_statement_error(
    query: &core::RawQuery,
    statement: &Statement,
) -> core::DiagnosticReport {
    query_error(
        query,
        format!(
            "unsupported SQLite SQL statement `{}`; supported statement kind is `SELECT`",
            statement_keyword(statement)
        ),
    )
}

pub(super) fn statement_keyword(statement: &Statement) -> String {
    statement
        .to_string()
        .split_whitespace()
        .next()
        .unwrap_or("SQL")
        .trim_end_matches(';')
        .to_ascii_uppercase()
}
