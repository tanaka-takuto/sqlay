use sqlay_core as core;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::target::typescript) struct TypeMappingResolution {
    pub(super) builders: Vec<BuilderTypeMappingResolution>,
}

impl TypeMappingResolution {
    pub(super) const fn new(builders: Vec<BuilderTypeMappingResolution>) -> Self {
        Self { builders }
    }

    pub(in crate::target::typescript) fn builder(
        &self,
        id: &str,
    ) -> Option<&BuilderTypeMappingResolution> {
        self.builders.iter().find(|builder| builder.id == id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::target::typescript) struct BuilderTypeMappingResolution {
    pub(super) id: String,
    pub(super) source_path: Option<String>,
    pub(super) fields: Vec<NamedResolvedType>,
    pub(super) inputs: Vec<NamedResolvedType>,
    pub(super) fixed_params: Vec<ResolvedTypeScriptType>,
    pub(super) repeats: Vec<RepeatTypeMappingResolution>,
    pub(super) slot_branches: Vec<SlotBranchTypeMappingResolution>,
    pub(super) dynamic_params_annotation: Option<String>,
}

impl BuilderTypeMappingResolution {
    pub(super) const fn new(id: String, source_path: Option<String>) -> Self {
        Self {
            id,
            source_path,
            fields: Vec::new(),
            inputs: Vec::new(),
            fixed_params: Vec::new(),
            repeats: Vec::new(),
            slot_branches: Vec::new(),
            dynamic_params_annotation: None,
        }
    }

    pub(in crate::target::typescript) fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.ty.annotation.as_str())
    }

    pub(in crate::target::typescript) fn input(&self, name: &str) -> Option<&str> {
        self.inputs
            .iter()
            .find(|input| input.name == name)
            .map(|input| input.ty.annotation.as_str())
    }

    pub(in crate::target::typescript) fn fixed_param(&self, index: usize) -> Option<&str> {
        self.fixed_params
            .get(index)
            .map(|param| param.annotation.as_str())
    }

    pub(in crate::target::typescript) fn repeat_field(
        &self,
        repeat_id: &str,
        field_name: &str,
    ) -> Option<&str> {
        self.repeats
            .iter()
            .find(|repeat| repeat.id == repeat_id)
            .and_then(|repeat| {
                repeat
                    .fields
                    .iter()
                    .find(|field| field.name == field_name)
                    .map(|field| field.ty.annotation.as_str())
            })
    }

    pub(in crate::target::typescript) fn dynamic_params_annotation(&self) -> Option<&str> {
        self.dynamic_params_annotation.as_deref()
    }

    pub(in crate::target::typescript) fn slot_branch_param(
        &self,
        slot_id: &str,
        target_id: &str,
        param_name: &str,
    ) -> Option<&str> {
        self.slot_branches
            .iter()
            .find(|branch| branch.slot_id == slot_id && branch.target_id == target_id)
            .and_then(|branch| {
                branch
                    .params
                    .iter()
                    .find(|param| param.name == param_name)
                    .map(|param| param.ty.annotation.as_str())
            })
    }

    pub(in crate::target::typescript) fn slot_branch_repeat_field(
        &self,
        slot_id: &str,
        target_id: &str,
        repeat_id: &str,
        field_name: &str,
    ) -> Option<&str> {
        self.slot_branches
            .iter()
            .find(|branch| branch.slot_id == slot_id && branch.target_id == target_id)
            .and_then(|branch| {
                branch
                    .repeats
                    .iter()
                    .find(|repeat| repeat.id == repeat_id)
                    .and_then(|repeat| {
                        repeat
                            .fields
                            .iter()
                            .find(|field| field.name == field_name)
                            .map(|field| field.ty.annotation.as_str())
                    })
            })
    }

    pub(in crate::target::typescript) fn imports(&self) -> Vec<&core::TypeScriptTypeImport> {
        self.resolved_types()
            .into_iter()
            .filter_map(|resolved_type| resolved_type.import.as_ref())
            .collect()
    }

    pub(in crate::target::typescript::type_mapping) fn resolved_types(
        &self,
    ) -> Vec<&ResolvedTypeScriptType> {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::target::typescript) struct RepeatTypeMappingResolution {
    pub(super) id: String,
    pub(super) fields: Vec<NamedResolvedType>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::target::typescript) struct SlotBranchTypeMappingResolution {
    pub(super) slot_id: String,
    pub(super) target_id: String,
    pub(super) params: Vec<NamedResolvedType>,
    pub(super) repeats: Vec<RepeatTypeMappingResolution>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::target::typescript) struct NamedResolvedType {
    pub(super) name: String,
    pub(super) ty: ResolvedTypeScriptType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::target::typescript) struct ResolvedTypeScriptType {
    pub(super) annotation: String,
    pub(super) import: Option<core::TypeScriptTypeImport>,
}

impl ResolvedTypeScriptType {
    pub(super) const fn new(
        annotation: String,
        import: Option<core::TypeScriptTypeImport>,
    ) -> Self {
        Self { annotation, import }
    }
}
