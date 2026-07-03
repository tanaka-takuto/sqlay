use sqlay_core as core;

use super::diagnostics::TypeMappingUsage;
use super::model::{BuilderTypeMappingResolution, NamedResolvedType, RepeatTypeMappingResolution};
use super::{display_path, find_named_override, resolve_surface_type};

pub(super) fn resolve_builder_type_mapping(
    builder: &core::CompiledBuilder,
    mapping: &core::TypeScriptTypeMappingConfig,
    usage: &mut TypeMappingUsage,
) -> BuilderTypeMappingResolution {
    let builder_override = mapping
        .builders()
        .iter()
        .find(|override_config| override_config.builder_id() == builder.id());
    let mut resolution = BuilderTypeMappingResolution::new(
        builder.id().to_owned(),
        builder.source_path().map(display_path),
    );

    match builder {
        core::CompiledBuilder::Query(query) => {
            resolve_result_fields(query, builder_override, mapping, usage, &mut resolution);
            resolve_direct_inputs(
                query.input(),
                query.params(),
                builder_override,
                mapping,
                usage,
                &mut resolution,
            );
            resolve_params(
                query.dynamic_body(),
                query.params(),
                builder_override,
                mapping,
                usage,
                &mut resolution,
            );
            resolve_direct_repeats(
                query.dynamic_body(),
                builder_override,
                mapping,
                usage,
                &mut resolution,
            );
        }
        core::CompiledBuilder::Mutation(mutation) => {
            resolve_direct_inputs(
                mutation.input(),
                mutation.params(),
                builder_override,
                mapping,
                usage,
                &mut resolution,
            );
            resolve_params(
                mutation.dynamic_body(),
                mutation.params(),
                builder_override,
                mapping,
                usage,
                &mut resolution,
            );
            resolve_direct_repeats(
                mutation.dynamic_body(),
                builder_override,
                mapping,
                usage,
                &mut resolution,
            );
        }
    }

    resolution
}

fn resolve_result_fields(
    query: &core::CompiledQuery,
    builder_override: Option<&core::BuilderTypeOverrides>,
    mapping: &core::TypeScriptTypeMappingConfig,
    usage: &mut TypeMappingUsage,
    resolution: &mut BuilderTypeMappingResolution,
) {
    for column in query.row() {
        let override_config = builder_override
            .and_then(|builder| find_named_override(builder.fields(), column.name()))
            .map(|type_override| {
                (
                    format!("builders.{}.fields.{}", query.id().as_str(), column.name()),
                    type_override,
                )
            });
        let ty = resolve_surface_type(
            override_config,
            column.schema_column_reference(),
            column.type_ref(),
            column.is_nullable(),
            mapping,
            usage,
        );
        resolution.fields.push(NamedResolvedType {
            name: column.name().to_owned(),
            ty,
        });
    }
}

fn resolve_direct_inputs(
    inputs: &[core::InputField],
    params: &[core::ParamBinding],
    builder_override: Option<&core::BuilderTypeOverrides>,
    mapping: &core::TypeScriptTypeMappingConfig,
    usage: &mut TypeMappingUsage,
    resolution: &mut BuilderTypeMappingResolution,
) {
    for input in inputs {
        let override_config = builder_override
            .and_then(|builder| find_named_override(builder.params(), input.name()))
            .map(|type_override| {
                (
                    format!("builders.{}.params.{}", resolution.id, input.name()),
                    type_override,
                )
            });
        let schema_reference = input
            .schema_column_reference()
            .or_else(|| unique_param_schema_reference(input.name(), params));
        let ty = resolve_surface_type(
            override_config,
            schema_reference,
            input.type_ref(),
            input.is_nullable(),
            mapping,
            usage,
        );
        resolution.inputs.push(NamedResolvedType {
            name: input.name().to_owned(),
            ty,
        });
    }
}

fn resolve_params(
    dynamic_body: Option<&core::CompiledDynamicQuery>,
    params: &[core::ParamBinding],
    builder_override: Option<&core::BuilderTypeOverrides>,
    mapping: &core::TypeScriptTypeMappingConfig,
    usage: &mut TypeMappingUsage,
    resolution: &mut BuilderTypeMappingResolution,
) {
    if dynamic_body.is_some() {
        resolution.dynamic_params_annotation = Some("readonly SqlParam[]".to_owned());
        return;
    }

    for param in params {
        let override_config = builder_override
            .and_then(|builder| find_named_override(builder.params(), param.input_name()))
            .map(|type_override| {
                (
                    format!("builders.{}.params.{}", resolution.id, param.input_name()),
                    type_override,
                )
            });
        let ty = resolve_surface_type(
            override_config,
            param.schema_column_reference(),
            param.type_ref(),
            param.is_nullable(),
            mapping,
            usage,
        );
        resolution.fixed_params.push(ty);
    }
}

fn resolve_direct_repeats(
    dynamic_body: Option<&core::CompiledDynamicQuery>,
    builder_override: Option<&core::BuilderTypeOverrides>,
    mapping: &core::TypeScriptTypeMappingConfig,
    usage: &mut TypeMappingUsage,
    resolution: &mut BuilderTypeMappingResolution,
) {
    let Some(dynamic_body) = dynamic_body else {
        return;
    };

    for repeat in dynamic_body.repeats() {
        let repeat_override = builder_override.and_then(|builder| {
            builder
                .repeats()
                .iter()
                .find(|override_config| override_config.repeat_id() == repeat.id())
        });
        let mut fields = Vec::with_capacity(repeat.fields().len());
        for field in repeat.fields() {
            let override_config = repeat_override
                .and_then(|repeat| find_named_override(repeat.fields(), field.input_name()))
                .map(|type_override| {
                    (
                        format!(
                            "builders.{}.repeats.{}.fields.{}",
                            resolution.id,
                            repeat.id(),
                            field.input_name()
                        ),
                        type_override,
                    )
                });
            let ty = resolve_surface_type(
                override_config,
                field.schema_column_reference(),
                field.type_ref(),
                field.is_nullable(),
                mapping,
                usage,
            );
            fields.push(NamedResolvedType {
                name: field.input_name().to_owned(),
                ty,
            });
        }
        resolution.repeats.push(RepeatTypeMappingResolution {
            id: repeat.id().to_owned(),
            fields,
        });
    }
}

fn unique_param_schema_reference<'a>(
    input_name: &str,
    params: &'a [core::ParamBinding],
) -> Option<&'a core::ColumnTypeReference> {
    let mut references = params
        .iter()
        .filter(|param| param.input_name() == input_name)
        .filter_map(core::ParamBinding::schema_column_reference);
    let first = references.next()?;
    references
        .all(|reference| reference == first)
        .then_some(first)
}
