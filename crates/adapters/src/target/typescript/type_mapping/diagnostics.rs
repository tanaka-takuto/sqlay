use std::collections::BTreeMap;

use sqlay_core as core;

use super::model::{BuilderTypeMappingResolution, ResolvedTypeScriptType};
use super::{column_reference_key, core_type_key};

pub(super) fn push_unused_override_diagnostics(
    mapping: &core::TypeScriptTypeMappingConfig,
    builders: &[core::CompiledBuilder],
    usage: &TypeMappingUsage,
    diagnostics: &mut Vec<core::Diagnostic>,
) {
    for core_override in mapping.core() {
        let path = format!("core.{}", core_type_key(core_override.core_type()));
        if !usage.is_used(&path) {
            diagnostics.push(core::Diagnostic::error(format!(
                "unused TypeScript type mapping override `{path}`; no generated field, Param, or Repeat item used that Core type without a narrower override"
            )));
        }
    }

    for column_override in mapping.columns() {
        let path = format!(
            "columns.{}",
            column_reference_key(column_override.reference())
        );
        if !usage.is_used(&path) {
            diagnostics.push(core::Diagnostic::error(format!(
                "unused TypeScript type mapping override `{path}`; no generated field, Param, or Repeat item resolved to that schema column"
            )));
        }
    }

    for builder_override in mapping.builders() {
        let Some(builder) = builders
            .iter()
            .find(|builder| builder.id() == builder_override.builder_id())
        else {
            continue;
        };
        let surface = BuilderSurface::from_builder(builder);

        for field in builder_override.fields() {
            let path = format!(
                "builders.{}.fields.{}",
                builder_override.builder_id(),
                field.name()
            );
            if !usage.is_used(&path) {
                diagnostics.push(core::Diagnostic::error(format!(
                    "unused TypeScript type mapping override `{path}`; no result field with that name exists on builder `{}`",
                    builder_override.builder_id()
                )));
            }
        }

        for param in builder_override.params() {
            let path = format!(
                "builders.{}.params.{}",
                builder_override.builder_id(),
                param.name()
            );
            if !usage.is_used(&path) {
                diagnostics.push(core::Diagnostic::error(format!(
                    "unused TypeScript type mapping override `{path}`; no direct Param input with that name exists on builder `{}`",
                    builder_override.builder_id()
                )));
            }
        }

        for repeat in builder_override.repeats() {
            let repeat_path = format!(
                "builders.{}.repeats.{}",
                builder_override.builder_id(),
                repeat.repeat_id()
            );
            let Some(surface_fields) = surface
                .repeats
                .iter()
                .find(|surface_repeat| surface_repeat.id == repeat.repeat_id())
            else {
                diagnostics.push(core::Diagnostic::error(format!(
                    "unused TypeScript type mapping override `{repeat_path}`; no direct Repeat input with that id exists on builder `{}`",
                    builder_override.builder_id()
                )));
                continue;
            };

            for field in repeat.fields() {
                let path = format!("{repeat_path}.fields.{}", field.name());
                if !usage.is_used(&path) {
                    let reason = if surface_fields
                        .fields
                        .iter()
                        .any(|field_name| field_name == field.name())
                    {
                        "that Repeat field was not selected by type mapping resolution"
                    } else {
                        "no Repeat item field with that name exists"
                    };
                    diagnostics.push(core::Diagnostic::error(format!(
                        "unused TypeScript type mapping override `{path}`; {reason} on builder `{}` Repeat `{}`",
                        builder_override.builder_id(),
                        repeat.repeat_id()
                    )));
                }
            }
        }
    }
}

pub(super) fn push_import_conflict_diagnostics(
    builders: &[BuilderTypeMappingResolution],
    diagnostics: &mut Vec<core::Diagnostic>,
) {
    let mut imports_by_source = BTreeMap::<String, BTreeMap<String, String>>::new();

    for builder in builders {
        let source_path = builder
            .source_path
            .clone()
            .unwrap_or_else(|| "<unknown>".to_owned());
        for resolved_type in builder.resolved_types() {
            let Some(import) = resolved_type.import.as_ref() else {
                continue;
            };
            let imports = imports_by_source.entry(source_path.clone()).or_default();
            if let Some(previous_from) = imports.get(import.name()) {
                if previous_from != import.from() {
                    diagnostics.push(core::Diagnostic::error(format!(
                        "TypeScript type import conflict in source file `{source_path}`: local type `{}` is imported from both `{previous_from}` and `{}`",
                        import.name(),
                        import.from()
                    )));
                }
            } else {
                imports.insert(import.name().to_owned(), import.from().to_owned());
            }
        }
    }
}

#[derive(Default)]
pub(super) struct TypeMappingUsage {
    paths: Vec<String>,
}

impl TypeMappingUsage {
    pub(super) fn mark(&mut self, path: String) {
        if !self.paths.iter().any(|used| used == &path) {
            self.paths.push(path);
        }
    }

    fn is_used(&self, path: &str) -> bool {
        self.paths.iter().any(|used| used == path)
    }
}

impl BuilderTypeMappingResolution {
    fn resolved_types(&self) -> Vec<&ResolvedTypeScriptType> {
        self.fields
            .iter()
            .map(|field| &field.ty)
            .chain(self.inputs.iter().map(|input| &input.ty))
            .chain(self.fixed_params.iter())
            .chain(
                self.repeats
                    .iter()
                    .flat_map(|repeat| repeat.fields.iter().map(|field| &field.ty)),
            )
            .chain(
                self.slot_branches
                    .iter()
                    .flat_map(|branch| branch.params.iter().map(|param| &param.ty)),
            )
            .chain(self.slot_branches.iter().flat_map(|branch| {
                branch
                    .repeats
                    .iter()
                    .flat_map(|repeat| repeat.fields.iter().map(|field| &field.ty))
            }))
            .collect()
    }
}

struct BuilderSurface {
    repeats: Vec<RepeatSurface>,
}

impl BuilderSurface {
    fn from_builder(builder: &core::CompiledBuilder) -> Self {
        let repeats = match builder {
            core::CompiledBuilder::Query(query) => query
                .dynamic_body()
                .map_or(Vec::new(), direct_repeat_surfaces),
            core::CompiledBuilder::Mutation(mutation) => mutation
                .dynamic_body()
                .map_or(Vec::new(), direct_repeat_surfaces),
        };

        Self { repeats }
    }
}

struct RepeatSurface {
    id: String,
    fields: Vec<String>,
}

fn direct_repeat_surfaces(dynamic_body: &core::CompiledDynamicQuery) -> Vec<RepeatSurface> {
    dynamic_body
        .repeats()
        .iter()
        .map(|repeat| RepeatSurface {
            id: repeat.id().to_owned(),
            fields: repeat
                .fields()
                .iter()
                .map(|field| field.input_name().to_owned())
                .collect(),
        })
        .collect()
}
