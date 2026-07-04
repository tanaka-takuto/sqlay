use sqlay_core as core;

pub(super) use crate::diagnostics::{
    mutation_error, mutation_param_usage_error, param_usage_error, query_error,
};

pub(super) fn connection_error(
    database_url_env: Option<&str>,
    database_url: &str,
    error: &::sqlx::Error,
) -> core::DiagnosticReport {
    let source = database_url_env.map_or_else(String::new, |env_name| {
        format!(" using environment variable `{env_name}` configured by `database.urlEnv`")
    });
    let driver_message = driver_error_message(error, database_url);

    core::DiagnosticReport::new(core::Diagnostic::error(format!(
        "could not connect to MySQL database before validating SQL metadata{source}: {driver_message}"
    )))
}

fn driver_error_message(error: &::sqlx::Error, database_url: &str) -> String {
    let mut message = error.to_string();

    if let ::sqlx::Error::Database(database_error) = error
        && let Some(code) = database_error.code()
    {
        let code = code.as_ref();
        if !code.is_empty() && !message.contains(code) {
            message.push_str(" (code ");
            message.push_str(code);
            message.push(')');
        }
    }

    if !database_url.is_empty() {
        message = message.replace(database_url, "<redacted database URL>");
    }

    message
}
