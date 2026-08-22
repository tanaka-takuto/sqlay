use std::path::{Path, PathBuf};
use std::str::FromStr;

use sqlay_core as core;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};

use super::diagnostics::{connection_error, database_configuration_error};

pub(super) async fn connect_sqlite(
    database_url: &str,
    database_url_env: Option<&str>,
) -> core::DiagnosticResult<SqliteConnection> {
    let current_dir = std::env::current_dir().map_err(|_| {
        database_configuration_error(
            database_url_env,
            "could not resolve the current directory for the relative database path",
        )
    })?;
    let resolved_path = resolve_existing_database_file(database_url, &current_dir)
        .map_err(|detail| database_configuration_error(database_url_env, detail))?;

    let options = SqliteConnectOptions::new()
        .filename(&resolved_path)
        .create_if_missing(false)
        .read_only(true);
    SqliteConnection::connect_with(&options)
        .await
        .map_err(|error| connection_error(database_url_env, database_url, &error))
}

fn resolve_existing_database_file(
    database_url: &str,
    current_dir: &Path,
) -> Result<PathBuf, &'static str> {
    let Some(configured_path) = database_url.strip_prefix("sqlite://") else {
        return Err(
            "expected `sqlite://relative/path` or `sqlite:///absolute/path` for an existing regular file",
        );
    };
    if configured_path.is_empty()
        || configured_path.contains(['?', '#'])
        || configured_path.eq_ignore_ascii_case(":memory:")
        || configured_path
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("sqlite:"))
    {
        return Err(
            "expected a plain file URL without temporary, in-memory, query, or fragment options",
        );
    }

    let parsed_options = SqliteConnectOptions::from_str(database_url).map_err(|_| {
        "expected a valid percent-encoded SQLite file URL without temporary, in-memory, query, or fragment options"
    })?;
    let configured_path = parsed_options.get_filename();
    let configured_path_text = configured_path.to_str().ok_or(
        "expected a valid percent-encoded SQLite file URL without temporary, in-memory, query, or fragment options",
    )?;
    if configured_path_text.eq_ignore_ascii_case(":memory:")
        || configured_path_text.contains(['?', '#'])
        || configured_path_text
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file:"))
        || configured_path_text
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("sqlite:"))
    {
        return Err(
            "expected a plain file URL without temporary, in-memory, query, or fragment options",
        );
    }

    let resolved_path = if configured_path.is_absolute() {
        configured_path.to_path_buf()
    } else {
        current_dir.join(configured_path)
    };
    if !resolved_path
        .metadata()
        .is_ok_and(|metadata| metadata.is_file())
    {
        return Err(
            "the configured SQLite database must be an existing regular file; sqlay will not create it",
        );
    }

    Ok(resolved_path)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    #[test]
    fn resolves_existing_relative_file_against_supplied_current_directory() {
        let directory = unique_test_directory();
        std::fs::create_dir(&directory).expect("test directory should be created");
        let database_path = directory.join("relative.sqlite");
        std::fs::File::create(&database_path).expect("test database file should be created");

        let resolved = resolve_existing_database_file("sqlite://relative.sqlite", &directory)
            .expect("existing relative database file should be accepted");

        assert_eq!(resolved, database_path);
        std::fs::remove_file(&database_path).expect("test database file should be removed");
        std::fs::remove_dir(&directory).expect("test directory should be removed");
    }

    #[test]
    fn rejects_existing_directory_as_database_file() {
        let directory = unique_test_directory();
        std::fs::create_dir(&directory).expect("test directory should be created");

        let error = resolve_existing_database_file("sqlite://.", &directory)
            .expect_err("an existing directory is not a regular database file");

        assert!(error.contains("existing regular file"), "{error}");
        std::fs::remove_dir(&directory).expect("test directory should be removed");
    }

    #[test]
    fn resolves_percent_encoded_path_before_validating_the_file() {
        let directory = unique_test_directory();
        let decoded_directory = directory.join("real");
        std::fs::create_dir_all(&decoded_directory).expect("test directories should be created");
        let encoded_path = directory.join("real%2Ffixture.sqlite");
        let decoded_path = decoded_directory.join("fixture.sqlite");
        std::fs::File::create(&encoded_path).expect("encoded decoy file should be created");
        std::fs::File::create(&decoded_path).expect("decoded database file should be created");

        let resolved = resolve_existing_database_file(
            &format!("sqlite://{}", encoded_path.display()),
            &directory,
        )
        .expect("the decoded existing file should be accepted");

        assert_eq!(resolved, decoded_path);
        std::fs::remove_file(&encoded_path).expect("encoded decoy file should be removed");
        std::fs::remove_file(&decoded_path).expect("decoded database file should be removed");
        std::fs::remove_dir(&decoded_directory).expect("decoded directory should be removed");
        std::fs::remove_dir(&directory).expect("test directory should be removed");
    }

    #[test]
    fn rejects_percent_encoded_in_memory_path() {
        let directory = unique_test_directory();
        std::fs::create_dir(&directory).expect("test directory should be created");
        let encoded_path = directory.join("%3Amemory%3A");
        std::fs::File::create(&encoded_path).expect("encoded decoy file should be created");

        let error = resolve_existing_database_file("sqlite://%3Amemory%3A", &directory)
            .expect_err("a decoded in-memory path must be rejected");

        assert!(error.contains("in-memory"), "{error}");
        std::fs::remove_file(&encoded_path).expect("encoded decoy file should be removed");
        std::fs::remove_dir(&directory).expect("test directory should be removed");
    }

    #[test]
    fn rejects_percent_encoded_file_uri_options() {
        let directory = unique_test_directory();
        std::fs::create_dir(&directory).expect("test directory should be created");
        let encoded_path = directory.join("file%3Asqlay%3Fmode%3Dmemory");
        std::fs::File::create(&encoded_path).expect("encoded decoy file should be created");

        let error =
            resolve_existing_database_file("sqlite://file%3Asqlay%3Fmode%3Dmemory", &directory)
                .expect_err("encoded SQLite URI options must be rejected");

        assert!(error.contains("query"), "{error}");
        std::fs::remove_file(&encoded_path).expect("encoded decoy file should be removed");
        std::fs::remove_dir(&directory).expect("test directory should be removed");
    }

    #[test]
    fn rejects_nested_sqlite_scheme_that_would_become_absolute() {
        let directory = unique_test_directory();
        std::fs::create_dir(&directory).expect("test directory should be created");
        let database_path = directory.join("fixture.sqlite");
        std::fs::File::create(&database_path).expect("test database file should be created");

        let error = resolve_existing_database_file(
            &format!("sqlite://sqlite:{}", database_path.display()),
            &directory,
        )
        .expect_err("a nested SQLite scheme must not change a relative path to absolute");

        assert!(error.contains("plain file URL"), "{error}");
        std::fs::remove_file(&database_path).expect("test database file should be removed");
        std::fs::remove_dir(&directory).expect("test directory should be removed");
    }

    fn unique_test_directory() -> PathBuf {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);

        std::env::temp_dir().join(format!(
            "sqlay-sqlite-url-test-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }
}
