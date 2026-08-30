use sqlay_adapters::metadata::sqlite::sqlx::SqlxSqliteMetadataProvider;
use sqlay_app::{MetadataProvider, MutationMetadataProvider};
use sqlay_core as core;

use super::support::{SqliteFixture, raw_mutation, raw_query, sqlite_url, unique_database_path};

#[test]
fn connection_diagnostic_names_url_env_without_leaking_url() {
    let secret_url = "sqlite:///private/secret/missing.sqlite?mode=rw";
    let provider = SqlxSqliteMetadataProvider::new(secret_url)
        .with_database_url_env("SECRET_SQLITE_DATABASE_URL");
    let query = raw_query("SELECT 1;", Vec::new());

    let report = provider
        .describe(&query, &core::AnalyzedQuery::new(core::Cardinality::Many))
        .expect_err("missing database file should fail connection");
    let message = report.diagnostics()[0].message();

    assert!(message.contains("SECRET_SQLITE_DATABASE_URL"), "{message}");
    assert!(message.contains("database.urlEnv"), "{message}");
    assert!(!message.contains(secret_url), "{message}");
    assert!(!message.contains("/private/secret"), "{message}");
}

#[test]
fn provider_rejects_missing_file_without_creating_it() {
    let path = unique_database_path();
    let url = sqlite_url(&path);
    let provider =
        SqlxSqliteMetadataProvider::new(&url).with_database_url_env("SQLITE_DATABASE_URL");
    let query = raw_query("SELECT 1;", Vec::new());

    let report = provider
        .describe(&query, &core::AnalyzedQuery::new(core::Cardinality::Many))
        .expect_err("a missing database file must be rejected before connect");
    let created = path.exists();
    let _ = std::fs::remove_file(&path);

    assert!(!created, "metadata lookup must never create a SQLite file");
    assert!(
        report.diagnostics()[0]
            .message()
            .contains("existing regular file")
    );
    assert!(!report.diagnostics()[0].message().contains(&url));
}

#[tokio::test]
async fn provider_rejects_creation_modes_and_in_memory_urls()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SqliteFixture::new("CREATE TABLE users (id INTEGER);").await?;
    let query = raw_query("SELECT 1;", Vec::new());

    for url in [
        format!("{}?mode=rwc", fixture.url()),
        "sqlite::memory:".to_owned(),
        "sqlite://:memory:".to_owned(),
    ] {
        let provider =
            SqlxSqliteMetadataProvider::new(&url).with_database_url_env("SQLITE_DATABASE_URL");
        let report = provider
            .describe(&query, &core::AnalyzedQuery::new(core::Cardinality::Many))
            .expect_err("temporary or file-creating SQLite URLs must be rejected");
        let message = report.diagnostics()[0].message();

        assert!(message.contains("SQLITE_DATABASE_URL"), "{message}");
        assert!(!message.contains(&url), "{message}");
    }

    Ok(())
}

#[tokio::test]
async fn describe_diagnostic_names_url_env_without_leaking_url()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SqliteFixture::new("CREATE TABLE users (id INTEGER);").await?;
    let provider =
        SqlxSqliteMetadataProvider::new(fixture.url()).with_database_url_env("SQLITE_DATABASE_URL");
    let query = raw_query("SELECT missing FROM users;", Vec::new());

    let report = provider
        .describe(&query, &core::AnalyzedQuery::new(core::Cardinality::Many))
        .expect_err("unknown result column should fail describe");
    let message = report.diagnostics()[0].message();

    assert!(message.contains("SQLITE_DATABASE_URL"), "{message}");
    assert!(message.contains("database.urlEnv"), "{message}");
    assert!(!message.contains(fixture.url()), "{message}");

    Ok(())
}

#[test]
fn mutation_connection_diagnostic_does_not_execute_or_leak_url() {
    let secret_url = "sqlite:///private/secret/mutation.sqlite?mode=rw";
    let provider = SqlxSqliteMetadataProvider::new(secret_url)
        .with_database_url_env("SECRET_SQLITE_DATABASE_URL");
    let mutation = raw_mutation("DELETE FROM users WHERE id = 1;", Vec::new());

    let report = provider
        .describe_mutation(
            &mutation,
            &core::AnalyzedMutation::new(core::MutationKind::Delete),
        )
        .expect_err("missing database file should fail connection");
    let message = report.diagnostics()[0].message();

    assert!(message.contains("SECRET_SQLITE_DATABASE_URL"), "{message}");
    assert!(!message.contains(secret_url), "{message}");
    assert!(!message.contains("/private/secret"), "{message}");
}
