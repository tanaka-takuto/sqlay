use sqlay_core as core;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::metadata::mysql::sqlx) struct ResolvedSchemaTypeRef {
    pub(in crate::metadata::mysql::sqlx) type_ref: core::CoreTypeRef,
    pub(in crate::metadata::mysql::sqlx) schema_column_reference: Option<core::ColumnTypeReference>,
}
