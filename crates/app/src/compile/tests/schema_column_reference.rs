use sqlay_core as core;

use crate::compile::param_validation::{
    validate_expanded_mutation_variant_param_bindings, validate_expanded_variant_param_bindings,
};
use crate::compile::slot_variants::{
    AnalyzedMutationVariant, AnalyzedQueryVariant, ExpandedFragmentParamOccurrence,
    ExpandedFragmentRepeatParamOccurrence, ExpandedParamOccurrence, ExpandedParamScope,
    ExpandedRepeatParamOccurrence,
};

#[test]
fn query_direct_param_validation_rejects_conflicting_schema_column_references() {
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        "SELECT id FROM users WHERE email = ? OR contact_email = ?;".to_owned(),
    )
    .with_param_usages(vec![
        test_param_usage("email", 35, false),
        test_param_usage("email", 56, false),
    ]);
    let variant = AnalyzedQueryVariant {
        query,
        analysis: core::AnalyzedQuery::new(core::Cardinality::Many),
        context: None,
        param_scopes: direct_param_scopes(),
        param_occurrences: direct_param_occurrences(),
    };
    let metadata = core::DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String)
            .with_schema_column_reference(column_ref(None, "users", "email")),
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
    ]);
    let mut scoped_param_bindings = Vec::new();

    let report =
        validate_expanded_variant_param_bindings(&variant, &metadata, &mut scoped_param_bindings)
            .expect_err("direct Param schema column reference conflicts should be rejected");

    super::assert_diagnostic_messages(
        &report,
        "conflicting Param `email` schema column references: first occurrence resolved from users.email but later occurrence resolved from no schema column reference",
    );
}

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
fn query_fragment_repeat_param_validation_rejects_conflicting_schema_column_references() {
    let repeat_location = core::SourceLocation::at_position(
        "sql/fragments.sql",
        core::SourcePosition::one_based(3, 14).expect("test position should be valid"),
    );
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
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
        param_scopes: fragment_repeat_param_scopes("filter", "byIds", "ids"),
        param_occurrences: fragment_repeat_param_occurrences(
            "filter",
            "byIds",
            "ids",
            repeat_location,
        ),
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
            .expect_err(
                "Fragment Repeat item Param schema column reference conflicts should be rejected",
            );

    super::assert_diagnostic_messages(
        &report,
        "conflicting Fragment Repeat item Param `id` schema column reference in query `listUsers`, Slot `filter`, Fragment `byIds`, Repeat `ids`: first representative occurrence resolved from users.id but conflicting representative occurrence resolved from orders.user_id; Repeat item fields with the same ID must resolve matching Param type, nullability, and schema column reference\nfirst Repeat `ids` occurrence in Slot `filter` selecting Fragment `byIds` is here\nconflicting Repeat `ids` occurrence in Slot `filter` selecting Fragment `byIds` is here",
    );
}

#[test]
fn repeated_query_fragment_param_validation_rejects_conflicting_schema_column_references() {
    let first_slot_location = core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(8, 88).expect("test position should be valid"),
    );
    let second_slot_location = core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(9, 96).expect("test position should be valid"),
    );
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT id FROM users WHERE kind = ? OR kind = ?;".to_owned(),
    )
    .with_param_usages(vec![
        test_param_usage("kind", 35, false),
        test_param_usage("kind", 47, false),
    ]);
    let variant = AnalyzedQueryVariant {
        query,
        analysis: core::AnalyzedQuery::new(core::Cardinality::Many),
        context: None,
        param_scopes: fragment_param_scopes("filter", "byKind"),
        param_occurrences: fragment_param_occurrences(
            "filter",
            "byKind",
            first_slot_location,
            second_slot_location,
        ),
    };
    let metadata = core::DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
        core::DbParamUsage::new("kind".to_owned(), core::CoreType::String)
            .with_schema_column_reference(column_ref(None, "users", "kind")),
        core::DbParamUsage::new("kind".to_owned(), core::CoreType::String)
            .with_schema_column_reference(column_ref(None, "profiles", "kind")),
    ]);
    let mut scoped_param_bindings = Vec::new();

    let report =
        validate_expanded_variant_param_bindings(&variant, &metadata, &mut scoped_param_bindings)
            .expect_err(
                "repeated Fragment Param schema column reference conflicts should be rejected",
            );

    super::assert_diagnostic_messages(
        &report,
        "conflicting Fragment Param `kind` schema column reference in query `listUsers`, Slot `filter`, Fragment `byKind`: occurrence 1 resolved from users.kind but occurrence 2 resolved from profiles.kind; repeated Slot occurrences that select the same Fragment must resolve matching Param type, nullability, and schema column reference\nfirst occurrence of Slot `filter` selecting Fragment `byKind` is here\nconflicting occurrence of Slot `filter` selecting Fragment `byKind` is here",
    );
}

#[test]
fn mutation_direct_param_validation_rejects_conflicting_schema_column_references() {
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("renameUser".to_owned()),
        "UPDATE users SET email = ? WHERE contact_email = ?;".to_owned(),
    )
    .with_param_usages(vec![
        test_param_usage("email", 25, false),
        test_param_usage("email", 47, false),
    ]);
    let variant = AnalyzedMutationVariant {
        mutation,
        analysis: core::AnalyzedMutation::new(core::MutationKind::Update),
        context: None,
        param_scopes: direct_param_scopes(),
        param_occurrences: direct_param_occurrences(),
    };
    let metadata = core::DbMutationMetadata::new().with_param_usages(vec![
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String)
            .with_schema_column_reference(column_ref(Some("app"), "users", "email")),
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String)
            .with_schema_column_reference(column_ref(None, "contacts", "email")),
    ]);
    let mut scoped_param_bindings = Vec::new();

    let report = validate_expanded_mutation_variant_param_bindings(
        &variant,
        &metadata,
        &mut scoped_param_bindings,
    )
    .expect_err("direct mutation Param schema column reference conflicts should be rejected");

    super::assert_diagnostic_messages(
        &report,
        "conflicting Param `email` schema column references: first occurrence resolved from app.users.email but later occurrence resolved from contacts.email",
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

#[test]
fn mutation_fragment_repeat_param_validation_rejects_conflicting_schema_column_references() {
    let repeat_location = core::SourceLocation::at_position(
        "sql/mutation_fragments.sql",
        core::SourcePosition::one_based(2, 17).expect("test position should be valid"),
    );
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("touchUsers".to_owned()),
        "UPDATE users AS u SET name = name WHERE u.id IN (?,?);".to_owned(),
    )
    .with_param_usages(vec![
        test_param_usage("id", 48, false),
        test_param_usage("id", 50, false),
    ]);
    let variant = AnalyzedMutationVariant {
        mutation,
        analysis: core::AnalyzedMutation::new(core::MutationKind::Update),
        context: None,
        param_scopes: fragment_repeat_param_scopes("filter", "byIds", "ids"),
        param_occurrences: fragment_repeat_param_occurrences(
            "filter",
            "byIds",
            "ids",
            repeat_location,
        ),
    };
    let metadata = core::DbMutationMetadata::new().with_param_usages(vec![
        core::DbParamUsage::new("id".to_owned(), core::CoreType::Int64)
            .with_schema_column_reference(column_ref(None, "users", "id")),
        core::DbParamUsage::new("id".to_owned(), core::CoreType::Int64)
            .with_schema_column_reference(column_ref(None, "orders", "user_id")),
    ]);
    let mut scoped_param_bindings = Vec::new();

    let report = validate_expanded_mutation_variant_param_bindings(
        &variant,
        &metadata,
        &mut scoped_param_bindings,
    )
    .expect_err(
        "mutation Fragment Repeat item Param schema column reference conflicts should be rejected",
    );

    super::assert_diagnostic_messages(
        &report,
        "conflicting Fragment Repeat item Param `id` schema column reference in mutation `touchUsers`, Slot `filter`, Fragment `byIds`, Repeat `ids`: first representative occurrence resolved from users.id but conflicting representative occurrence resolved from orders.user_id; Repeat item fields with the same ID must resolve matching Param type, nullability, and schema column reference\nfirst Repeat `ids` occurrence in Slot `filter` selecting Fragment `byIds` is here\nconflicting Repeat `ids` occurrence in Slot `filter` selecting Fragment `byIds` is here",
    );
}

#[test]
fn repeated_mutation_fragment_param_validation_rejects_conflicting_schema_column_references() {
    let first_slot_location = core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(9, 5).expect("test position should be valid"),
    );
    let second_slot_location = core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(10, 5).expect("test position should be valid"),
    );
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("touchUsers".to_owned()),
        "UPDATE users SET kind = ? WHERE kind = ?;".to_owned(),
    )
    .with_param_usages(vec![
        test_param_usage("kind", 24, false),
        test_param_usage("kind", 39, false),
    ]);
    let variant = AnalyzedMutationVariant {
        mutation,
        analysis: core::AnalyzedMutation::new(core::MutationKind::Update),
        context: None,
        param_scopes: fragment_param_scopes("filter", "byKind"),
        param_occurrences: fragment_param_occurrences(
            "filter",
            "byKind",
            first_slot_location,
            second_slot_location,
        ),
    };
    let metadata = core::DbMutationMetadata::new().with_param_usages(vec![
        core::DbParamUsage::new("kind".to_owned(), core::CoreType::String)
            .with_schema_column_reference(column_ref(None, "users", "kind")),
        core::DbParamUsage::new("kind".to_owned(), core::CoreType::String)
            .with_schema_column_reference(column_ref(None, "profiles", "kind")),
    ]);
    let mut scoped_param_bindings = Vec::new();

    let report = validate_expanded_mutation_variant_param_bindings(
        &variant,
        &metadata,
        &mut scoped_param_bindings,
    )
    .expect_err(
        "repeated mutation Fragment Param schema column reference conflicts should be rejected",
    );

    super::assert_diagnostic_messages(
        &report,
        "conflicting Fragment Param `kind` schema column reference in mutation `touchUsers`, Slot `filter`, Fragment `byKind`: occurrence 1 resolved from users.kind but occurrence 2 resolved from profiles.kind; repeated Slot occurrences that select the same Fragment must resolve matching Param type, nullability, and schema column reference\nfirst occurrence of Slot `filter` selecting Fragment `byKind` is here\nconflicting occurrence of Slot `filter` selecting Fragment `byKind` is here",
    );
}

fn direct_param_scopes() -> Vec<ExpandedParamScope> {
    vec![
        ExpandedParamScope::QueryDirect,
        ExpandedParamScope::QueryDirect,
    ]
}

fn direct_param_occurrences() -> Vec<ExpandedParamOccurrence> {
    vec![
        ExpandedParamOccurrence::QueryDirect,
        ExpandedParamOccurrence::QueryDirect,
    ]
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

fn fragment_param_scopes(slot_id: &str, target_id: &str) -> Vec<ExpandedParamScope> {
    vec![
        ExpandedParamScope::Fragment {
            slot_id: slot_id.to_owned(),
            target_id: target_id.to_owned(),
        },
        ExpandedParamScope::Fragment {
            slot_id: slot_id.to_owned(),
            target_id: target_id.to_owned(),
        },
    ]
}

fn fragment_param_occurrences(
    slot_id: &str,
    target_id: &str,
    first_slot_location: core::SourceLocation,
    second_slot_location: core::SourceLocation,
) -> Vec<ExpandedParamOccurrence> {
    vec![
        ExpandedParamOccurrence::Fragment(ExpandedFragmentParamOccurrence {
            slot_id: slot_id.to_owned(),
            target_id: target_id.to_owned(),
            slot_occurrence_index: 1,
            slot_location: first_slot_location,
        }),
        ExpandedParamOccurrence::Fragment(ExpandedFragmentParamOccurrence {
            slot_id: slot_id.to_owned(),
            target_id: target_id.to_owned(),
            slot_occurrence_index: 2,
            slot_location: second_slot_location,
        }),
    ]
}

fn fragment_repeat_param_scopes(
    slot_id: &str,
    target_id: &str,
    repeat_id: &str,
) -> Vec<ExpandedParamScope> {
    vec![
        ExpandedParamScope::FragmentRepeatItem {
            slot_id: slot_id.to_owned(),
            target_id: target_id.to_owned(),
            repeat_id: repeat_id.to_owned(),
        },
        ExpandedParamScope::FragmentRepeatItem {
            slot_id: slot_id.to_owned(),
            target_id: target_id.to_owned(),
            repeat_id: repeat_id.to_owned(),
        },
    ]
}

fn fragment_repeat_param_occurrences(
    slot_id: &str,
    target_id: &str,
    repeat_id: &str,
    repeat_location: core::SourceLocation,
) -> Vec<ExpandedParamOccurrence> {
    vec![
        ExpandedParamOccurrence::FragmentRepeatItem(ExpandedFragmentRepeatParamOccurrence {
            slot_id: slot_id.to_owned(),
            target_id: target_id.to_owned(),
            repeat_id: repeat_id.to_owned(),
            representative_item_index: 1,
            slot_occurrence_index: 1,
            slot_location: core::SourceLocation::unknown(),
            repeat_location: repeat_location.clone(),
        }),
        ExpandedParamOccurrence::FragmentRepeatItem(ExpandedFragmentRepeatParamOccurrence {
            slot_id: slot_id.to_owned(),
            target_id: target_id.to_owned(),
            repeat_id: repeat_id.to_owned(),
            representative_item_index: 2,
            slot_occurrence_index: 1,
            slot_location: core::SourceLocation::unknown(),
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
