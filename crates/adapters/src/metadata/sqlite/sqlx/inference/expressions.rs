use sqlparser::ast::{
    BinaryOperator, Expr, FunctionArg, FunctionArgExpr, FunctionArguments, Query as SqlQuery, Value,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ColumnRef {
    pub(super) qualifier: String,
    pub(super) column: String,
}

impl ColumnRef {
    pub(super) fn new(qualifier: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            qualifier: qualifier.into(),
            column: column.into(),
        }
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn collect_expr_param_contexts(expr: &Expr, contexts: &mut Vec<Option<ColumnRef>>) {
    if is_placeholder(expr) {
        contexts.push(None);
        return;
    }

    match expr {
        Expr::BinaryOp { left, op, right } if is_supported_comparison_operator(op) => {
            if let Some(column) = qualified_column_ref(left)
                && is_placeholder(right)
            {
                contexts.push(Some(column));
            } else if is_placeholder(left)
                && let Some(column) = qualified_column_ref(right)
            {
                contexts.push(Some(column));
            } else {
                collect_expr_param_contexts(left, contexts);
                collect_expr_param_contexts(right, contexts);
            }
        }
        Expr::BinaryOp { left, right, .. }
        | Expr::AnyOp { left, right, .. }
        | Expr::AllOp { left, right, .. }
        | Expr::IsDistinctFrom(left, right)
        | Expr::IsNotDistinctFrom(left, right) => {
            collect_expr_param_contexts(left, contexts);
            collect_expr_param_contexts(right, contexts);
        }
        Expr::InList {
            expr,
            list,
            negated: false,
        } => {
            let column = qualified_column_ref(expr);
            for item in list {
                if is_placeholder(item) {
                    contexts.push(column.clone());
                } else {
                    collect_expr_param_contexts(item, contexts);
                }
            }
        }
        Expr::InList { expr, list, .. } => {
            collect_expr_param_contexts(expr, contexts);
            for item in list {
                collect_expr_param_contexts(item, contexts);
            }
        }
        Expr::Nested(expr)
        | Expr::UnaryOp { expr, .. }
        | Expr::Cast { expr, .. }
        | Expr::Extract { expr, .. }
        | Expr::Ceil { expr, .. }
        | Expr::Floor { expr, .. }
        | Expr::Collate { expr, .. }
        | Expr::IsFalse(expr)
        | Expr::IsNotFalse(expr)
        | Expr::IsTrue(expr)
        | Expr::IsNotTrue(expr)
        | Expr::IsNull(expr)
        | Expr::IsNotNull(expr)
        | Expr::IsUnknown(expr)
        | Expr::IsNotUnknown(expr) => collect_expr_param_contexts(expr, contexts),
        Expr::Between {
            expr, low, high, ..
        } => {
            collect_expr_param_contexts(expr, contexts);
            collect_expr_param_contexts(low, contexts);
            collect_expr_param_contexts(high, contexts);
        }
        Expr::Like { expr, pattern, .. }
        | Expr::ILike { expr, pattern, .. }
        | Expr::SimilarTo { expr, pattern, .. }
        | Expr::RLike { expr, pattern, .. } => {
            collect_expr_param_contexts(expr, contexts);
            collect_expr_param_contexts(pattern, contexts);
        }
        Expr::Function(function) => {
            collect_function_arguments(&function.parameters, contexts);
            collect_function_arguments(&function.args, contexts);
            if let Some(filter) = &function.filter {
                collect_expr_param_contexts(filter, contexts);
            }
        }
        Expr::Case {
            operand,
            conditions,
            else_result,
            ..
        } => {
            if let Some(operand) = operand {
                collect_expr_param_contexts(operand, contexts);
            }
            for condition in conditions {
                collect_expr_param_contexts(&condition.condition, contexts);
                collect_expr_param_contexts(&condition.result, contexts);
            }
            if let Some(else_result) = else_result {
                collect_expr_param_contexts(else_result, contexts);
            }
        }
        Expr::Tuple(items) => {
            for item in items {
                collect_expr_param_contexts(item, contexts);
            }
        }
        Expr::Exists { subquery, .. }
        | Expr::Subquery(subquery)
        | Expr::InSubquery { subquery, .. } => collect_query_params_as_unknown(subquery, contexts),
        _ => {}
    }
}

fn collect_function_arguments(
    arguments: &FunctionArguments,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match arguments {
        FunctionArguments::None => {}
        FunctionArguments::Subquery(query) => collect_query_params_as_unknown(query, contexts),
        FunctionArguments::List(list) => {
            for arg in &list.args {
                let arg = match arg {
                    FunctionArg::Named { arg, .. } | FunctionArg::Unnamed(arg) => arg,
                    FunctionArg::ExprNamed { name, arg, .. } => {
                        collect_expr_param_contexts(name, contexts);
                        arg
                    }
                };
                if let FunctionArgExpr::Expr(expr) = arg {
                    collect_expr_param_contexts(expr, contexts);
                }
            }
        }
    }
}

fn collect_query_params_as_unknown(query: &SqlQuery, contexts: &mut Vec<Option<ColumnRef>>) {
    let placeholder_count = query
        .to_string()
        .chars()
        .filter(|character| *character == '?')
        .count();
    contexts.extend(std::iter::repeat_n(None, placeholder_count));
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

pub(super) fn qualified_column_ref(expr: &Expr) -> Option<ColumnRef> {
    let Expr::CompoundIdentifier(parts) = expr else {
        return None;
    };
    let parts = parts
        .iter()
        .map(|part| part.value.clone())
        .collect::<Vec<_>>();
    if parts.iter().any(|part| part.contains('.')) {
        return None;
    }

    match parts.as_slice() {
        [qualifier, column] => Some(ColumnRef::new(qualifier.clone(), column.clone())),
        [schema, table, column] => {
            Some(ColumnRef::new(format!("{schema}.{table}"), column.clone()))
        }
        _ => None,
    }
}

pub(super) fn is_placeholder(expr: &Expr) -> bool {
    matches!(expr, Expr::Value(value) if matches!(&value.value, Value::Placeholder(value) if value == "?"))
}
