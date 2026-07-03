use sqlay_core as core;

use crate::compile::param_validation::{
    validate_expanded_mutation_variant_param_bindings, validate_expanded_variant_param_bindings,
};
use crate::compile::slot_variants::{
    AnalyzedMutationVariant, AnalyzedQueryVariant, ExpandedParamOccurrence, ExpandedParamScope,
    ExpandedRepeatParamOccurrence,
};

#[test]
fn query_repeat_param_validation_rejects_conflicting_schema_column_references() {
    let repeat_location = core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(4, 12).expect("test position should be valid"),
    );
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        "SELECT id FROM users WHERE id IN (?,?);".to_owned(),
    )
    .with_param_usages(vec![
        test_param_usage("id", 34, false),
        test_param_usage("id", 36, false),
    ]);
    let variant = AnalyzedQueryVariant {
        query,
        analysis: core::AnalyzedQuery::new(core::Cardinality::Many),
        context: None,
        param_scopes: repeat_param_scopes("ids"),
        param_occurrences: repeat_param_occurrences("ids", repeat_location),
    };
    let metadata = core::DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
        core::DbParamUsage::new("id".to_owned(), core::CoreType::Int64)
            .with_schema_column_reference(column_ref(None, "users", "id")),
        core::DbParamUsage::new("id".to_owned(), core::CoreType::Int64)
            .with_schema_column_reference(column_ref(None, "orders", "user_id")),
    ]);
    let mut scoped_param_bindings = Vec::new();

    let report =
        validate_expanded_variant_param_bindings(&variant, &metadata, &mut scoped_param_bindings)
            .expect_err("Repeat item Param schema column reference conflicts should be rejected");

    super::assert_diagnostic_messages(
        &report,
        "conflicting Repeat item Param `id` schema column reference in query `findUsers`, Repeat `ids`: first representative occurrence resolved from users.id but conflicting representative occurrence resolved from orders.user_id; Repeat item fields with the same ID must resolve matching Param type, nullability, and schema column reference\nfirst Repeat `ids` occurrence is here\nconflicting Repeat `ids` occurrence is here",
    );
}

#[test]
fn mutation_repeat_param_validation_rejects_conflicting_schema_column_references() {
    let repeat_location = core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(5, 1).expect("test position should be valid"),
    );
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("createUsers".to_owned()),
        "INSERT INTO users (email) VALUES (?),(?);".to_owned(),
    )
    .with_param_usages(vec![
        test_param_usage("email", 34, false),
        test_param_usage("email", 38, false),
    ]);
    let variant = AnalyzedMutationVariant {
        mutation,
        analysis: core::AnalyzedMutation::new(core::MutationKind::Insert),
        context: None,
        param_scopes: repeat_param_scopes("rows"),
        param_occurrences: repeat_param_occurrences("rows", repeat_location),
    };
    let metadata = core::DbMutationMetadata::new().with_param_usages(vec![
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String)
            .with_schema_column_reference(column_ref(None, "users", "email")),
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String)
            .with_schema_column_reference(column_ref(None, "profiles", "email")),
    ]);
    let mut scoped_param_bindings = Vec::new();

    let report = validate_expanded_mutation_variant_param_bindings(
        &variant,
        &metadata,
        &mut scoped_param_bindings,
    )
    .expect_err("Repeat item Param schema column reference conflicts should be rejected");

    super::assert_diagnostic_messages(
        &report,
        "conflicting Repeat item Param `email` schema column reference in mutation `createUsers`, Repeat `rows`: first representative occurrence resolved from users.email but conflicting representative occurrence resolved from profiles.email; Repeat item fields with the same ID must resolve matching Param type, nullability, and schema column reference\nfirst Repeat `rows` occurrence is here\nconflicting Repeat `rows` occurrence is here",
    );
}

fn repeat_param_scopes(repeat_id: &str) -> Vec<ExpandedParamScope> {
    vec![
        ExpandedParamScope::RepeatItem {
            repeat_id: repeat_id.to_owned(),
        },
        ExpandedParamScope::RepeatItem {
            repeat_id: repeat_id.to_owned(),
        },
    ]
}

fn repeat_param_occurrences(
    repeat_id: &str,
    repeat_location: core::SourceLocation,
) -> Vec<ExpandedParamOccurrence> {
    vec![
        ExpandedParamOccurrence::RepeatItem(ExpandedRepeatParamOccurrence {
            repeat_id: repeat_id.to_owned(),
            representative_item_index: 1,
            repeat_location: repeat_location.clone(),
        }),
        ExpandedParamOccurrence::RepeatItem(ExpandedRepeatParamOccurrence {
            repeat_id: repeat_id.to_owned(),
            representative_item_index: 2,
            repeat_location,
        }),
    ]
}

fn column_ref(database: Option<&str>, table: &str, column: &str) -> core::ColumnTypeReference {
    core::ColumnTypeReference::new(
        database.map(str::to_owned),
        table.to_owned(),
        column.to_owned(),
    )
}

fn test_param_usage(id: &str, placeholder_index: usize, nullable: bool) -> core::ParamUsage {
    core::ParamUsage::new(
        id.to_owned(),
        None,
        nullable,
        core::SourceLocation::unknown(),
    )
    .with_placeholder_index(placeholder_index)
}
