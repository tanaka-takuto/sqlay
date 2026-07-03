use sqlay_core as core;

use super::super::super::type_mapping::resolve_type_mapping;
use super::super::support::sql_segment;
use super::support::{
    column_override, column_ref, core_type_override, enum_type_ref, import, named_override,
};

#[test]
fn resolves_static_query_type_mapping_priority() {
    let order_total = column_ref(None, "orders", "total_amount");
    let order_status = column_ref(None, "orders", "status");
    let mapping = core::TypeScriptTypeMappingConfig::new(
        vec![core_type_override(core::CoreType::DateTime, "Date", None)],
        vec![column_override(
            order_total.clone(),
            "MoneyAmount",
            Some(import("@/domain/money", "MoneyAmount")),
        )],
        vec![core::BuilderTypeOverrides::new(
            "listOrders".to_owned(),
            vec![named_override("totalAmount", "OrderTotal", None)],
            Vec::new(),
            Vec::new(),
        )],
    );
    let query = core::CompiledQuery::new(
        core::QueryId::new("listOrders".to_owned()),
        "SELECT total_amount AS totalAmount, status, created_at AS createdAt FROM orders WHERE total_amount >= ?;".to_owned(),
        core::Cardinality::Many,
        vec![
            core::InputField::new("minimumAmount".to_owned(), core::CoreType::Decimal, true)
                .with_schema_column_reference(order_total.clone()),
        ],
        vec![
            core::ResultColumn::new("totalAmount".to_owned(), core::CoreType::Decimal, true)
                .with_schema_column_reference(order_total),
            core::ResultColumn::new_type_ref(
                "status".to_owned(),
                enum_type_ref(["draft", "paid"]),
                false,
            )
            .with_schema_column_reference(order_status),
            core::ResultColumn::new("createdAt".to_owned(), core::CoreType::DateTime, false),
        ],
    )
    .with_params(vec![
        core::ParamBinding::new("minimumAmount".to_owned(), core::CoreType::Decimal, true)
            .with_schema_column_reference(column_ref(None, "orders", "total_amount")),
    ])
    .with_source_path("sql/orders.sql");

    let resolution = resolve_type_mapping(&mapping, &[core::CompiledBuilder::Query(query)])
        .expect("type mapping should resolve");
    let builder = resolution
        .builder("listOrders")
        .expect("builder mapping should be resolved");

    assert_eq!(builder.field("totalAmount"), Some("OrderTotal | null"));
    assert_eq!(builder.field("status"), Some(r#""draft" | "paid""#));
    assert_eq!(builder.field("createdAt"), Some("Date"));
    assert_eq!(builder.input("minimumAmount"), Some("MoneyAmount | null"));
    assert_eq!(builder.fixed_param(0), Some("MoneyAmount | null"));
}

#[test]
fn resolves_repeat_fields_without_narrowing_dynamic_params_array() {
    let mapping = core::TypeScriptTypeMappingConfig::new(
        Vec::new(),
        Vec::new(),
        vec![core::BuilderTypeOverrides::new(
            "createOrder".to_owned(),
            Vec::new(),
            Vec::new(),
            vec![core::RepeatTypeOverrides::new(
                "lineItems".to_owned(),
                vec![named_override(
                    "unitPrice",
                    "MoneyAmount",
                    Some(import("@/domain/money", "MoneyAmount")),
                )],
            )],
        )],
    );
    let repeat = core::CompiledRepeatDefinition::new(
        "lineItems".to_owned(),
        vec![
            core::ParamBinding::new("sku".to_owned(), core::CoreType::String, false),
            core::ParamBinding::new("unitPrice".to_owned(), core::CoreType::Decimal, false)
                .with_schema_column_reference(column_ref(None, "order_items", "unit_price")),
        ],
    );
    let dynamic_body = core::CompiledDynamicQuery::new_with_bodies(
        vec![core::CompiledSqlBody::new(
            vec![
                sql_segment(
                    "INSERT INTO order_items (sku, unit_price) VALUES ",
                    Vec::new(),
                ),
                sql_segment(";", Vec::new()),
            ],
            vec![core::CompiledRepeatOccurrence::new(
                "lineItems".to_owned(),
                ",".to_owned(),
                sql_segment(
                    "(?, ?)",
                    vec![
                        core::ParamBinding::new("sku".to_owned(), core::CoreType::String, false),
                        core::ParamBinding::new(
                            "unitPrice".to_owned(),
                            core::CoreType::Decimal,
                            false,
                        )
                        .with_schema_column_reference(column_ref(
                            None,
                            "order_items",
                            "unit_price",
                        )),
                    ],
                ),
            )],
        )],
        Vec::new(),
        Vec::new(),
        vec![repeat],
    );
    let mutation = core::CompiledMutation::new(
        core::MutationId::new("createOrder".to_owned()),
        "INSERT INTO order_items (sku, unit_price) VALUES (?, ?);".to_owned(),
        core::MutationKind::Insert,
        Vec::new(),
    )
    .with_dynamic_body(dynamic_body)
    .with_source_path("sql/orders.sql");

    let resolution = resolve_type_mapping(&mapping, &[core::CompiledBuilder::Mutation(mutation)])
        .expect("type mapping should resolve");
    let builder = resolution
        .builder("createOrder")
        .expect("builder mapping should be resolved");

    assert_eq!(
        builder.dynamic_params_annotation(),
        Some("readonly SqlParam[]")
    );
    assert_eq!(builder.fixed_param(0), None);
    assert_eq!(
        builder.repeat_field("lineItems", "unitPrice"),
        Some("MoneyAmount")
    );
    assert_eq!(builder.repeat_field("lineItems", "sku"), Some("string"));
}

#[test]
fn slot_branch_param_column_override_counts_as_used() {
    let user_email = column_ref(None, "users", "email");
    let mapping = core::TypeScriptTypeMappingConfig::new(
        Vec::new(),
        vec![column_override(user_email.clone(), "EmailAddress", None)],
        Vec::new(),
    );
    let branch = core::CompiledSlotBranch::new(
        "emailFilter".to_owned(),
        vec![sql_segment(
            " AND email = ?",
            vec![
                core::ParamBinding::new("email".to_owned(), core::CoreType::String, false)
                    .with_schema_column_reference(user_email),
            ],
        )],
    );
    let dynamic_body = core::CompiledDynamicQuery::new(
        vec![
            sql_segment("SELECT id FROM users WHERE active = TRUE", Vec::new()),
            sql_segment(";", Vec::new()),
        ],
        vec![core::CompiledSlotOccurrence::new("filter".to_owned())],
        vec![core::CompiledSlotDefinition::new(
            "filter".to_owned(),
            vec![branch],
        )],
    );
    let query = core::CompiledQuery::new(
        core::QueryId::new("listUsers".to_owned()),
        "SELECT id FROM users WHERE active = TRUE;".to_owned(),
        core::Cardinality::Many,
        Vec::new(),
        Vec::new(),
    )
    .with_dynamic_body(dynamic_body)
    .with_source_path("sql/users.sql");

    let resolution = resolve_type_mapping(&mapping, &[core::CompiledBuilder::Query(query)])
        .expect("slot branch Param column override should be resolved as used");
    let builder = resolution
        .builder("listUsers")
        .expect("builder mapping should be resolved");

    assert_eq!(
        builder.slot_branch_param("filter", "emailFilter", "email"),
        Some("EmailAddress")
    );
}
