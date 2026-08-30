use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use sqlay_core as core;
use sqlx::{Connection, Executor, SqliteConnection};

pub(super) struct SqliteFixture {
    path: PathBuf,
    url: String,
}

impl SqliteFixture {
    pub(super) async fn new(schema: &'static str) -> Result<Self, Box<dyn std::error::Error>> {
        let path = unique_database_path();
        let url = sqlite_url(&path);
        let create_url = format!("{url}?mode=rwc");
        let mut connection = SqliteConnection::connect(&create_url).await?;
        connection.execute(schema).await?;
        connection.close().await?;

        Ok(Self { path, url })
    }

    pub(super) fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for SqliteFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_file(self.path.with_extension("sqlite-shm"));
        let _ = std::fs::remove_file(self.path.with_extension("sqlite-wal"));
    }
}

pub(super) fn raw_query(sql: &str, params: Vec<core::ParamUsage>) -> core::RawQuery {
    core::RawQuery::new(
        core::QueryMetadata::new("sqliteQuery".to_owned(), None),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_param_usages(params)
}

pub(super) fn raw_mutation(sql: &str, params: Vec<core::ParamUsage>) -> core::RawMutation {
    core::RawMutation::new(
        core::MutationMetadata::new("sqliteMutation".to_owned()),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_param_usages(params)
}

pub(super) fn param(id: &str) -> core::ParamUsage {
    core::ParamUsage::new(id.to_owned(), None, false, core::SourceLocation::unknown())
}

pub(super) fn typed_param(id: &str, ty: core::CoreType) -> core::ParamUsage {
    core::ParamUsage::new(
        id.to_owned(),
        Some(ty),
        false,
        core::SourceLocation::unknown(),
    )
}

pub(super) fn column_reference(table: &str, column: &str) -> core::ColumnTypeReference {
    core::ColumnTypeReference::new(None, table.to_owned(), column.to_owned())
}

pub(super) fn explicit_main_column_reference(
    table: &str,
    column: &str,
) -> core::ColumnTypeReference {
    core::ColumnTypeReference::new(Some("main".to_owned()), table.to_owned(), column.to_owned())
}

pub(super) fn unique_database_path() -> PathBuf {
    static NEXT_DATABASE_ID: AtomicU64 = AtomicU64::new(0);

    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "sqlay-sqlite-metadata-{}-{unique}-{}.sqlite",
        std::process::id(),
        NEXT_DATABASE_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

pub(super) fn sqlite_url(path: &std::path::Path) -> String {
    format!("sqlite://{}", path.display())
}
