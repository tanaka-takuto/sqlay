use super::{CoreType, InputField, ParamBinding, ResultColumn};
use crate::ColumnTypeReference;

#[test]
fn input_field_equality_ignores_schema_column_reference() {
    assert_eq!(
        InputField::new("email".to_owned(), CoreType::String, false)
            .with_schema_column_reference(column_ref("users", "email")),
        InputField::new("email".to_owned(), CoreType::String, false)
            .with_schema_column_reference(column_ref("contacts", "email"))
    );
}

#[test]
fn param_binding_equality_ignores_schema_column_reference() {
    assert_eq!(
        ParamBinding::new("email".to_owned(), CoreType::String, false)
            .with_schema_column_reference(column_ref("users", "email")),
        ParamBinding::new("email".to_owned(), CoreType::String, false)
            .with_schema_column_reference(column_ref("contacts", "email"))
    );
}

#[test]
fn result_column_equality_ignores_schema_column_reference() {
    assert_eq!(
        ResultColumn::new("email".to_owned(), CoreType::String, false)
            .with_schema_column_reference(column_ref("users", "email")),
        ResultColumn::new("email".to_owned(), CoreType::String, false)
            .with_schema_column_reference(column_ref("contacts", "email"))
    );
}

fn column_ref(table: &str, column: &str) -> ColumnTypeReference {
    ColumnTypeReference::new(None, table.to_owned(), column.to_owned())
}
