use sqlay_core as core;

pub(super) use crate::diagnostics::{
    mutation_error, mutation_param_usage_error, param_usage_error, query_error,
};

pub(super) fn connection_error(
    database_url_env: Option<&str>,
    database_url: &str,
    error: &sqlx::Error,
) -> core::DiagnosticReport {
    core::DiagnosticReport::new(core::Diagnostic::error(format!(
        "could not connect to SQLite database before validating SQL metadata{}: {}",
        configured_source(database_url_env),
        redacted_driver_message(error, database_url)
    )))
}

pub(super) fn database_configuration_error(
    database_url_env: Option<&str>,
    detail: &str,
) -> core::DiagnosticReport {
    core::DiagnosticReport::new(core::Diagnostic::error(format!(
        "invalid SQLite database configuration{}: {detail}",
        configured_source(database_url_env)
    )))
}

pub(super) fn query_database_error(
    query: &core::RawQuery,
    operation: &str,
    database_url_env: Option<&str>,
    database_url: &str,
    error: &sqlx::Error,
) -> core::DiagnosticReport {
    query_error(
        query,
        format!(
            "failed to {operation} SQLite metadata{}: {}",
            configured_source(database_url_env),
            redacted_driver_message(error, database_url)
        ),
    )
}

pub(super) fn mutation_database_error(
    mutation: &core::RawMutation,
    operation: &str,
    database_url_env: Option<&str>,
    database_url: &str,
    error: &sqlx::Error,
) -> core::DiagnosticReport {
    mutation_error(
        mutation,
        format!(
            "failed to {operation} SQLite metadata{}: {}",
            configured_source(database_url_env),
            redacted_driver_message(error, database_url)
        ),
    )
}

pub(super) fn configured_source(database_url_env: Option<&str>) -> String {
    database_url_env.map_or_else(String::new, |env_name| {
        format!(" using environment variable `{env_name}` configured by `database.urlEnv`")
    })
}

fn redacted_driver_message(error: &sqlx::Error, database_url: &str) -> String {
    let mut message = error.to_string();
    if !database_url.is_empty() {
        message = message.replace(database_url, "<redacted database URL>");
    }

    if let Some(sqlite_path) = database_url
        .strip_prefix("sqlite://")
        .and_then(|value| value.split('?').next())
        .filter(|value| !value.is_empty())
    {
        message = message.replace(sqlite_path, "<redacted database path>");
    }

    message
}
