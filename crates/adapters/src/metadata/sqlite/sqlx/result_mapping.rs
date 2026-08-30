use sqlay_core as core;

use super::schema::SqliteSchemaColumn;

pub(super) fn map_direct_result_column(
    name: &str,
    schema_column: &SqliteSchemaColumn,
    describe_nullable: Option<bool>,
) -> core::DbResultColumn {
    core::DbResultColumn::new(
        name.to_owned(),
        schema_column.ty,
        combine_nullability(describe_nullable, schema_column.nullable),
    )
    .with_schema_column_reference(schema_column.reference())
}

pub(super) fn map_unknown_result_column(name: &str) -> core::DbResultColumn {
    core::DbResultColumn::new(name.to_owned(), core::CoreType::Unknown, None)
}

pub(super) fn sqlite_declared_type_to_core_type(declared_type: &str) -> core::CoreType {
    let normalized = declared_type.trim().to_ascii_uppercase();

    if ["NUMERIC", "DECIMAL", "DATE", "TIME", "JSON", "BOOL"]
        .iter()
        .any(|ambiguous| normalized.contains(ambiguous))
    {
        core::CoreType::Unknown
    } else if normalized.contains("INT") {
        core::CoreType::Int64
    } else if normalized.contains("CHAR")
        || normalized.contains("CLOB")
        || normalized.contains("TEXT")
    {
        core::CoreType::String
    } else if normalized.contains("BLOB") {
        core::CoreType::Bytes
    } else if normalized.contains("REAL")
        || normalized.contains("FLOA")
        || normalized.contains("DOUB")
    {
        core::CoreType::Float64
    } else {
        core::CoreType::Unknown
    }
}

const fn combine_nullability(
    describe_nullable: Option<bool>,
    schema_nullable: Option<bool>,
) -> Option<bool> {
    match (describe_nullable, schema_nullable) {
        (Some(false), Some(false)) => Some(false),
        (Some(true), Some(true)) => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_only_precise_initial_sqlite_declared_type_families() {
        let cases = [
            ("INTEGER", core::CoreType::Int64),
            ("BIGINT", core::CoreType::Int64),
            ("REAL", core::CoreType::Float64),
            ("DOUBLE PRECISION", core::CoreType::Float64),
            ("TEXT", core::CoreType::String),
            ("VARCHAR(80)", core::CoreType::String),
            ("BLOB", core::CoreType::Bytes),
        ];

        for (declared_type, expected) in cases {
            assert_eq!(
                sqlite_declared_type_to_core_type(declared_type),
                expected,
                "{declared_type}"
            );
        }
    }

    #[test]
    fn maps_ambiguous_sqlite_declared_types_to_unknown() {
        for declared_type in [
            "",
            "NUMERIC",
            "DECIMAL(12, 2)",
            "DATE",
            "TIME",
            "DATETIME",
            "TIMESTAMP",
            "JSON",
            "JSON TEXT",
            "DATE TEXT",
            "DECIMAL INT",
            "TIMESTAMP INTEGER",
            "BOOL TEXT",
            "BOOL",
            "BOOLEAN",
            "BOOLEAN INT",
            "UUID",
        ] {
            assert_eq!(
                sqlite_declared_type_to_core_type(declared_type),
                core::CoreType::Unknown,
                "{declared_type}"
            );
        }
    }

    #[test]
    fn combines_describe_and_schema_nullability_only_when_they_agree() {
        assert_eq!(combine_nullability(Some(false), Some(false)), Some(false));
        assert_eq!(combine_nullability(Some(true), Some(true)), Some(true));
        assert_eq!(combine_nullability(Some(true), Some(false)), None);
        assert_eq!(combine_nullability(Some(false), Some(true)), None);
        assert_eq!(combine_nullability(None, Some(false)), None);
        assert_eq!(combine_nullability(Some(false), None), None);
    }
}
