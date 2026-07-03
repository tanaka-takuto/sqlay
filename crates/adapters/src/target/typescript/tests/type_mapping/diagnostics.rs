use sqlay_app::TargetGenerator;
use sqlay_core as core;

use super::super::super::files::TypeScriptTargetGenerator;
use super::super::super::type_mapping::resolve_type_mapping;
use super::support::{
    assert_message, column_override, column_ref, compilation_plan_with_mapping,
    diagnostic_messages, import, named_override,
};

#[test]
fn rejects_unknown_and_unused_overrides_during_generation() {
    let mapping = core::TypeScriptTypeMappingConfig::new(
        Vec::new(),
        vec![column_override(
            column_ref(None, "orders", "missing_column"),
            "MissingColumn",
            None,
        )],
        vec![
            core::BuilderTypeOverrides::new(
                "missingBuilder".to_owned(),
                vec![named_override("field", "FieldType", None)],
                Vec::new(),
                Vec::new(),
            ),
            core::BuilderTypeOverrides::new(
                "listOrders".to_owned(),
                vec![named_override("missingField", "MissingField", None)],
                vec![named_override("missingParam", "MissingParam", None)],
                vec![core::RepeatTypeOverrides::new(
                    "missingRepeat".to_owned(),
                    vec![named_override("field", "FieldType", None)],
                )],
            ),
        ],
    );
    let plan = compilation_plan_with_mapping(mapping);
    let query = core::CompiledQuery::new(
        core::QueryId::new("listOrders".to_owned()),
        "SELECT total_amount AS totalAmount FROM orders WHERE total_amount >= ?;".to_owned(),
        core::Cardinality::Many,
        vec![core::InputField::new(
            "minimumAmount".to_owned(),
            core::CoreType::Decimal,
            false,
        )],
        vec![core::ResultColumn::new(
            "totalAmount".to_owned(),
            core::CoreType::Decimal,
            false,
        )],
    )
    .with_params(vec![core::ParamBinding::new(
        "minimumAmount".to_owned(),
        core::CoreType::Decimal,
        false,
    )])
    .with_source_path("sql/orders.sql");

    let report = TypeScriptTargetGenerator
        .generate(&plan, &[core::CompiledBuilder::Query(query)])
        .expect_err("invalid type mapping should stop generation");
    let messages = diagnostic_messages(&report);

    assert_message(
        &messages,
        "unknown TypeScript type mapping builder override `builders.missingBuilder`; no generated builder with that id exists",
    );
    assert_message(
        &messages,
        "unused TypeScript type mapping override `builders.listOrders.fields.missingField`; no result field with that name exists on builder `listOrders`",
    );
    assert_message(
        &messages,
        "unused TypeScript type mapping override `builders.listOrders.params.missingParam`; no direct Param input with that name exists on builder `listOrders`",
    );
    assert_message(
        &messages,
        "unused TypeScript type mapping override `builders.listOrders.repeats.missingRepeat`; no direct Repeat input with that id exists on builder `listOrders`",
    );
    assert_message(
        &messages,
        "unused TypeScript type mapping override `columns.orders.missing_column`; no generated field, Param, or Repeat item resolved to that schema column",
    );
}

#[test]
fn rejects_used_import_conflicts_within_one_generated_file() {
    let mapping = core::TypeScriptTypeMappingConfig::new(
        Vec::new(),
        Vec::new(),
        vec![core::BuilderTypeOverrides::new(
            "listOrders".to_owned(),
            vec![
                named_override(
                    "subtotal",
                    "Money",
                    Some(import("@/billing/money", "Money")),
                ),
                named_override("total", "Money", Some(import("@/orders/money", "Money"))),
            ],
            Vec::new(),
            Vec::new(),
        )],
    );
    let query = core::CompiledQuery::new(
        core::QueryId::new("listOrders".to_owned()),
        "SELECT subtotal, total FROM orders;".to_owned(),
        core::Cardinality::Many,
        Vec::new(),
        vec![
            core::ResultColumn::new("subtotal".to_owned(), core::CoreType::Decimal, false),
            core::ResultColumn::new("total".to_owned(), core::CoreType::Decimal, false),
        ],
    )
    .with_source_path("sql/orders.sql");

    let report = resolve_type_mapping(&mapping, &[core::CompiledBuilder::Query(query)])
        .expect_err("used import conflicts should be rejected");
    let messages = diagnostic_messages(&report);

    assert_message(
        &messages,
        "TypeScript type import conflict in source file `sql/orders.sql`: local type `Money` is imported from both `@/billing/money` and `@/orders/money`",
    );
}
