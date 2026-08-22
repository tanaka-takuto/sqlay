use sqlay_adapters::metadata::sqlite::sqlx::SqlxSqliteMetadataProvider;
use sqlay_app::MutationMetadataProvider;
use sqlay_core as core;
use sqlx::{Connection, Row, SqliteConnection};

use super::support::{SqliteFixture, column_reference, param, raw_mutation, typed_param};

const SCHEMA: &str = r"
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    email TEXT NOT NULL,
    score REAL,
    active BOOL,
    ambiguous NUMERIC
);
INSERT INTO users (id, email, score, active)
VALUES (1, 'ada@example.test', 2.5, TRUE);
";

#[tokio::test]
async fn mutation_metadata_infers_insert_values_target_columns()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SqliteFixture::new(SCHEMA).await?;
    let provider = SqlxSqliteMetadataProvider::new(fixture.url());
    let mutation = raw_mutation(
        "INSERT INTO users (email, score, active) VALUES (?, ?, ?);",
        vec![param("email"), param("score"), param("active")],
    );

    let metadata = provider.describe_mutation(
        &mutation,
        &core::AnalyzedMutation::new(core::MutationKind::Insert),
    )?;

    assert_eq!(
        metadata
            .param_usages()
            .iter()
            .map(core::DbParamUsage::ty)
            .collect::<Vec<_>>(),
        [
            core::CoreType::String,
            core::CoreType::Float64,
            core::CoreType::Bool,
        ]
    );
    assert_eq!(
        metadata.param_usages()[0].schema_column_reference(),
        Some(&column_reference("users", "email"))
    );

    Ok(())
}

#[tokio::test]
async fn mutation_metadata_requires_value_type_for_ambiguous_declared_param_column()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SqliteFixture::new(SCHEMA).await?;
    let provider = SqlxSqliteMetadataProvider::new(fixture.url());
    let untyped = raw_mutation(
        "UPDATE users AS u SET ambiguous = ? WHERE u.id = ?;",
        vec![param("ambiguous"), param("id")],
    );

    let report = provider
        .describe_mutation(
            &untyped,
            &core::AnalyzedMutation::new(core::MutationKind::Update),
        )
        .expect_err("ambiguous SQLite declarations must require valueType");
    assert!(
        report.diagnostics()[0].message().contains("valueType"),
        "{}",
        report.diagnostics()[0].message()
    );

    let typed = raw_mutation(
        "UPDATE users AS u SET ambiguous = ? WHERE u.id = ?;",
        vec![
            typed_param("ambiguous", core::CoreType::Decimal),
            param("id"),
        ],
    );
    let metadata = provider.describe_mutation(
        &typed,
        &core::AnalyzedMutation::new(core::MutationKind::Update),
    )?;
    assert_eq!(metadata.param_usages()[0].ty(), core::CoreType::Decimal);
    assert_eq!(
        metadata.param_usages()[0].schema_column_reference(),
        Some(&column_reference("users", "ambiguous"))
    );

    Ok(())
}

#[tokio::test]
async fn mutation_metadata_infers_update_set_and_qualified_predicate_params()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SqliteFixture::new(SCHEMA).await?;
    let provider = SqlxSqliteMetadataProvider::new(fixture.url());
    let mutation = raw_mutation(
        "UPDATE users AS u SET email = ?, score = ? WHERE u.id = ?;",
        vec![param("email"), param("score"), param("id")],
    );

    let metadata = provider.describe_mutation(
        &mutation,
        &core::AnalyzedMutation::new(core::MutationKind::Update),
    )?;

    assert_eq!(
        metadata
            .param_usages()
            .iter()
            .map(core::DbParamUsage::ty)
            .collect::<Vec<_>>(),
        [
            core::CoreType::String,
            core::CoreType::Float64,
            core::CoreType::Int64,
        ]
    );

    Ok(())
}

#[tokio::test]
async fn mutation_metadata_only_reads_schema_and_never_executes_delete()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SqliteFixture::new(SCHEMA).await?;
    let provider = SqlxSqliteMetadataProvider::new(fixture.url());
    let mutation = raw_mutation(
        "DELETE FROM users AS u WHERE u.id = ? AND lower(u.email) = ?;",
        vec![param("id"), typed_param("email", core::CoreType::String)],
    );

    let metadata = provider.describe_mutation(
        &mutation,
        &core::AnalyzedMutation::new(core::MutationKind::Delete),
    )?;
    assert_eq!(metadata.param_usages()[0].ty(), core::CoreType::Int64);
    assert_eq!(metadata.param_usages()[1].ty(), core::CoreType::String);

    let mut connection = SqliteConnection::connect(fixture.url()).await?;
    let row = sqlx::query("SELECT COUNT(*) AS row_count FROM users;")
        .fetch_one(&mut connection)
        .await?;
    let row_count: i64 = row.try_get("row_count")?;
    assert_eq!(
        row_count, 1,
        "metadata lookup must not execute mutation SQL"
    );

    Ok(())
}
