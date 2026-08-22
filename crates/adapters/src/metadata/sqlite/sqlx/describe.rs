use sqlay_app::{MetadataProvider, MutationMetadataProvider};
use sqlay_core as core;
use sqlx::{AssertSqlSafe, Column, Executor, SqlSafeStr, SqliteConnection, Statement};

use super::connection::connect_sqlite;
use super::diagnostics::{
    configured_source, mutation_database_error, mutation_error, query_database_error, query_error,
};
use super::inference::{infer_mutation_params, infer_query};
use super::result_mapping::{map_direct_result_column, map_unknown_result_column};
use super::schema::fetch_main_schema;

/// sqlx-backed `SQLite` metadata provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlxSqliteMetadataProvider {
    database_url: String,
    database_url_env: Option<String>,
}

impl SqlxSqliteMetadataProvider {
    /// Build a provider for an existing configured `SQLite` database file.
    #[must_use]
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
            database_url_env: None,
        }
    }

    /// Attach the configured `database.urlEnv` name for diagnostics.
    #[must_use]
    pub fn with_database_url_env(mut self, env_name: impl Into<String>) -> Self {
        self.database_url_env = Some(env_name.into());
        self
    }

    /// Configured database URL.
    #[must_use]
    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    /// Configured environment variable name used to read the database URL.
    #[must_use]
    pub fn database_url_env(&self) -> Option<&str> {
        self.database_url_env.as_deref()
    }
}

impl MetadataProvider for SqlxSqliteMetadataProvider {
    fn describe(
        &self,
        query: &core::RawQuery,
        _analysis: &core::AnalyzedQuery,
    ) -> core::DiagnosticResult<core::DbQueryMetadata> {
        if tokio::runtime::Handle::try_current().is_ok() {
            describe_query_on_worker_thread(
                self.database_url.clone(),
                self.database_url_env.clone(),
                query.clone(),
            )
        } else {
            describe_query_blocking(self.database_url(), self.database_url_env(), query)
        }
    }
}

impl MutationMetadataProvider for SqlxSqliteMetadataProvider {
    fn describe_mutation(
        &self,
        mutation: &core::RawMutation,
        _analysis: &core::AnalyzedMutation,
    ) -> core::DiagnosticResult<core::DbMutationMetadata> {
        if tokio::runtime::Handle::try_current().is_ok() {
            describe_mutation_on_worker_thread(
                self.database_url.clone(),
                self.database_url_env.clone(),
                mutation.clone(),
            )
        } else {
            describe_mutation_blocking(self.database_url(), self.database_url_env(), mutation)
        }
    }
}

fn describe_query_on_worker_thread(
    database_url: String,
    database_url_env: Option<String>,
    query: core::RawQuery,
) -> core::DiagnosticResult<core::DbQueryMetadata> {
    let error_query = query.clone();
    std::thread::spawn(move || {
        describe_query_blocking(&database_url, database_url_env.as_deref(), &query)
    })
    .join()
    .unwrap_or_else(|_| {
        Err(query_error(
            &error_query,
            "SQLite metadata worker thread panicked",
        ))
    })
}

fn describe_mutation_on_worker_thread(
    database_url: String,
    database_url_env: Option<String>,
    mutation: core::RawMutation,
) -> core::DiagnosticResult<core::DbMutationMetadata> {
    let error_mutation = mutation.clone();
    std::thread::spawn(move || {
        describe_mutation_blocking(&database_url, database_url_env.as_deref(), &mutation)
    })
    .join()
    .unwrap_or_else(|_| {
        Err(mutation_error(
            &error_mutation,
            "SQLite mutation metadata worker thread panicked",
        ))
    })
}

fn describe_query_blocking(
    database_url: &str,
    database_url_env: Option<&str>,
    query: &core::RawQuery,
) -> core::DiagnosticResult<core::DbQueryMetadata> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            query_error(
                query,
                format!(
                    "failed to create SQLite metadata runtime{}: {error}",
                    configured_source(database_url_env)
                ),
            )
        })?;
    runtime.block_on(describe_query_metadata(
        database_url,
        database_url_env,
        query,
    ))
}

fn describe_mutation_blocking(
    database_url: &str,
    database_url_env: Option<&str>,
    mutation: &core::RawMutation,
) -> core::DiagnosticResult<core::DbMutationMetadata> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            mutation_error(
                mutation,
                format!(
                    "failed to create SQLite mutation metadata runtime{}: {error}",
                    configured_source(database_url_env)
                ),
            )
        })?;
    runtime.block_on(describe_mutation_metadata(
        database_url,
        database_url_env,
        mutation,
    ))
}

async fn describe_query_metadata(
    database_url: &str,
    database_url_env: Option<&str>,
    query: &core::RawQuery,
) -> core::DiagnosticResult<core::DbQueryMetadata> {
    let mut connection = connect_sqlite(database_url, database_url_env).await?;
    let schema = fetch_main_schema(&mut connection).await.map_err(|error| {
        query_database_error(
            query,
            "inspect main-schema",
            database_url_env,
            database_url,
            &error,
        )
    })?;
    let inference = infer_query(query, &schema)?;
    let described_columns =
        describe_query_columns(&mut connection, query, inference.requires_prepare_only)
            .await
            .map_err(|error| {
                query_database_error(
                    query,
                    "describe query",
                    database_url_env,
                    database_url,
                    &error,
                )
            })?;
    let use_hints = inference.result_columns.len() == described_columns.len();
    let columns = described_columns
        .iter()
        .enumerate()
        .map(|(index, described_column)| {
            if use_hints && let Some(schema_column) = inference.result_columns[index].as_ref() {
                map_direct_result_column(
                    &described_column.name,
                    schema_column,
                    described_column.nullable,
                )
            } else {
                map_unknown_result_column(&described_column.name)
            }
        })
        .collect();

    Ok(core::DbQueryMetadata::new(columns).with_param_usages(inference.param_usages))
}

struct DescribedQueryColumn {
    name: String,
    nullable: Option<bool>,
}

async fn describe_query_columns(
    connection: &mut SqliteConnection,
    query: &core::RawQuery,
    requires_prepare_only: bool,
) -> Result<Vec<DescribedQueryColumn>, sqlx::Error> {
    let sql = AssertSqlSafe(query.analysis_sql().to_owned()).into_sql_str();
    if requires_prepare_only {
        // sqlx 0.9 steps a SQLite statement when a result column has no declared
        // type or is an expression. Preparing still validates the statement and
        // exposes its result names without evaluating expressions.
        let statement = connection.prepare_with(sql, &[]).await?;
        Ok(statement
            .columns()
            .iter()
            .map(|column| DescribedQueryColumn {
                name: column.name().to_owned(),
                nullable: None,
            })
            .collect())
    } else {
        let description = connection.describe(sql).await?;
        Ok(description
            .columns()
            .iter()
            .enumerate()
            .map(|(index, column)| DescribedQueryColumn {
                name: column.name().to_owned(),
                nullable: description.nullable(index),
            })
            .collect())
    }
}

async fn describe_mutation_metadata(
    database_url: &str,
    database_url_env: Option<&str>,
    mutation: &core::RawMutation,
) -> core::DiagnosticResult<core::DbMutationMetadata> {
    let mut connection = connect_sqlite(database_url, database_url_env).await?;
    let schema = fetch_main_schema(&mut connection).await.map_err(|error| {
        mutation_database_error(
            mutation,
            "inspect main-schema",
            database_url_env,
            database_url,
            &error,
        )
    })?;
    let param_usages = infer_mutation_params(mutation, &schema)?;

    Ok(core::DbMutationMetadata::new().with_param_usages(param_usages))
}
