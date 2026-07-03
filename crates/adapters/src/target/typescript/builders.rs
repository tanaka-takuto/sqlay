use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use sqlay_core as core;

use super::literals::typescript_string_literal;
use super::slots::{
    is_slot_mutation, is_slot_query, render_dynamic_sql_body, render_repeat_guards,
    render_slot_switch,
};
use super::symbols::{MutationSymbols, QuerySymbols};
use super::type_mapping::{BuilderTypeMappingResolution, TypeMappingResolution};
use super::types::{
    function_input_name, function_input_name_for_dynamic_body, input_param_access,
    render_dynamic_function_input_parameter, render_dynamic_input_type_alias,
    render_function_input_parameter, render_input_type_alias, typescript_dynamic_params_type,
    typescript_output_type, typescript_params_expression, typescript_params_type,
    typescript_property_name, typescript_result_type,
};

/// Render a generated query builder `sql` property.
#[must_use]
pub fn render_sql_property(query: &core::CompiledQuery) -> String {
    render_sql_property_from_sql(query.sql())
}

/// Render a full generated TypeScript file from compiled queries.
#[must_use]
pub fn render_generated_file_contents(queries: &[core::CompiledQuery]) -> String {
    render_generated_file_contents_from_iter(queries.iter())
}

pub(super) fn render_generated_file_contents_from_iter<'a>(
    queries: impl IntoIterator<Item = &'a core::CompiledQuery>,
) -> String {
    let queries = queries.into_iter().collect::<Vec<_>>();
    let mut contents =
        render_generated_file_prelude(queries.iter().copied().any(is_slot_query), &[]);

    let mut is_first_builder = true;
    for query in queries {
        if is_first_builder {
            is_first_builder = false;
        } else {
            contents.push('\n');
        }
        contents.push_str(&render_query_with_type_mapping(query, None));
    }

    contents
}

pub(super) fn render_generated_builder_file_contents_with_type_mapping(
    builders: &[&core::CompiledBuilder],
    type_mapping: &TypeMappingResolution,
) -> String {
    render_generated_builder_file_contents_with_optional_type_mapping(builders, Some(type_mapping))
}

fn render_generated_builder_file_contents_with_optional_type_mapping(
    builders: &[&core::CompiledBuilder],
    type_mapping: Option<&TypeMappingResolution>,
) -> String {
    let type_imports = collect_type_imports(builders, type_mapping);
    let mut contents = render_generated_file_prelude(
        builders.iter().copied().any(builder_uses_sql_param_alias),
        &type_imports,
    );

    let mut is_first_builder = true;
    for builder in builders {
        if is_first_builder {
            is_first_builder = false;
        } else {
            contents.push('\n');
        }
        let builder_type_mapping =
            type_mapping.map(|resolution| required_builder_type_mapping(builder, resolution));
        contents.push_str(&render_builder(builder, builder_type_mapping));
    }

    contents
}

fn render_generated_file_prelude(
    include_sql_param_alias: bool,
    type_imports: &[(String, Vec<String>)],
) -> String {
    let mut contents = String::from(core::GENERATED_FILE_HEADER);
    contents.push_str("\n\n");

    for (from, names) in type_imports {
        writeln!(
            contents,
            "import type {{ {} }} from {};",
            names.join(", "),
            typescript_string_literal(from)
        )
        .expect("writing to String cannot fail");
    }

    if !type_imports.is_empty() {
        contents.push('\n');
    }

    if include_sql_param_alias {
        contents.push_str("type SqlParam = unknown;\n\n");
    }

    contents
}

fn collect_type_imports(
    builders: &[&core::CompiledBuilder],
    type_mapping: Option<&TypeMappingResolution>,
) -> Vec<(String, Vec<String>)> {
    let Some(type_mapping) = type_mapping else {
        return Vec::new();
    };

    let mut by_module = BTreeMap::<String, BTreeSet<String>>::new();
    for builder in builders {
        let builder_mapping = required_builder_type_mapping(builder, type_mapping);
        for import in builder_mapping.imports() {
            by_module
                .entry(import.from().to_owned())
                .or_default()
                .insert(import.name().to_owned());
        }
    }

    by_module
        .into_iter()
        .map(|(from, names)| (from, names.into_iter().collect()))
        .collect()
}

fn required_builder_type_mapping<'a>(
    builder: &core::CompiledBuilder,
    type_mapping: &'a TypeMappingResolution,
) -> &'a BuilderTypeMappingResolution {
    type_mapping
        .builder(builder.id())
        .expect("resolved TypeScript type mapping should include every builder")
}

const fn builder_uses_sql_param_alias(builder: &core::CompiledBuilder) -> bool {
    match builder {
        core::CompiledBuilder::Query(query) => is_slot_query(query),
        core::CompiledBuilder::Mutation(mutation) => is_slot_mutation(mutation),
    }
}

fn render_builder(
    builder: &core::CompiledBuilder,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) -> String {
    match builder {
        core::CompiledBuilder::Query(query) => render_query_with_type_mapping(query, type_mapping),
        core::CompiledBuilder::Mutation(mutation) => {
            render_mutation_with_type_mapping(mutation, type_mapping)
        }
    }
}

/// Render TypeScript declarations and the SQL builder for one compiled query.
#[must_use]
pub fn render_query(query: &core::CompiledQuery) -> String {
    render_query_with_type_mapping(query, None)
}

fn render_query_with_type_mapping(
    query: &core::CompiledQuery,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) -> String {
    let symbols = QuerySymbols::for_query(query);
    let mut output = String::new();

    render_input_type_alias(&mut output, query, &symbols, type_mapping);
    output.push('\n');

    writeln!(&mut output, "export type {} = {{", symbols.row_type_name())
        .expect("writing to String cannot fail");
    for column in query.row() {
        writeln!(
            &mut output,
            "  {}: {};",
            typescript_property_name(column.name()),
            typescript_result_type(column, type_mapping)
        )
        .expect("writing to String cannot fail");
    }
    output.push_str("};\n\n");

    writeln!(
        &mut output,
        "export type {} = {};",
        symbols.output_type_name(),
        typescript_output_type(&symbols, query.cardinality())
    )
    .expect("writing to String cannot fail");
    output.push('\n');

    writeln!(&mut output, "export function {}(", symbols.function_name())
        .expect("writing to String cannot fail");
    render_function_input_parameter(&mut output, query, &symbols);
    writeln!(
        &mut output,
        "): {{ sql: string; params: {} }} {{",
        typescript_params_type(query, type_mapping)
    )
    .expect("writing to String cannot fail");
    if let Some(dynamic_body) = query.dynamic_body() {
        render_dynamic_builder_body(&mut output, query, dynamic_body);
    } else {
        render_static_builder_body(&mut output, query.sql(), query.params());
    }
    output.push_str("}\n");

    output
}

/// Render TypeScript declarations and the SQL builder for one compiled mutation.
#[must_use]
#[cfg(test)]
pub fn render_mutation(mutation: &core::CompiledMutation) -> String {
    render_mutation_with_type_mapping(mutation, None)
}

fn render_mutation_with_type_mapping(
    mutation: &core::CompiledMutation,
    type_mapping: Option<&BuilderTypeMappingResolution>,
) -> String {
    let symbols = MutationSymbols::for_mutation(mutation);
    let mut output = String::new();

    render_dynamic_input_type_alias(
        &mut output,
        symbols.input_type_name(),
        mutation.input(),
        mutation.dynamic_body(),
        type_mapping,
    );
    output.push('\n');

    writeln!(&mut output, "export function {}(", symbols.function_name())
        .expect("writing to String cannot fail");
    render_dynamic_function_input_parameter(
        &mut output,
        symbols.input_type_name(),
        mutation.input(),
        mutation.dynamic_body(),
    );
    writeln!(
        &mut output,
        "): {{ sql: string; params: {} }} {{",
        typescript_dynamic_params_type(mutation.dynamic_body(), mutation.params(), type_mapping)
    )
    .expect("writing to String cannot fail");
    if let Some(dynamic_body) = mutation.dynamic_body() {
        render_dynamic_sql_builder_body(
            &mut output,
            function_input_name_for_dynamic_body(mutation.input(), mutation.dynamic_body()),
            dynamic_body,
        );
    } else {
        render_static_builder_body(&mut output, mutation.sql(), mutation.params());
    }
    output.push_str("}\n");

    output
}

fn render_sql_property_from_sql(sql: &str) -> String {
    format!("    sql: {},", typescript_string_literal(sql))
}

fn render_static_builder_body(output: &mut String, sql: &str, params: &[core::ParamBinding]) {
    output.push_str("  return {\n");
    writeln!(output, "{}", render_sql_property_from_sql(sql))
        .expect("writing to String cannot fail");
    writeln!(
        output,
        "    params: {} as const,",
        typescript_params_expression(params)
    )
    .expect("writing to String cannot fail");
    output.push_str("  };\n");
}

fn render_dynamic_builder_body(
    output: &mut String,
    query: &core::CompiledQuery,
    dynamic_body: &core::CompiledDynamicQuery,
) {
    render_dynamic_sql_builder_body(output, function_input_name(query), dynamic_body);
}

fn render_dynamic_sql_builder_body(
    output: &mut String,
    input_name: &str,
    dynamic_body: &core::CompiledDynamicQuery,
) {
    output.push_str("  const sqlParts: string[] = [];\n");
    output.push_str("  const params: SqlParam[] = [];\n\n");

    render_repeat_guards(
        output,
        "  ",
        dynamic_body.repeats(),
        |repeat_id| input_param_access(input_name, repeat_id),
        true,
    );

    for (index, body) in dynamic_body.base_bodies().iter().enumerate() {
        render_dynamic_sql_body(
            output,
            "  ",
            body,
            |param| input_param_access(input_name, param.input_name()),
            |repeat_id| input_param_access(input_name, repeat_id),
        );

        if let Some(occurrence) = dynamic_body.slot_occurrences().get(index) {
            let slot = dynamic_body
                .slots()
                .iter()
                .find(|slot| slot.id() == occurrence.slot_id())
                .expect("compiled dynamic query Slot occurrence must have a Slot definition");
            render_slot_switch(output, input_name, slot);
        }
    }

    output.push('\n');
    output.push_str("  return {\n");
    output.push_str("    sql: sqlParts.join(\"\"),\n");
    output.push_str("    params,\n");
    output.push_str("  };\n");
}
