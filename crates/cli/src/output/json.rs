use serde_json::{Map, Value, json};
use sqlay_app as app;
use sqlay_core as core;
use std::path::Path;

use crate::args::{ConfiguredCommand, OutputFormat};

pub fn print_json_failure_result(
    command: ConfiguredCommand,
    config_path: Option<&Path>,
    report: &core::DiagnosticReport,
) {
    println!(
        "{}",
        format_json_failure_result(command, config_path, report)
    );
}

pub fn print_json_check_success_result(
    command: ConfiguredCommand,
    config_path: Option<&Path>,
    outcome: &app::CheckOutcome,
    report: &core::DiagnosticReport,
) {
    println!(
        "{}",
        format_json_check_success_result(command, config_path, outcome, report)
    );
}

fn format_json_failure_result(
    command: ConfiguredCommand,
    config_path: Option<&Path>,
    report: &core::DiagnosticReport,
) -> String {
    serde_json::to_string(&json!({
        "version": env!("CARGO_PKG_VERSION"),
        "command": {
            "name": command_name(command),
            "options": command_options_json(command, config_path),
        },
        "status": "failure",
        "exitCode": 1,
        "summary": null,
        "diagnostics": diagnostics_json(report),
    }))
    .expect("JSON failure envelope should serialize")
}

fn format_json_check_success_result(
    command: ConfiguredCommand,
    config_path: Option<&Path>,
    outcome: &app::CheckOutcome,
    report: &core::DiagnosticReport,
) -> String {
    serde_json::to_string(&json!({
        "version": env!("CARGO_PKG_VERSION"),
        "command": {
            "name": command_name(command),
            "options": command_options_json(command, config_path),
        },
        "status": "success",
        "exitCode": 0,
        "summary": check_summary_json(outcome),
        "diagnostics": diagnostics_json(report),
    }))
    .expect("JSON check success envelope should serialize")
}

fn check_summary_json(outcome: &app::CheckOutcome) -> Value {
    json!({
        "sourceFileCount": outcome.source_file_count(),
        "builderCount": outcome.builder_count(),
        "queryCount": outcome.query_count(),
        "mutationCount": outcome.mutation_count(),
        "fragmentCount": outcome.fragment_count(),
        "uniqueSlotCount": outcome.unique_slot_count(),
        "uniqueRepeatCount": outcome.unique_repeat_count(),
        "validationCaseCount": outcome.validation_case_count(),
        "outputDir": outcome.output_dir().display().to_string(),
        "queries": outcome.query_summaries().iter().map(query_summary_json).collect::<Vec<_>>(),
        "mutations": outcome
            .mutation_summaries()
            .iter()
            .map(mutation_summary_json)
            .collect::<Vec<_>>(),
    })
}

fn query_summary_json(summary: &app::QuerySummary) -> Value {
    json!({
        "id": summary.id(),
        "sourcePath": source_path_json(summary.source_path()),
        "paramCount": summary.param_count(),
        "inputFieldCount": summary.input_field_count(),
        "slotCount": summary.slot_count(),
        "repeatCount": summary.repeat_count(),
        "validationCaseCount": summary.validation_case_count(),
    })
}

fn mutation_summary_json(summary: &app::MutationSummary) -> Value {
    json!({
        "id": summary.id(),
        "sourcePath": source_path_json(summary.source_path()),
        "kind": super::mutation_kind_name(summary.kind()),
        "paramCount": summary.param_count(),
        "inputFieldCount": summary.input_field_count(),
        "slotCount": summary.slot_count(),
        "repeatCount": summary.repeat_count(),
        "validationCaseCount": summary.validation_case_count(),
    })
}

fn source_path_json(path: Option<&Path>) -> Value {
    path.map_or(Value::Null, |path| json!(path.display().to_string()))
}

const fn command_name(command: ConfiguredCommand) -> &'static str {
    match command {
        ConfiguredCommand::Check { .. } => "check",
        ConfiguredCommand::Compile { .. } => "compile",
    }
}

fn command_options_json(command: ConfiguredCommand, config_path: Option<&Path>) -> Value {
    let mut options = Map::new();
    options.insert(
        "config".to_owned(),
        config_path.map_or(Value::Null, |path| json!(path.display().to_string())),
    );

    match command {
        ConfiguredCommand::Check {
            format,
            fail_on_empty,
        } => {
            options.insert("failOnEmpty".to_owned(), json!(fail_on_empty));
            options.insert("format".to_owned(), json!(format_name(format)));
        }
        ConfiguredCommand::Compile {
            format,
            clean,
            fail_on_empty,
            allow_empty_clean,
        } => {
            options.insert("failOnEmpty".to_owned(), json!(fail_on_empty));
            options.insert("clean".to_owned(), json!(clean));
            options.insert("allowEmptyClean".to_owned(), json!(allow_empty_clean));
            options.insert("format".to_owned(), json!(format_name(format)));
        }
    }

    Value::Object(options)
}

const fn format_name(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Human => "human",
        OutputFormat::Json => "json",
    }
}

fn diagnostics_json(report: &core::DiagnosticReport) -> Value {
    Value::Array(report.diagnostics().iter().map(diagnostic_json).collect())
}

fn diagnostic_json(diagnostic: &core::Diagnostic) -> Value {
    let mut object = Map::new();
    object.insert("severity".to_owned(), json!(diagnostic.severity().as_str()));
    object.insert("message".to_owned(), json!(diagnostic.message()));

    if let Some(location) = diagnostic.location().and_then(location_json) {
        object.insert("location".to_owned(), location);
    }

    Value::Object(object)
}

fn location_json(location: &core::SourceLocation) -> Option<Value> {
    let mut object = Map::new();

    if let Some(path) = location.path() {
        object.insert("path".to_owned(), json!(path.display().to_string()));
    }

    if let Some(range) = location.range() {
        object.insert("range".to_owned(), range_json(range));
    }

    (!object.is_empty()).then_some(Value::Object(object))
}

fn range_json(range: core::SourceRange) -> Value {
    let mut object = Map::new();
    object.insert("start".to_owned(), position_json(range.start()));

    if let Some(end) = range.end() {
        object.insert("end".to_owned(), position_json(end));
    }

    Value::Object(object)
}

fn position_json(position: core::SourcePosition) -> Value {
    json!({
        "line": position.line(),
        "column": position.column(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    #[test]
    fn formats_check_success_json_with_builder_summaries() {
        let diagnostics =
            core::DiagnosticReport::new(core::Diagnostic::warning("unused fragment `activeOnly`"));
        let outcome = app::CheckOutcome::new(
            diagnostics.clone(),
            2,
            PathBuf::from("/tmp/project/src/generated/sqlay"),
            vec![app::QuerySummary::new(
                "filterUsers".to_owned(),
                Some(PathBuf::from("sql/users.sql")),
                app::BuilderSummaryCounts::new(3, 2, 1, 1, 2),
            )],
            vec![app::MutationSummary::new(
                "bulkCreateUsers".to_owned(),
                Some(PathBuf::from("sql/users.sql")),
                core::MutationKind::Insert,
                app::BuilderSummaryCounts::new(2, 1, 0, 1, 1),
            )],
            1,
        );

        let json: Value = serde_json::from_str(&format_json_check_success_result(
            ConfiguredCommand::Check {
                format: OutputFormat::Json,
                fail_on_empty: true,
            },
            Some(Path::new("sqlay.config.json")),
            &outcome,
            &diagnostics,
        ))
        .expect("check success JSON should parse");

        assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(json["command"]["name"], "check");
        assert_eq!(json["command"]["options"]["config"], "sqlay.config.json");
        assert_eq!(json["command"]["options"]["format"], "json");
        assert_eq!(json["command"]["options"]["failOnEmpty"], true);
        assert_eq!(json["status"], "success");
        assert_eq!(json["exitCode"], 0);
        assert_eq!(json["summary"]["sourceFileCount"], 2);
        assert_eq!(json["summary"]["builderCount"], 2);
        assert_eq!(json["summary"]["queryCount"], 1);
        assert_eq!(json["summary"]["mutationCount"], 1);
        assert_eq!(json["summary"]["fragmentCount"], 1);
        assert_eq!(json["summary"]["uniqueSlotCount"], 1);
        assert_eq!(json["summary"]["uniqueRepeatCount"], 2);
        assert_eq!(json["summary"]["validationCaseCount"], 3);
        assert_eq!(
            json["summary"]["outputDir"],
            "/tmp/project/src/generated/sqlay"
        );
        assert_eq!(json["summary"]["queries"][0]["id"], "filterUsers");
        assert_eq!(json["summary"]["queries"][0]["sourcePath"], "sql/users.sql");
        assert_eq!(json["summary"]["queries"][0]["paramCount"], 3);
        assert_eq!(json["summary"]["queries"][0]["inputFieldCount"], 2);
        assert_eq!(json["summary"]["queries"][0]["slotCount"], 1);
        assert_eq!(json["summary"]["queries"][0]["repeatCount"], 1);
        assert_eq!(json["summary"]["queries"][0]["validationCaseCount"], 2);
        assert_eq!(json["summary"]["mutations"][0]["id"], "bulkCreateUsers");
        assert_eq!(
            json["summary"]["mutations"][0]["sourcePath"],
            "sql/users.sql"
        );
        assert_eq!(json["summary"]["mutations"][0]["kind"], "insert");
        assert_eq!(json["summary"]["mutations"][0]["paramCount"], 2);
        assert_eq!(json["summary"]["mutations"][0]["inputFieldCount"], 1);
        assert_eq!(json["summary"]["mutations"][0]["slotCount"], 0);
        assert_eq!(json["summary"]["mutations"][0]["repeatCount"], 1);
        assert_eq!(json["summary"]["mutations"][0]["validationCaseCount"], 1);
        assert_eq!(json["diagnostics"][0]["severity"], "warning");
        assert_eq!(
            json["diagnostics"][0]["message"],
            "unused fragment `activeOnly`"
        );
    }
}
