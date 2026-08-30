use sqlparser::ast::{Expr, Value};

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
