use std::fmt::Write as _;

use sqlay_core as core;

use super::literals::typescript_string_literal;
use super::slots::render_slot_input_field;
use super::symbols::QuerySymbols;
use super::type_mapping::BuilderTypeMappingResolution;

pub(super) fn render_input_type_alias(
    output: &mut String,
    query: &core::CompiledQuery,
    symbols: &QuerySymbols,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) {
    render_dynamic_input_type_alias(
        output,
        symbols.input_type_name(),
        query.input(),
        query.dynamic_body(),
        type_mapping,
    );
}

pub(super) fn render_dynamic_input_type_alias(
    output: &mut String,
    input_type_name: &str,
    input: &[core::InputField],
    dynamic_body: Option<&core::CompiledDynamicQuery>,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) {
    let Some(dynamic_body) = dynamic_body else {
        render_static_input_type_alias(output, input_type_name, input, type_mapping);
        return;
    };

    if dynamic_body.slots().is_empty() && dynamic_body.repeats().is_empty() {
        render_static_input_type_alias(output, input_type_name, input, type_mapping);
        return;
    }

    writeln!(output, "export type {input_type_name} = {{").expect("writing to String cannot fail");
    render_dynamic_input_fields(output, input, dynamic_body, type_mapping);
    output.push_str("};\n");
}

pub(super) fn render_static_input_type_alias(
    output: &mut String,
    input_type_name: &str,
    input: &[core::InputField],
    type_mapping: Option<&BuilderTypeMappingResolution>,
) {
    if input.is_empty() {
        writeln!(
            output,
            "export type {input_type_name} = Record<string, never>;"
        )
        .expect("writing to String cannot fail");
        return;
    }

    writeln!(output, "export type {input_type_name} = {{").expect("writing to String cannot fail");
    for field in input {
        render_input_field(output, "  ", field, type_mapping);
    }
    output.push_str("};\n");
}

pub(super) fn render_function_input_parameter(
    output: &mut String,
    query: &core::CompiledQuery,
    symbols: &QuerySymbols,
) {
    render_dynamic_function_input_parameter(
        output,
        symbols.input_type_name(),
        query.input(),
        query.dynamic_body(),
    );
}

pub(super) fn render_dynamic_function_input_parameter(
    output: &mut String,
    input_type_name: &str,
    input: &[core::InputField],
    dynamic_body: Option<&core::CompiledDynamicQuery>,
) {
    let (input_name, default) = if input.is_empty() && !dynamic_body_requires_input(dynamic_body) {
        ("_input", " = {}")
    } else {
        ("input", "")
    };
    writeln!(output, "  {input_name}: {input_type_name}{default},")
        .expect("writing to String cannot fail");
}

pub(super) fn function_input_name(query: &core::CompiledQuery) -> &'static str {
    function_input_name_for_dynamic_body(query.input(), query.dynamic_body())
}

pub(super) fn function_input_name_for_dynamic_body(
    input: &[core::InputField],
    dynamic_body: Option<&core::CompiledDynamicQuery>,
) -> &'static str {
    if input.is_empty() && !dynamic_body_requires_input(dynamic_body) {
        "_input"
    } else {
        "input"
    }
}

pub(super) fn typescript_output_type(
    symbols: &QuerySymbols,
    cardinality: core::Cardinality,
) -> String {
    let row_type = symbols.row_type_name();

    match cardinality {
        core::Cardinality::One => format!("{row_type} | null"),
        core::Cardinality::Many => format!("{row_type}[]"),
    }
}

fn typescript_input_field_type(
    field: &core::InputField,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) -> String {
    type_mapping
        .and_then(|mapping| mapping.input(field.name()))
        .map_or_else(
            || typescript_nullable_type_ref(field.type_ref(), field.is_nullable()),
            str::to_owned,
        )
}

pub(super) fn typescript_param_binding_type(param: &core::ParamBinding) -> String {
    typescript_nullable_type_ref(param.type_ref(), param.is_nullable())
}

pub(super) fn render_repeat_input_field(
    output: &mut String,
    indent: &str,
    repeat: &core::CompiledRepeatDefinition,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) {
    writeln!(
        output,
        "{indent}{}: {};",
        typescript_property_name(repeat.id()),
        typescript_repeat_input_type(repeat, type_mapping)
    )
    .expect("writing to String cannot fail");
}

pub(super) fn render_slot_branch_repeat_input_field(
    output: &mut String,
    indent: &str,
    slot_id: &str,
    target_id: &str,
    repeat: &core::CompiledRepeatDefinition,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) {
    writeln!(
        output,
        "{indent}{}: {};",
        typescript_property_name(repeat.id()),
        typescript_slot_branch_repeat_input_type(slot_id, target_id, repeat, type_mapping)
    )
    .expect("writing to String cannot fail");
}

pub(super) fn typescript_result_type(
    column: &core::ResultColumn,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) -> String {
    type_mapping
        .and_then(|mapping| mapping.field(column.name()))
        .map_or_else(
            || typescript_nullable_type_ref(column.type_ref(), column.is_nullable()),
            str::to_owned,
        )
}

fn typescript_nullable_type_ref(type_ref: &core::CoreTypeRef, nullable: bool) -> String {
    let base_type = type_ref.enum_values().map_or_else(
        || typescript_core_type(type_ref.core_type()).to_owned(),
        enum_literal_union,
    );

    if nullable {
        format!("{base_type} | null")
    } else {
        base_type
    }
}

fn enum_literal_union(values: &[String]) -> String {
    values
        .iter()
        .map(|value| typescript_string_literal(value))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn render_input_field(
    output: &mut String,
    indent: &str,
    field: &core::InputField,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) {
    writeln!(
        output,
        "{indent}{}: {};",
        typescript_property_name(field.name()),
        typescript_input_field_type(field, type_mapping)
    )
    .expect("writing to String cannot fail");
}

fn render_dynamic_input_fields(
    output: &mut String,
    input: &[core::InputField],
    dynamic_body: &core::CompiledDynamicQuery,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) {
    let mut rendered_fields = Vec::new();
    let mut rendered_repeats = Vec::new();
    let mut rendered_slots = Vec::new();

    for (body_index, body) in dynamic_body.base_bodies().iter().enumerate() {
        for (segment_index, segment) in body.base_segments().iter().enumerate() {
            for param in segment.params() {
                render_dynamic_direct_input_field(
                    output,
                    input,
                    param.input_name(),
                    &mut rendered_fields,
                    type_mapping,
                );
            }

            if let Some(repeat) = body.repeat_occurrences().get(segment_index) {
                render_dynamic_repeat_input_field(
                    output,
                    dynamic_body.repeats(),
                    repeat.repeat_id(),
                    &mut rendered_repeats,
                    type_mapping,
                );
            }
        }

        if let Some(slot) = dynamic_body.slot_occurrences().get(body_index) {
            render_dynamic_slot_input_field(
                output,
                dynamic_body.slots(),
                slot.slot_id(),
                &mut rendered_slots,
                type_mapping,
            );
        }
    }

    for field in input {
        render_dynamic_direct_input_field(
            output,
            input,
            field.name(),
            &mut rendered_fields,
            type_mapping,
        );
    }
    for repeat in dynamic_body.repeats() {
        render_dynamic_repeat_input_field(
            output,
            dynamic_body.repeats(),
            repeat.id(),
            &mut rendered_repeats,
            type_mapping,
        );
    }
    for slot in dynamic_body.slots() {
        render_dynamic_slot_input_field(
            output,
            dynamic_body.slots(),
            slot.id(),
            &mut rendered_slots,
            type_mapping,
        );
    }
}

fn render_dynamic_direct_input_field(
    output: &mut String,
    input: &[core::InputField],
    name: &str,
    rendered_fields: &mut Vec<String>,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) {
    if rendered_fields.iter().any(|rendered| rendered == name) {
        return;
    }

    if let Some(field) = input.iter().find(|field| field.name() == name) {
        render_input_field(output, "  ", field, type_mapping);
        rendered_fields.push(field.name().to_owned());
    }
}

fn render_dynamic_repeat_input_field(
    output: &mut String,
    repeats: &[core::CompiledRepeatDefinition],
    id: &str,
    rendered_repeats: &mut Vec<String>,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) {
    if rendered_repeats.iter().any(|rendered| rendered == id) {
        return;
    }

    if let Some(repeat) = repeats.iter().find(|repeat| repeat.id() == id) {
        render_repeat_input_field(output, "  ", repeat, type_mapping);
        rendered_repeats.push(repeat.id().to_owned());
    }
}

fn render_dynamic_slot_input_field(
    output: &mut String,
    slots: &[core::CompiledSlotDefinition],
    id: &str,
    rendered_slots: &mut Vec<String>,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) {
    if rendered_slots.iter().any(|rendered| rendered == id) {
        return;
    }

    if let Some(slot) = slots.iter().find(|slot| slot.id() == id) {
        render_slot_input_field(output, slot, type_mapping);
        rendered_slots.push(slot.id().to_owned());
    }
}

pub(super) fn render_param_binding_input_field(
    output: &mut String,
    indent: &str,
    param: &core::ParamBinding,
    annotation: Option<&str>,
) {
    writeln!(
        output,
        "{indent}{}: {};",
        typescript_property_name(param.input_name()),
        annotation.map_or_else(|| typescript_param_binding_type(param), str::to_owned)
    )
    .expect("writing to String cannot fail");
}

fn typescript_repeat_input_type(
    repeat: &core::CompiledRepeatDefinition,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) -> String {
    let item_type = typescript_repeat_item_type(repeat.fields(), |field| {
        type_mapping
            .and_then(|mapping| mapping.repeat_field(repeat.id(), field.input_name()))
            .map_or_else(|| typescript_param_binding_type(field), str::to_owned)
    });
    format!("readonly [{item_type}, ...{item_type}[]]")
}

fn typescript_slot_branch_repeat_input_type(
    slot_id: &str,
    target_id: &str,
    repeat: &core::CompiledRepeatDefinition,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) -> String {
    let item_type = typescript_repeat_item_type(repeat.fields(), |field| {
        type_mapping
            .and_then(|mapping| {
                mapping.slot_branch_repeat_field(
                    slot_id,
                    target_id,
                    repeat.id(),
                    field.input_name(),
                )
            })
            .map_or_else(|| typescript_param_binding_type(field), str::to_owned)
    });
    format!("readonly [{item_type}, ...{item_type}[]]")
}

fn typescript_repeat_item_type<F>(fields: &[core::ParamBinding], field_type: F) -> String
where
    F: Fn(&core::ParamBinding) -> String,
{
    let fields = fields
        .iter()
        .map(|field| {
            format!(
                "{}: {}",
                typescript_property_name(field.input_name()),
                field_type(field)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!("{{ {fields} }}")
}

fn dynamic_body_requires_input(dynamic_body: Option<&core::CompiledDynamicQuery>) -> bool {
    let Some(dynamic_body) = dynamic_body else {
        return false;
    };

    !dynamic_body.repeats().is_empty()
        || dynamic_body.slots().iter().any(|slot| {
            slot.branches()
                .iter()
                .any(|branch| !branch.repeats().is_empty())
        })
}

pub(super) const fn typescript_core_type(ty: core::CoreType) -> &'static str {
    match ty {
        core::CoreType::Bool => "boolean",
        core::CoreType::Int32 | core::CoreType::Float64 => "number",
        core::CoreType::Int64
        | core::CoreType::Decimal
        | core::CoreType::Date
        | core::CoreType::Time
        | core::CoreType::DateTime
        | core::CoreType::String => "string",
        core::CoreType::Bytes => "Uint8Array",
        core::CoreType::Json | core::CoreType::Unknown => "unknown",
    }
}

pub(super) fn typescript_property_name(name: &str) -> String {
    if is_simple_typescript_identifier(name) {
        name.to_owned()
    } else {
        typescript_string_literal(name)
    }
}

pub(super) fn typescript_params_tuple_type(
    params: &[core::ParamBinding],
    type_mapping: Option<&BuilderTypeMappingResolution>,
) -> String {
    if params.is_empty() {
        "readonly []".to_owned()
    } else {
        format!(
            "readonly [{}]",
            params
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    type_mapping
                        .and_then(|mapping| mapping.fixed_param(index))
                        .map_or_else(|| typescript_param_binding_type(param), str::to_owned)
                })
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

pub(super) fn typescript_params_type(
    query: &core::CompiledQuery,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) -> String {
    typescript_dynamic_params_type(query.dynamic_body(), query.params(), type_mapping)
}

pub(super) fn typescript_dynamic_params_type(
    dynamic_body: Option<&core::CompiledDynamicQuery>,
    params: &[core::ParamBinding],
    type_mapping: Option<&BuilderTypeMappingResolution>,
) -> String {
    if dynamic_body.is_some() {
        type_mapping
            .and_then(BuilderTypeMappingResolution::dynamic_params_annotation)
            .unwrap_or("readonly SqlParam[]")
            .to_owned()
    } else {
        typescript_params_tuple_type(params, type_mapping)
    }
}

pub(super) fn typescript_params_expression(params: &[core::ParamBinding]) -> String {
    if params.is_empty() {
        "[]".to_owned()
    } else {
        format!(
            "[{}]",
            params
                .iter()
                .map(|param| input_param_access("input", param.input_name()))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

pub(super) fn input_param_access(input_name: &str, param_name: &str) -> String {
    typescript_property_access(input_name, param_name)
}

pub(super) fn nested_slot_param_access(
    input_name: &str,
    slot_id: &str,
    param_name: &str,
) -> String {
    typescript_property_access(&typescript_property_access(input_name, slot_id), param_name)
}

fn typescript_property_access(base: &str, property: &str) -> String {
    if is_simple_typescript_identifier(property) {
        format!("{base}.{property}")
    } else {
        format!("{base}[{}]", typescript_string_literal(property))
    }
}

fn is_simple_typescript_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    is_identifier_start(first) && chars.all(is_identifier_continue)
}

const fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic()
}

const fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}
