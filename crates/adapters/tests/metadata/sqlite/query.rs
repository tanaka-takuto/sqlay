use sqlay_adapters::metadata::sqlite::sqlx::SqlxSqliteMetadataProvider;
use sqlay_app::MetadataProvider;
use sqlay_core as core;

use super::support::{SqliteFixture, column_reference, param, raw_query, typed_param};

const SCHEMA: &str = r"
CREATE TABLE values_by_affinity (
    id INTEGER PRIMARY KEY,
    integer_value BIGINT,
    real_value DOUBLE,
    text_value VARCHAR(80) NOT NULL,
    blob_value BLOB,
    bool_value BOOLEAN,
    numeric_value NUMERIC,
    decimal_value DECIMAL(12, 2),
    date_value DATE,
    time_value TIME,
    datetime_value DATETIME,
    timestamp_value TIMESTAMP,
    json_value JSON,
    unknown_value UUID,
    undeclared_value
);
CREATE VIEW values_view AS SELECT id FROM values_by_affinity;
CREATE TABLE child_values (
    id INTEGER PRIMARY KEY,
    parent_id INTEGER NOT NULL,
    label TEXT NOT NULL
);
CREATE TABLE text_primary_keys (
    id TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

#[tokio::test]
async fn query_metadata_uses_main_table_declarations_conservatively()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SqliteFixture::new(SCHEMA).await?;
    let provider = SqlxSqliteMetadataProvider::new(fixture.url());
    let query = raw_query(
        "SELECT v.id, v.integer_value, v.real_value, v.text_value, v.blob_value, \
         v.bool_value, v.numeric_value, v.decimal_value, v.date_value, v.time_value, \
         v.datetime_value, v.timestamp_value, v.json_value, v.unknown_value, \
         v.undeclared_value FROM values_by_affinity AS v;",
        Vec::new(),
    );

    let metadata = provider.describe(&query, &core::AnalyzedQuery::new(core::Cardinality::Many))?;
    let types = metadata
        .columns()
        .iter()
        .map(|column| (column.name(), column.ty()))
        .collect::<Vec<_>>();

    assert_eq!(
        types,
        [
            ("id", core::CoreType::Int64),
            ("integer_value", core::CoreType::Int64),
            ("real_value", core::CoreType::Float64),
            ("text_value", core::CoreType::String),
            ("blob_value", core::CoreType::Bytes),
            ("bool_value", core::CoreType::Bool),
            ("numeric_value", core::CoreType::Unknown),
            ("decimal_value", core::CoreType::Unknown),
            ("date_value", core::CoreType::Unknown),
            ("time_value", core::CoreType::Unknown),
            ("datetime_value", core::CoreType::Unknown),
            ("timestamp_value", core::CoreType::Unknown),
            ("json_value", core::CoreType::Unknown),
            ("unknown_value", core::CoreType::Unknown),
            ("undeclared_value", core::CoreType::Unknown),
        ]
    );
    for column in metadata.columns() {
        assert_eq!(
            column.nullable(),
            None,
            "an ambiguous projection must use prepare-only metadata"
        );
    }

    Ok(())
}

#[tokio::test]
async fn query_metadata_attaches_provenance_only_to_direct_main_table_columns()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SqliteFixture::new(SCHEMA).await?;
    let provider = SqlxSqliteMetadataProvider::new(fixture.url());
    let query = raw_query(
        "SELECT v.id AS valueId, v.id + 1 AS nextId, COUNT(*) AS total \
         FROM main.values_by_affinity AS v;",
        Vec::new(),
    );

    let metadata = provider.describe(&query, &core::AnalyzedQuery::new(core::Cardinality::Many))?;

    assert_eq!(metadata.columns()[0].ty(), core::CoreType::Int64);
    assert_eq!(metadata.columns()[0].nullable(), None);
    assert_eq!(
        metadata.columns()[0].schema_column_reference(),
        Some(&column_reference("values_by_affinity", "id"))
    );
    for column in &metadata.columns()[1..] {
        assert_eq!(column.ty(), core::CoreType::Unknown);
        assert_eq!(column.nullable(), None);
        assert_eq!(column.schema_column_reference(), None);
    }

    Ok(())
}

#[tokio::test]
async fn query_metadata_combines_describe_and_schema_nullability_conservatively()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SqliteFixture::new(SCHEMA).await?;
    let provider = SqlxSqliteMetadataProvider::new(fixture.url());
    let query = raw_query(
        "SELECT parent.id, child.label \
         FROM values_by_affinity AS parent \
         LEFT JOIN child_values AS child ON child.parent_id = parent.id;",
        Vec::new(),
    );

    let metadata = provider.describe(&query, &core::AnalyzedQuery::new(core::Cardinality::Many))?;

    assert_eq!(metadata.columns()[0].nullable(), Some(false));
    assert_eq!(
        metadata.columns()[1].nullable(),
        None,
        "conflicting schema and outer-join describe evidence must stay unknown"
    );

    Ok(())
}

#[tokio::test]
async fn query_metadata_does_not_treat_ordinary_text_primary_key_as_non_null()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SqliteFixture::new(SCHEMA).await?;
    let provider = SqlxSqliteMetadataProvider::new(fixture.url());
    let query = raw_query("SELECT t.id FROM text_primary_keys AS t;", Vec::new());

    let metadata = provider.describe(&query, &core::AnalyzedQuery::new(core::Cardinality::Many))?;

    assert_eq!(metadata.columns()[0].nullable(), Some(true));

    Ok(())
}

#[tokio::test]
async fn query_metadata_does_not_trust_view_column_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SqliteFixture::new(SCHEMA).await?;
    let provider = SqlxSqliteMetadataProvider::new(fixture.url());
    let query = raw_query("SELECT v.id FROM values_view AS v;", Vec::new());

    let metadata = provider.describe(&query, &core::AnalyzedQuery::new(core::Cardinality::Many))?;
    let column = &metadata.columns()[0];

    assert_eq!(column.ty(), core::CoreType::Unknown);
    assert_eq!(column.nullable(), None);
    assert_eq!(column.schema_column_reference(), None);

    Ok(())
}

#[tokio::test]
async fn query_metadata_infers_qualified_comparison_params_and_honors_value_type()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SqliteFixture::new(SCHEMA).await?;
    let provider = SqlxSqliteMetadataProvider::new(fixture.url());
    let query = raw_query(
        "SELECT v.id FROM values_by_affinity AS v \
         WHERE v.id = ? AND v.text_value <> ? AND lower(v.text_value) = ?;",
        vec![
            param("id"),
            param("text"),
            typed_param("normalized", core::CoreType::String),
        ],
    );

    let metadata = provider.describe(&query, &core::AnalyzedQuery::new(core::Cardinality::Many))?;

    assert_eq!(metadata.param_usages()[0].ty(), core::CoreType::Int64);
    assert_eq!(
        metadata.param_usages()[0].schema_column_reference(),
        Some(&column_reference("values_by_affinity", "id"))
    );
    assert_eq!(metadata.param_usages()[1].ty(), core::CoreType::String);
    assert_eq!(metadata.param_usages()[2].ty(), core::CoreType::String);

    Ok(())
}

#[tokio::test]
async fn query_metadata_requires_value_type_for_ambiguous_declared_param_column()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SqliteFixture::new(SCHEMA).await?;
    let provider = SqlxSqliteMetadataProvider::new(fixture.url());
    let untyped = raw_query(
        "SELECT v.id FROM values_by_affinity AS v WHERE v.numeric_value = ?;",
        vec![param("numeric")],
    );

    let report = provider
        .describe(&untyped, &core::AnalyzedQuery::new(core::Cardinality::Many))
        .expect_err("ambiguous SQLite declarations must require valueType");
    assert!(
        report.diagnostics()[0].message().contains("valueType"),
        "{}",
        report.diagnostics()[0].message()
    );

    let typed = raw_query(
        "SELECT v.id FROM values_by_affinity AS v WHERE v.numeric_value = ?;",
        vec![typed_param("numeric", core::CoreType::Decimal)],
    );
    let metadata = provider.describe(&typed, &core::AnalyzedQuery::new(core::Cardinality::Many))?;
    assert_eq!(metadata.param_usages()[0].ty(), core::CoreType::Decimal);
    assert_eq!(
        metadata.param_usages()[0].schema_column_reference(),
        Some(&column_reference("values_by_affinity", "numeric_value"))
    );

    Ok(())
}
