use sqlay_app::{DialectAnalyzer, MutationAnalyzer};
use sqlay_core as core;

use super::SqliteDialectAnalyzer;

#[test]
fn query_accepts_sqlite_selects_and_infers_cardinality() {
    for (sql, expected) in [
        (
            "SELECT [id] FROM [users] WHERE [name] REGEXP '^A';",
            core::Cardinality::Many,
        ),
        (
            "SELECT id FROM users ORDER BY id LIMIT 1;",
            core::Cardinality::One,
        ),
        (
            "SELECT id FROM users ORDER BY id LIMIT 1 OFFSET 10;",
            core::Cardinality::One,
        ),
        (
            "SELECT (SELECT id FROM users LIMIT 1) AS latest_id FROM accounts;",
            core::Cardinality::Many,
        ),
    ] {
        let analysis = analyze_query(sql)
            .unwrap_or_else(|report| panic!("{sql}: {}", report.diagnostics()[0].message()));

        assert_eq!(analysis.cardinality(), expected, "{sql}");
    }
}

#[test]
fn query_requires_exactly_one_terminated_select_statement() {
    for (sql, expected_message) in [
        (
            "SELECT 1; SELECT 2;",
            "expected exactly one SQL statement per query block; found 2",
        ),
        (
            "",
            "expected exactly one SQL statement per query block; found 0",
        ),
        ("SELECT 1", "query block must end with `;`"),
        (
            "DELETE FROM users WHERE id = 1;",
            "unsupported SQLite SQL statement `DELETE`; supported statement kind is `SELECT`",
        ),
    ] {
        let report = analyze_query(sql).expect_err("unsupported query shape should fail");

        assert_eq!(report.diagnostics()[0].message(), expected_message, "{sql}");
    }
}

#[test]
fn query_rejects_raw_sqlite_placeholders_with_param_guidance() {
    for placeholder in ["?", "?1", ":name", "@name", "$name"] {
        let sql = format!("SELECT id FROM users WHERE name = {placeholder};");
        let report = analyze_query(&sql).expect_err("raw placeholder should fail");

        assert_eq!(
            report.diagnostics()[0].message(),
            "raw SQLite parameter placeholders are not supported in source SQL; use paired `@sqlay` Param markers around a sample expression"
        );
    }
}

#[test]
fn query_accepts_generated_param_placeholder_and_rejects_count_mismatch() {
    let query = raw_query("SELECT id FROM users WHERE name = ?;")
        .with_param_usages(vec![param_usage("name", "'Ada'")]);
    let analysis = SqliteDialectAnalyzer
        .analyze(&query)
        .expect("generated Param placeholder should be accepted");
    assert_eq!(analysis.cardinality(), core::Cardinality::Many);

    let query = raw_query("SELECT id FROM users WHERE name = ? AND id = ?;")
        .with_param_usages(vec![param_usage("name", "'Ada'")]);
    let report = SqliteDialectAnalyzer
        .analyze(&query)
        .expect_err("placeholder count mismatch should fail");
    assert_eq!(
        report.diagnostics()[0].message(),
        "generated placeholder count 2 does not match Param usage count 1"
    );
}

#[test]
fn query_requires_each_param_sample_to_be_one_sqlite_expression() {
    let query = raw_query("SELECT id FROM users WHERE id = ?;")
        .with_param_usages(vec![param_usage("id", "1, 2")]);
    let report = SqliteDialectAnalyzer
        .analyze(&query)
        .expect_err("multiple sample expressions should fail");

    assert_eq!(
        report.diagnostics()[0].message(),
        "Param range must contain exactly one SQL expression"
    );
}

#[test]
fn query_ignores_placeholder_characters_inside_string_literals() {
    let analysis = analyze_query("SELECT '?', ':name', '@name', '$name';")
        .expect("placeholder text inside literals should be accepted");

    assert_eq!(analysis.cardinality(), core::Cardinality::Many);
}

#[test]
fn mutation_accepts_supported_sqlite_statement_forms() {
    for (sql, expected_kind) in [
        (
            "INSERT INTO users (email, name) VALUES (?, ?), (?, ?);",
            core::MutationKind::Insert,
        ),
        (
            "REPLACE INTO users (id, email) VALUES (?, ?);",
            core::MutationKind::Replace,
        ),
        (
            "UPDATE users AS u SET name = ? WHERE u.id = ?;",
            core::MutationKind::Update,
        ),
        (
            "DELETE FROM users AS u WHERE u.id = ?;",
            core::MutationKind::Delete,
        ),
    ] {
        let mutation = raw_mutation(sql).with_param_usages(
            (0..sql.matches('?').count())
                .map(|index| param_usage(&format!("value{index}"), "'value'"))
                .collect(),
        );
        let analysis = SqliteDialectAnalyzer
            .analyze_mutation(&mutation)
            .unwrap_or_else(|report| panic!("{sql}: {}", report.diagnostics()[0].message()));

        assert_eq!(analysis.kind(), expected_kind, "{sql}");
    }
}

#[test]
fn mutation_requires_exactly_one_terminated_statement() {
    for (sql, expected_message) in [
        (
            "INSERT INTO users (id) VALUES (1); DELETE FROM users WHERE id = 1;",
            "expected exactly one SQL statement per mutation block; found 2",
        ),
        (
            "",
            "expected exactly one SQL statement per mutation block; found 0",
        ),
        (
            "INSERT INTO users (id) VALUES (1)",
            "mutation block must end with `;`",
        ),
    ] {
        let report = analyze_mutation(sql).expect_err("unsupported mutation shape should fail");

        assert_eq!(report.diagnostics()[0].message(), expected_message, "{sql}");
    }
}

#[test]
fn mutation_rejects_unsupported_insert_and_replace_forms_actionably() {
    for (sql, expected_message) in [
        (
            "INSERT INTO archived_users (id) SELECT id FROM users;",
            "unsupported SQLite INSERT ... SELECT; supported form is `INSERT ... VALUES`",
        ),
        (
            "INSERT INTO users SET name = 'Ada';",
            "unsupported SQLite INSERT ... SET; supported form is `INSERT ... VALUES`",
        ),
        (
            "INSERT INTO users (id, name) VALUES (1, 'Ada') ON CONFLICT(id) DO UPDATE SET name = excluded.name;",
            "unsupported SQLite INSERT upsert; `ON CONFLICT` is outside the supported mutation scope",
        ),
        (
            "INSERT INTO users (id) VALUES (1) RETURNING id;",
            "unsupported SQLite mutation `RETURNING`; mutation builders do not return rows",
        ),
        (
            "REPLACE INTO archived_users (id) SELECT id FROM users;",
            "unsupported SQLite REPLACE ... SELECT; supported form is `REPLACE ... VALUES`",
        ),
    ] {
        let report = analyze_mutation(sql).expect_err("unsupported mutation should fail");

        assert_eq!(report.diagnostics()[0].message(), expected_message, "{sql}");
    }
}

#[test]
fn mutation_rejects_cte_update_from_and_multi_table_forms_actionably() {
    for (sql, expected_message) in [
        (
            "WITH stale AS (SELECT id FROM users) UPDATE users SET active = 0 WHERE id IN (SELECT id FROM stale);",
            "unsupported SQLite CTE mutation; `WITH ... INSERT/UPDATE/DELETE/REPLACE` is outside the supported mutation scope",
        ),
        (
            "UPDATE users SET name = accounts.name FROM accounts WHERE users.id = accounts.user_id;",
            "unsupported SQLite `UPDATE ... FROM`; supported form is single-table `UPDATE ... WHERE`",
        ),
        (
            "UPDATE users JOIN accounts ON accounts.user_id = users.id SET name = accounts.name WHERE users.id = 1;",
            "unsupported multi-table SQLite UPDATE; supported form is single-table `UPDATE ... WHERE`",
        ),
        (
            "DELETE FROM users USING accounts WHERE accounts.user_id = users.id;",
            "unsupported multi-table SQLite DELETE; supported form is single-table `DELETE ... WHERE`",
        ),
    ] {
        let report = analyze_mutation(sql).expect_err("unsupported mutation should fail");

        assert_eq!(report.diagnostics()[0].message(), expected_message, "{sql}");
    }
}

#[test]
fn mutation_requires_where_for_update_and_delete() {
    for (sql, expected_message) in [
        (
            "UPDATE users SET active = 0;",
            "SQLite UPDATE mutation requires a WHERE clause",
        ),
        (
            "DELETE FROM users;",
            "SQLite DELETE mutation requires a WHERE clause",
        ),
    ] {
        let report = analyze_mutation(sql).expect_err("full-table mutation should fail");

        assert_eq!(report.diagnostics()[0].message(), expected_message, "{sql}");
    }
}

#[test]
fn mutation_validates_placeholders_and_param_sample_expressions() {
    let report = analyze_mutation("UPDATE users SET name = :name WHERE id = 1;")
        .expect_err("raw placeholder should fail");
    assert_eq!(
        report.diagnostics()[0].message(),
        "raw SQLite parameter placeholders are not supported in source SQL; use paired `@sqlay` Param markers around a sample expression"
    );

    let mutation = raw_mutation("UPDATE users SET name = ? WHERE id = 1;")
        .with_param_usages(vec![param_usage("name", "'Ada', 'Grace'")]);
    let report = SqliteDialectAnalyzer
        .analyze_mutation(&mutation)
        .expect_err("multiple sample expressions should fail");
    assert_eq!(
        report.diagnostics()[0].message(),
        "Param range must contain exactly one SQL expression"
    );
}

fn analyze_query(sql: &str) -> core::DiagnosticResult<core::AnalyzedQuery> {
    SqliteDialectAnalyzer.analyze(&raw_query(sql))
}

fn raw_query(sql: &str) -> core::RawQuery {
    core::RawQuery::new(
        core::QueryMetadata::new("testQuery".to_owned(), None),
        sql.to_owned(),
    )
}

fn analyze_mutation(sql: &str) -> core::DiagnosticResult<core::AnalyzedMutation> {
    SqliteDialectAnalyzer.analyze_mutation(&raw_mutation(sql))
}

fn raw_mutation(sql: &str) -> core::RawMutation {
    core::RawMutation::new(
        core::MutationMetadata::new("testMutation".to_owned()),
        sql.to_owned(),
    )
}

fn param_usage(id: &str, sample_sql: &str) -> core::ParamUsage {
    core::ParamUsage::new(
        id.to_owned(),
        Some(core::CoreType::String),
        false,
        core::SourceLocation::unknown(),
    )
    .with_sample_sql(sample_sql.to_owned())
}
