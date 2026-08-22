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
    resolve_existing_database_file(database_url, &current_dir)
        .map_err(|detail| database_configuration_error(database_url_env, detail))?;

    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(|error| connection_error(database_url_env, database_url, &error))?
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
    {
        return Err(
            "expected a plain file URL without temporary, in-memory, query, or fragment options",
        );
    }

    let configured_path = PathBuf::from(configured_path);
    let resolved_path = if configured_path.is_absolute() {
        configured_path
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

    fn unique_test_directory() -> PathBuf {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);

        std::env::temp_dir().join(format!(
            "sqlay-sqlite-url-test-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }
}
