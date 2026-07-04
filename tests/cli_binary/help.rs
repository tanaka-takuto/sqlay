use std::process::Command;

fn assert_optional_filter_guidance(stdout: &str) {
    assert!(
        stdout.contains("Optional filter Slot/Fragment example:"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("type: fragment"), "stdout: {stdout}");
    assert!(stdout.contains("id: byStatus"), "stdout: {stdout}");
    assert!(
        stdout.contains(
            "AND orders.status = /* @sqlay { type: param id: status } */ 'paid' /* @sqlay { type: paramEnd } */"
        ),
        "stdout should keep status Param range on one line: {stdout}"
    );
    assert!(stdout.contains("id: listOrders"), "stdout: {stdout}");
    assert!(
        stdout.contains("type: slot id: statusFilter targets: [byStatus]"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("statusFilter?: {"), "stdout: {stdout}");
    assert!(
        stdout.contains("$fragment: \"byStatus\";"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("status: string;"), "stdout: {stdout}");
    assert!(
        stdout.contains(
            "For optional filters that change whether a predicate exists, prefer Slot/Fragment composition over nullable sentinel predicates"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "Use nullable: true for values that are semantically nullable in a stable SQL shape"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("valueType values: bool, int32, int64, float64, decimal, string, bytes, date, time, datetime, json"),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains("Repeat the same Param id for optional filters"),
        "stdout should not recommend repeated nullable Params as the optional-filter default: {stdout}"
    );
    assert!(
        !stdout.contains("listCustomersByFilter"),
        "stdout should not keep the old nullable-sentinel optional filter example: {stdout}"
    );
    assert!(
        !stdout.contains("emailFilter"),
        "stdout should not keep the old nullable-sentinel optional filter example: {stdout}"
    );
    assert!(
        !stdout.contains("createdBefore"),
        "stdout should not keep the old nullable-sentinel optional filter example: {stdout}"
    );
    assert!(
        !stdout.contains("active: boolean | null;"),
        "stdout should not keep the old nullable-sentinel optional filter example: {stdout}"
    );
}

fn assert_param_marker_guidance(stdout: &str) {
    assert_optional_filter_guidance(stdout);
    assert!(
        stdout.contains(
            "Use nullable: true for T | null inputs; optional input properties are not supported"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Repeated Param ids share one generated input field"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Each marker occurrence appends one params entry in source order"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "All occurrences of a repeated Param id must use the same valueType and nullability"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("For bool Params, use TRUE or FALSE as the sample expression"),
        "stdout: {stdout}"
    );
}

fn assert_mutation_guidance(stdout: &str) {
    assert!(
        stdout.contains("Minimal mutation annotation:"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("type: mutation"), "stdout: {stdout}");
    assert!(stdout.contains("id: createUser"), "stdout: {stdout}");
    assert!(
        stdout.contains("INSERT INTO users (email, name)"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("/* @sqlay { type: param id: email } */ 'ada@example.test' /* @sqlay { type: paramEnd } */"),
        "stdout should show a compact mutation Param range: {stdout}"
    );
    assert!(
        stdout.contains("supports INSERT, UPDATE, DELETE, and REPLACE builders"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("check and compile validate mutation SQL and infer input Params, but never execute mutation statements"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Mutation builders return { sql, params } only"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("affectedRows, insertId, changedRows, transactions, upserts, and REPLACE result interpretation belong to application/driver code"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("docs/mutation-execution.md"),
        "stdout: {stdout}"
    );
}

#[test]
fn no_args_prints_top_level_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .output()
        .expect("sqlay binary should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(
        stdout.contains("sqlay <command> [options]"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("sqlay init"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay check"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay compile"), "stdout: {stdout}");
    assert!(stdout.contains("/* @sqlay"), "stdout: {stdout}");
    assert!(stdout.contains("type: query"), "stdout: {stdout}");
    assert!(stdout.contains("id: listUsers"), "stdout: {stdout}");
    assert!(
        stdout.contains("cardinality: one | many"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("ordinary SQL comments"), "stdout: {stdout}");
    assert!(stdout.contains("raw `?` placeholders"), "stdout: {stdout}");
    assert!(stdout.contains("--format <human|json>"), "stdout: {stdout}");
    assert!(
        stdout.contains("JSON prints a stdout result envelope"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("source.include paths must stay inside the config directory"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("--fail-on-empty"), "stdout: {stdout}");
    assert!(
        stdout.contains(
            "Place sqlay.config.json at the project root when SQL lives in sibling directories"
        ),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("Query metadata:"), "stdout: {stdout}");
    assert!(
        stdout.contains("use paired @sqlay Param markers around a sample expression"),
        "stdout: {stdout}"
    );
    assert_param_marker_guidance(&stdout);
    assert_mutation_guidance(&stdout);
    assert!(
        !stdout.contains("MVP query metadata"),
        "stdout should not describe current help as MVP-only: {stdout}"
    );
    assert!(
        !stdout.contains("when dynamic inputs are supported"),
        "stdout should not describe Param markers as future-only: {stdout}"
    );
}

#[test]
fn help_lists_supported_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("--help")
        .output()
        .expect("sqlay help should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sqlay init"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay check"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay compile"), "stdout: {stdout}");
}

#[test]
fn init_help_describes_init_behavior() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["init", "--help"])
        .output()
        .expect("sqlay init help should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay init"), "stdout: {stdout}");
    assert!(
        stdout.contains("starter sqlay.config.json"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("refuses to overwrite"), "stdout: {stdout}");
}

#[test]
fn check_help_describes_config_discovery_and_database_url() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["check", "--help"])
        .output()
        .expect("sqlay check help should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay check"), "stdout: {stdout}");
    assert!(stdout.contains("--config <path>"), "stdout: {stdout}");
    assert!(stdout.contains("--format <human|json>"), "stdout: {stdout}");
    assert!(
        stdout.contains(
            "JSON prints a stdout result envelope with diagnostics and the check summary"
        ),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains("JSON rendering is not yet available in this slice"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("--fail-on-empty"), "stdout: {stdout}");
    assert!(
        stdout.contains("searches from the current working directory upward"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("database.urlEnv"), "stdout: {stdout}");
    assert!(stdout.contains("No files are written"), "stdout: {stdout}");
    assert!(
        stdout.contains("sqlay check --format json"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("preserves each input SQL path"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("source.include paths must stay inside the config directory"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "Place sqlay.config.json at the project root when SQL lives in sibling directories"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Fragment, Slot, Repeat, validation case counts"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("compiled builders with query and mutation counts"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("per-query/per-mutation Param, Slot, Repeat, and validation case counts"),
        "stdout: {stdout}"
    );
    assert_param_marker_guidance(&stdout);
    assert_mutation_guidance(&stdout);
}

#[test]
fn compile_help_describes_output_writing_and_clean() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["compile", "--help"])
        .output()
        .expect("sqlay compile help should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay compile"), "stdout: {stdout}");
    assert!(stdout.contains("--config <path>"), "stdout: {stdout}");
    assert!(stdout.contains("--format <human|json>"), "stdout: {stdout}");
    assert!(
        stdout.contains(
            "JSON prints a stdout result envelope with diagnostics and the compile summary"
        ),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains("compile success JSON is not yet available"),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains("JSON rendering is not yet available in this slice"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("--fail-on-empty"), "stdout: {stdout}");
    assert!(stdout.contains("--allow-empty-clean"), "stdout: {stdout}");
    assert!(
        stdout.contains("generated TypeScript files"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("--clean"), "stdout: {stdout}");
    assert!(stdout.contains("stale generated files"), "stdout: {stdout}");
    assert!(
        stdout.contains("sqlay compile --format json"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("skips stale generated file cleanup when no SQL files match"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("preserves each input SQL path"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("source.include paths must stay inside the config directory"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "Place sqlay.config.json at the project root when SQL lives in sibling directories"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Fragment, Slot, Repeat, validation case counts"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("compiled builders with query and mutation counts"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("per-query/per-mutation Param, Slot, Repeat, and validation case counts"),
        "stdout: {stdout}"
    );
    assert_param_marker_guidance(&stdout);
    assert_mutation_guidance(&stdout);
    assert!(
        stdout.contains("BIGINT, DECIMAL, date/time, and enum values map conservatively"),
        "stdout: {stdout}"
    );
}
