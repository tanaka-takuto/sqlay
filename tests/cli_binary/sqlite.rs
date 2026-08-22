use std::process::Command;

use crate::support::{TEST_DATABASE_URL_ENV, unique_temp_dir};

const SQLITE_CONFIG: &str = r#"
{
  "source": {
    "include": ["sql/**/*.sql"]
  },
  "output": {
    "dir": "generated"
  },
  "database": {
    "dialect": "sqlite",
    "urlEnv": "SQLAY_TEST_DATABASE_URL"
  },
  "target": {
    "language": "typescript"
  }
}
"#;

const SELECT_LITERAL_SQL: &str = r"
/* @sqlay
{
  type: query
  id: selectLiteral
}
*/
SELECT 1 AS value;
";

const UNSUPPORTED_SQLITE_RETURNING_SQL: &str = r"
/* @sqlay
{
  type: mutation
  id: createUser
}
*/
INSERT INTO users (id) VALUES (1) RETURNING id;
";

#[test]
fn check_dispatches_sqlite_mutations_to_the_sqlite_analyzer_before_reading_the_url() {
    let project_dir = write_sqlite_project(
        "sqlay-cli-sqlite-mutation-analyzer",
        UNSUPPORTED_SQLITE_RETURNING_SQL,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("check")
        .current_dir(&project_dir)
        .env_remove(TEST_DATABASE_URL_ENV)
        .output()
        .expect("sqlay check should run");

    assert_eq!(output.status.code(), Some(1), "status: {:?}", output.status);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unsupported SQLite mutation `RETURNING`"),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("database.urlEnv"), "stderr: {stderr}");
    assert!(!stderr.contains("MySQL"), "stderr: {stderr}");

    std::fs::remove_dir_all(project_dir).expect("temp SQLite project should be removed");
}

#[test]
fn check_dispatches_sqlite_projects_to_the_sqlite_metadata_provider() {
    let project_dir = write_sqlite_project("sqlay-cli-sqlite-provider", SELECT_LITERAL_SQL);
    let secret_url = "mysql://secret-user:secret-password@example.test/sqlay";

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("check")
        .current_dir(&project_dir)
        .env(TEST_DATABASE_URL_ENV, secret_url)
        .output()
        .expect("sqlay check should run");

    assert_eq!(output.status.code(), Some(1), "status: {:?}", output.status);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid SQLite database configuration"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains(TEST_DATABASE_URL_ENV), "stderr: {stderr}");
    assert!(!stderr.contains(secret_url), "stderr: {stderr}");
    assert!(!stderr.contains("secret-password"), "stderr: {stderr}");

    std::fs::remove_dir_all(project_dir).expect("temp SQLite project should be removed");
}

#[test]
fn check_reads_an_existing_sqlite_file_without_writing_generated_output() {
    let project_dir = write_sqlite_project("sqlay-cli-sqlite-check", SELECT_LITERAL_SQL);
    let database_path = project_dir.join("database.sqlite3");
    std::fs::File::create(&database_path).expect("empty SQLite database file should be created");
    let database_bytes_before = std::fs::read(&database_path).expect("database should be readable");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("check")
        .current_dir(&project_dir)
        .env(TEST_DATABASE_URL_ENV, "sqlite://database.sqlite3")
        .output()
        .expect("sqlay check should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Check passed."), "stdout: {stdout}");
    assert!(stdout.contains("No files written."), "stdout: {stdout}");
    assert!(
        !project_dir.join("generated").exists(),
        "check must not write generated output"
    );
    assert_eq!(
        std::fs::read(&database_path).expect("database should remain readable"),
        database_bytes_before,
        "check must not modify the configured SQLite file"
    );
    for suffix in ["-wal", "-shm", "-journal"] {
        let sidecar = std::path::PathBuf::from(format!("{}{suffix}", database_path.display()));
        assert!(
            !sidecar.exists(),
            "check must not leave {}",
            sidecar.display()
        );
    }

    std::fs::remove_dir_all(project_dir).expect("temp SQLite project should be removed");
}

fn write_sqlite_project(prefix: &str, sql: &str) -> std::path::PathBuf {
    let project_dir = unique_temp_dir(prefix);
    let sql_dir = project_dir.join("sql");
    std::fs::create_dir_all(&sql_dir).expect("temp SQLite SQL dir should be created");
    std::fs::write(project_dir.join("sqlay.config.json"), SQLITE_CONFIG)
        .expect("temp SQLite config should be written");
    std::fs::write(sql_dir.join("query.sql"), sql).expect("temp SQLite SQL should be written");
    project_dir
}
