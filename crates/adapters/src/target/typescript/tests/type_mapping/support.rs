use sqlay_core as core;

pub(super) fn compilation_plan_with_mapping(
    type_mapping: core::TypeScriptTypeMappingConfig,
) -> core::CompilationPlan {
    core::CompilationPlan::new(
        "/tmp/sqlay-project".into(),
        vec!["/tmp/sqlay-project/sql/**/*.sql".into()],
        Vec::new(),
        "/tmp/sqlay-project/src/generated/sqlay".into(),
        core::DatabaseConfig::new(core::DatabaseDialect::MySql, "DATABASE_URL".to_owned()),
        core::TargetConfig::new(core::TargetLanguage::TypeScript)
            .with_typescript_type_mapping(type_mapping),
    )
}

pub(super) fn column_ref(
    database: Option<&str>,
    table: &str,
    column: &str,
) -> core::ColumnTypeReference {
    core::ColumnTypeReference::new(
        database.map(str::to_owned),
        table.to_owned(),
        column.to_owned(),
    )
}

pub(super) fn core_type_override(
    core_type: core::CoreType,
    type_name: &str,
    import: Option<core::TypeScriptTypeImport>,
) -> core::CoreTypeOverride {
    core::CoreTypeOverride::new(core_type, type_override(type_name, import))
}

pub(super) fn column_override(
    reference: core::ColumnTypeReference,
    type_name: &str,
    import: Option<core::TypeScriptTypeImport>,
) -> core::ColumnTypeOverride {
    core::ColumnTypeOverride::new(reference, type_override(type_name, import))
}

pub(super) fn named_override(
    name: &str,
    type_name: &str,
    import: Option<core::TypeScriptTypeImport>,
) -> core::NamedTypeOverride {
    core::NamedTypeOverride::new(name.to_owned(), type_override(type_name, import))
}

fn type_override(
    type_name: &str,
    import: Option<core::TypeScriptTypeImport>,
) -> core::TypeScriptTypeOverride {
    core::TypeScriptTypeOverride::new(type_name.to_owned(), import)
}

pub(super) fn import(from: &str, name: &str) -> core::TypeScriptTypeImport {
    core::TypeScriptTypeImport::new(from.to_owned(), name.to_owned())
}

pub(super) fn enum_type_ref(values: impl IntoIterator<Item = &'static str>) -> core::CoreTypeRef {
    core::CoreTypeRef::from_enum_values(values.into_iter().map(str::to_owned).collect())
        .expect("test enum values should build a Core type reference")
}

pub(super) fn diagnostic_messages(report: &core::DiagnosticReport) -> Vec<&str> {
    report
        .diagnostics()
        .iter()
        .map(core::Diagnostic::message)
        .collect()
}

pub(super) fn assert_message(messages: &[&str], expected: &str) {
    assert!(
        messages.contains(&expected),
        "expected diagnostic `{expected}`, got {messages:?}"
    );
}
