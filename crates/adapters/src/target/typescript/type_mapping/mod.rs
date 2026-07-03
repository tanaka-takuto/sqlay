use std::path::Path;

use sqlay_core as core;

mod builder;
mod diagnostics;
mod model;

use builder::resolve_builder_type_mapping;
use diagnostics::{
    TypeMappingUsage, push_import_conflict_diagnostics, push_unused_override_diagnostics,
};
use model::ResolvedTypeScriptType;

pub(in crate::target::typescript) use model::{
    BuilderTypeMappingResolution, TypeMappingResolution,
};

use super::literals::typescript_string_literal;
use super::types::typescript_core_type;

pub(super) fn resolve_type_mapping(
    mapping: &core::TypeScriptTypeMappingConfig,
    builders: &[core::CompiledBuilder],
) -> core::DiagnosticResult<TypeMappingResolution> {
    let mut usage = TypeMappingUsage::default();
    let mut diagnostics = Vec::new();

    for builder_override in mapping.builders() {
        if !builders
            .iter()
            .any(|builder| builder.id() == builder_override.builder_id())
        {
            diagnostics.push(core::Diagnostic::error(format!(
                "unknown TypeScript type mapping builder override `builders.{}`; no generated builder with that id exists",
                builder_override.builder_id()
            )));
        }
    }

    let mut resolved_builders = Vec::with_capacity(builders.len());
    for builder in builders {
        resolved_builders.push(resolve_builder_type_mapping(builder, mapping, &mut usage));
    }

    push_unused_override_diagnostics(mapping, builders, &usage, &mut diagnostics);
    push_import_conflict_diagnostics(&resolved_builders, &mut diagnostics);

    if diagnostics.is_empty() {
        Ok(TypeMappingResolution::new(resolved_builders))
    } else {
        Err(core::DiagnosticReport::from_diagnostics(diagnostics))
    }
}

pub(in crate::target::typescript::type_mapping) fn resolve_surface_type(
    builder_override: Option<(String, &core::TypeScriptTypeOverride)>,
    schema_column_reference: Option<&core::ColumnTypeReference>,
    type_ref: &core::CoreTypeRef,
    nullable: bool,
    mapping: &core::TypeScriptTypeMappingConfig,
    usage: &mut TypeMappingUsage,
) -> ResolvedTypeScriptType {
    if let Some((path, type_override)) = builder_override {
        usage.mark(path);
        return resolved_override_type(type_override, nullable);
    }

    if let Some(reference) = schema_column_reference
        && let Some(column_override) = mapping
            .columns()
            .iter()
            .find(|override_config| override_config.reference() == reference)
    {
        usage.mark(format!("columns.{}", column_reference_key(reference)));
        return resolved_override_type(column_override.type_override(), nullable);
    }

    if let Some(enum_values) = type_ref.enum_values() {
        return ResolvedTypeScriptType::new(
            nullable_annotation(enum_literal_union(enum_values), nullable),
            None,
        );
    }

    if let Some(core_override) = mapping
        .core()
        .iter()
        .find(|override_config| override_config.core_type() == type_ref.core_type())
    {
        usage.mark(format!("core.{}", core_type_key(type_ref.core_type())));
        return resolved_override_type(core_override.type_override(), nullable);
    }

    ResolvedTypeScriptType::new(
        nullable_annotation(
            typescript_core_type(type_ref.core_type()).to_owned(),
            nullable,
        ),
        None,
    )
}

fn resolved_override_type(
    type_override: &core::TypeScriptTypeOverride,
    nullable: bool,
) -> ResolvedTypeScriptType {
    ResolvedTypeScriptType::new(
        nullable_annotation(type_override.type_name().to_owned(), nullable),
        type_override.import().cloned(),
    )
}

fn nullable_annotation(base_type: String, nullable: bool) -> String {
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

pub(in crate::target::typescript::type_mapping) fn find_named_override<'a>(
    overrides: &'a [core::NamedTypeOverride],
    name: &str,
) -> Option<&'a core::TypeScriptTypeOverride> {
    overrides
        .iter()
        .find(|override_config| override_config.name() == name)
        .map(core::NamedTypeOverride::type_override)
}

fn column_reference_key(reference: &core::ColumnTypeReference) -> String {
    reference.database().map_or_else(
        || format!("{}.{}", reference.table(), reference.column()),
        |database| format!("{database}.{}.{}", reference.table(), reference.column()),
    )
}

const fn core_type_key(core_type: core::CoreType) -> &'static str {
    match core_type {
        core::CoreType::Bool => "bool",
        core::CoreType::Int32 => "int32",
        core::CoreType::Int64 => "int64",
        core::CoreType::Float64 => "float64",
        core::CoreType::Decimal => "decimal",
        core::CoreType::String => "string",
        core::CoreType::Bytes => "bytes",
        core::CoreType::Date => "date",
        core::CoreType::Time => "time",
        core::CoreType::DateTime => "datetime",
        core::CoreType::Json => "json",
        core::CoreType::Unknown => "unknown",
    }
}

pub(in crate::target::typescript::type_mapping) fn display_path(path: &Path) -> String {
    path.display().to_string()
}
