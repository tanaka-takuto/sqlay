use sqlay_adapters::config_jsonc::JsoncConfigLoader;
use sqlay_adapters::dialect_mysql::MysqlDialectAnalyzer;
use sqlay_adapters::metadata::mysql::sqlx::SqlxMysqlMetadataProvider;
use sqlay_adapters::output_fs::FileSystemGeneratedFileWriter;
use sqlay_adapters::source_fs::FileSystemSourceReader;
use sqlay_adapters::target::typescript::TypeScriptTargetGenerator;
use sqlay_app::{
    CompilePipeline, ConfigLoader, DefaultCompilationPlanner, DefaultCompileUseCase,
    DefaultQueryCompiler,
};
use sqlx::{Connection, MySqlConnection};

use super::fixture_support::{
    DATABASE_URL_ENV, FRAGMENT_PARAM_INFERENCE_FAILURE, INIT_FIXTURES,
    MUTATION_UNSUPPORTED_INFERENCE_CONTEXT, MYSQL_FIXTURE_LOCK,
    PARAM_CONFLICTING_REPEATED_NULLABILITY, PARAM_CONFLICTING_REPEATED_TYPE,
    PARAM_UNSUPPORTED_INFERENCE_CONTEXT, REPEAT_PARAM_INFERENCE_FAILURE,
    REPEATED_REPEAT_ITEM_INFERRED_TYPE_CONFLICT, REPEATED_SLOT_FRAGMENT_PARAM_TYPE_CONFLICT,
    SLOT_VARIANT_ROW_SHAPE_MISMATCH, TYPE_MAPPING_INVALID_USAGE_CONFIG, TYPE_MAPPING_OVERRIDES,
    assert_mysql_invalid_fixture_error_contains, execute_fixture_statements, unique_temp_dir,
};

#[test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn mysql_param_invalid_fixtures_report_expected_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let _fixture_lock = MYSQL_FIXTURE_LOCK
        .lock()
        .expect("fixture lock should not be poisoned");
    let database_url = std::env::var(DATABASE_URL_ENV)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut connection = runtime.block_on(MySqlConnection::connect(&database_url))?;

    for fixture in INIT_FIXTURES {
        runtime.block_on(execute_fixture_statements(&mut connection, fixture))?;
    }

    let cases = [
        (
            "param_unsupported_inference_context.sql",
            PARAM_UNSUPPORTED_INFERENCE_CONTEXT,
            "Param `lowerVarchar` requires `valueType` because no supported qualified column context was found",
        ),
        (
            "param_conflicting_repeated_type.sql",
            PARAM_CONFLICTING_REPEATED_TYPE,
            "conflicting Param `sameValue` types: first occurrence resolved to Int64 but later occurrence resolved to String",
        ),
        (
            "param_conflicting_repeated_nullability.sql",
            PARAM_CONFLICTING_REPEATED_NULLABILITY,
            "conflicting Param `sameText` nullability: first occurrence is nullable false but later occurrence is nullable true",
        ),
    ];

    for (file_name, source, expected) in cases {
        assert_mysql_invalid_fixture_error_contains(&database_url, file_name, source, expected)?;
    }

    Ok(())
}

#[test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn mysql_slot_fragment_invalid_fixtures_report_expected_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let _fixture_lock = MYSQL_FIXTURE_LOCK
        .lock()
        .expect("fixture lock should not be poisoned");
    let database_url = std::env::var(DATABASE_URL_ENV)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut connection = runtime.block_on(MySqlConnection::connect(&database_url))?;

    for fixture in INIT_FIXTURES {
        runtime.block_on(execute_fixture_statements(&mut connection, fixture))?;
    }

    let cases = [
        (
            "fragment_param_inference_failure.sql",
            FRAGMENT_PARAM_INFERENCE_FAILURE,
            "Param `lowerText` requires `valueType` because no supported qualified column context was found",
        ),
        (
            "repeated_slot_fragment_param_type_conflict.sql",
            REPEATED_SLOT_FRAGMENT_PARAM_TYPE_CONFLICT,
            "conflicting Fragment Param `value` type in query `repeatedSlotFragmentParamTypeConflict`, Slot `comparator`, Fragment `equalsValue`",
        ),
        (
            "slot_variant_row_shape_mismatch.sql",
            SLOT_VARIANT_ROW_SHAPE_MISMATCH,
            "Slot expansion variant for query `slotVariantRowShapeMismatch` returned 2 result columns, but the base variant returned 1",
        ),
    ];

    for (file_name, source, expected) in cases {
        assert_mysql_invalid_fixture_error_contains(&database_url, file_name, source, expected)?;
    }

    assert_mysql_invalid_fixture_error_contains(
        &database_url,
        "fragment_param_inference_failure.sql",
        FRAGMENT_PARAM_INFERENCE_FAILURE,
        "while validating Slot expansion variant for query `fragmentParamInferenceFailure` with selections: filter=lowerTextFilter",
    )?;

    Ok(())
}

#[test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn mysql_mutation_invalid_fixtures_report_expected_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let _fixture_lock = MYSQL_FIXTURE_LOCK
        .lock()
        .expect("fixture lock should not be poisoned");
    let database_url = std::env::var(DATABASE_URL_ENV)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut connection = runtime.block_on(MySqlConnection::connect(&database_url))?;

    for fixture in INIT_FIXTURES {
        runtime.block_on(execute_fixture_statements(&mut connection, fixture))?;
    }

    assert_mysql_invalid_fixture_error_contains(
        &database_url,
        "mutation_unsupported_inference_context.sql",
        MUTATION_UNSUPPORTED_INFERENCE_CONTEXT,
        "Param `adjustment` requires `valueType` because no supported mutation column context was found",
    )?;

    Ok(())
}

#[test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn mysql_repeat_invalid_fixtures_report_expected_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let _fixture_lock = MYSQL_FIXTURE_LOCK
        .lock()
        .expect("fixture lock should not be poisoned");
    let database_url = std::env::var(DATABASE_URL_ENV)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut connection = runtime.block_on(MySqlConnection::connect(&database_url))?;

    for fixture in INIT_FIXTURES {
        runtime.block_on(execute_fixture_statements(&mut connection, fixture))?;
    }

    let cases = [
        (
            "repeat_param_inference_failure.sql",
            REPEAT_PARAM_INFERENCE_FAILURE,
            "Param `value` requires `valueType` because no supported qualified column context was found",
        ),
        (
            "repeated_repeat_item_inferred_type_conflict.sql",
            REPEATED_REPEAT_ITEM_INFERRED_TYPE_CONFLICT,
            "conflicting Repeat item Param `value` type in query `repeatedRepeatItemInferredTypeConflict`, Repeat `values`",
        ),
    ];

    for (file_name, source, expected) in cases {
        assert_mysql_invalid_fixture_error_contains(&database_url, file_name, source, expected)?;
    }

    Ok(())
}

#[test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn mysql_type_mapping_invalid_usage_fixture_reports_expected_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let _fixture_lock = MYSQL_FIXTURE_LOCK
        .lock()
        .expect("fixture lock should not be poisoned");
    let database_url = std::env::var(DATABASE_URL_ENV)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut connection = runtime.block_on(MySqlConnection::connect(&database_url))?;

    for fixture in INIT_FIXTURES {
        runtime.block_on(execute_fixture_statements(&mut connection, fixture))?;
    }

    let project_dir = unique_temp_dir("sqlay-type-mapping-invalid-usage-fixture");
    let valid_dir = project_dir.join("valid");
    std::fs::create_dir_all(&valid_dir)?;
    std::fs::write(
        valid_dir.join("type_mapping_overrides.sql"),
        TYPE_MAPPING_OVERRIDES,
    )?;
    std::fs::write(
        project_dir.join("sqlay.config.json"),
        TYPE_MAPPING_INVALID_USAGE_CONFIG,
    )?;

    let config = JsoncConfigLoader::new(project_dir.join("sqlay.config.json")).load()?;
    let metadata_provider = SqlxMysqlMetadataProvider::new(database_url);
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &FileSystemSourceReader,
        dialect_analyzer: &MysqlDialectAnalyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &DefaultQueryCompiler,
        target_generator: &TypeScriptTargetGenerator,
        generated_file_writer: &FileSystemGeneratedFileWriter,
    };
    let report = DefaultCompileUseCase::check(&config, &pipeline)
        .expect_err("invalid type mapping usage fixture should fail generation");
    let messages = report
        .diagnostics()
        .iter()
        .map(sqlay_core::Diagnostic::message)
        .collect::<Vec<_>>()
        .join("\n");

    for expected in [
        "unknown TypeScript type mapping builder override `builders.missingBuilder`; no generated builder with that id exists",
        "unused TypeScript type mapping override `builders.typeMappingOverrides.fields.missingField`; no result field with that name exists on builder `typeMappingOverrides`",
        "unused TypeScript type mapping override `builders.typeMappingOverrides.params.missingParam`; no direct Param input with that name exists on builder `typeMappingOverrides`",
        "unused TypeScript type mapping override `builders.typeMappingOverrides.repeats.missingRows`; no direct Repeat input with that id exists on builder `typeMappingOverrides`",
        "unused TypeScript type mapping override `columns.fixture_all_column_type.missing_col`; no generated field, Param, or Repeat item resolved to that schema column",
        "TypeScript type import conflict in source file",
        "local type `ConflictingType` is imported from both `@fixtures/conflict-a` and `@fixtures/conflict-b`",
    ] {
        assert!(
            messages.contains(expected),
            "expected diagnostic containing `{expected}`, got:\n{messages}"
        );
    }

    std::fs::remove_dir_all(project_dir)?;

    Ok(())
}
