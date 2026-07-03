use sqlay_core as core;

use super::super::schema_columns::MysqlSchemaTableRef;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::metadata::mysql::sqlx) struct ResolvedSchemaTypeRef {
    pub(in crate::metadata::mysql::sqlx) type_ref: core::CoreTypeRef,
    pub(in crate::metadata::mysql::sqlx) schema_column_reference: Option<core::ColumnTypeReference>,
}

impl ResolvedSchemaTypeRef {
    pub(in crate::metadata::mysql::sqlx) const fn new(
        type_ref: core::CoreTypeRef,
        schema_column_reference: Option<core::ColumnTypeReference>,
    ) -> Self {
        Self {
            type_ref,
            schema_column_reference,
        }
    }

    pub(in crate::metadata::mysql::sqlx) fn schema_column(
        type_ref: core::CoreTypeRef,
        table_ref: &MysqlSchemaTableRef,
        column_name: &str,
    ) -> Self {
        Self::new(type_ref, Some(table_ref.column_type_reference(column_name)))
    }
}
