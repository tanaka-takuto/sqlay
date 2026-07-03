use sqlay_core as core;

use crate::compile::diagnostics::{mutation_param_usage_error, param_usage_error};
use crate::compile::param_validation::ScopedParamBinding;
use crate::compile::slot_variants::ExpandedParamOccurrence;

pub(in crate::compile::param_validation) fn param_schema_column_reference_conflict_error(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    existing: &ScopedParamBinding,
    later_reference: Option<&core::ColumnTypeReference>,
    later_occurrence: &ExpandedParamOccurrence,
) -> core::DiagnosticReport {
    let first_reference = format_schema_column_reference(existing.schema_column_reference.as_ref());
    let later_reference = format_schema_column_reference(later_reference);
    if let Some((first, later)) =
        super::repeat_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return super::repeat_param_conflict_error(
            query,
            usage,
            first,
            later,
            format!(
                "conflicting Repeat item Param `{}` schema column reference in query `{}`, Repeat `{}`: first representative occurrence resolved from {} but conflicting representative occurrence resolved from {}; Repeat item fields with the same ID must resolve matching Param type, nullability, and schema column reference",
                usage.id(),
                query.metadata().id(),
                first.repeat_id,
                first_reference,
                later_reference,
            ),
        );
    }
    if let Some((first, later)) =
        super::fragment_repeat_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return super::fragment_repeat_param_conflict_error(
            query,
            usage,
            first,
            later,
            format!(
                "conflicting Fragment Repeat item Param `{}` schema column reference in query `{}`, Slot `{}`, Fragment `{}`, Repeat `{}`: first representative occurrence resolved from {} but conflicting representative occurrence resolved from {}; Repeat item fields with the same ID must resolve matching Param type, nullability, and schema column reference",
                usage.id(),
                query.metadata().id(),
                first.slot_id,
                first.target_id,
                first.repeat_id,
                first_reference,
                later_reference,
            ),
        );
    }
    if let Some((first, later)) =
        super::repeated_fragment_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return super::repeated_fragment_param_conflict_error(
            query,
            usage,
            first,
            later,
            format!(
                "conflicting Fragment Param `{}` schema column reference in query `{}`, Slot `{}`, Fragment `{}`: occurrence {} resolved from {} but occurrence {} resolved from {}; repeated Slot occurrences that select the same Fragment must resolve matching Param type, nullability, and schema column reference",
                usage.id(),
                query.metadata().id(),
                first.slot_id,
                first.target_id,
                first.slot_occurrence_index,
                first_reference,
                later.slot_occurrence_index,
                later_reference,
            ),
        );
    }

    param_usage_error(
        query,
        usage,
        format!(
            "conflicting Param `{}` schema column references: first occurrence resolved from {} but later occurrence resolved from {}",
            usage.id(),
            first_reference,
            later_reference,
        ),
    )
}

pub(in crate::compile::param_validation) fn mutation_param_schema_column_reference_conflict_error(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
    existing: &ScopedParamBinding,
    later_reference: Option<&core::ColumnTypeReference>,
    later_occurrence: &ExpandedParamOccurrence,
) -> core::DiagnosticReport {
    let first_reference = format_schema_column_reference(existing.schema_column_reference.as_ref());
    let later_reference = format_schema_column_reference(later_reference);
    if let Some((first, later)) =
        super::repeat_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return super::mutation_repeat_param_conflict_error(
            mutation,
            usage,
            first,
            later,
            format!(
                "conflicting Repeat item Param `{}` schema column reference in mutation `{}`, Repeat `{}`: first representative occurrence resolved from {} but conflicting representative occurrence resolved from {}; Repeat item fields with the same ID must resolve matching Param type, nullability, and schema column reference",
                usage.id(),
                mutation.metadata().id(),
                first.repeat_id,
                first_reference,
                later_reference,
            ),
        );
    }
    if let Some((first, later)) =
        super::fragment_repeat_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return super::mutation_fragment_repeat_param_conflict_error(
            mutation,
            usage,
            first,
            later,
            format!(
                "conflicting Fragment Repeat item Param `{}` schema column reference in mutation `{}`, Slot `{}`, Fragment `{}`, Repeat `{}`: first representative occurrence resolved from {} but conflicting representative occurrence resolved from {}; Repeat item fields with the same ID must resolve matching Param type, nullability, and schema column reference",
                usage.id(),
                mutation.metadata().id(),
                first.slot_id,
                first.target_id,
                first.repeat_id,
                first_reference,
                later_reference,
            ),
        );
    }
    if let Some((first, later)) =
        super::repeated_fragment_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return super::repeated_fragment_mutation_param_conflict_error(
            mutation,
            usage,
            first,
            later,
            format!(
                "conflicting Fragment Param `{}` schema column reference in mutation `{}`, Slot `{}`, Fragment `{}`: occurrence {} resolved from {} but occurrence {} resolved from {}; repeated Slot occurrences that select the same Fragment must resolve matching Param type, nullability, and schema column reference",
                usage.id(),
                mutation.metadata().id(),
                first.slot_id,
                first.target_id,
                first.slot_occurrence_index,
                first_reference,
                later.slot_occurrence_index,
                later_reference,
            ),
        );
    }

    mutation_param_usage_error(
        mutation,
        usage,
        format!(
            "conflicting Param `{}` schema column references: first occurrence resolved from {} but later occurrence resolved from {}",
            usage.id(),
            first_reference,
            later_reference,
        ),
    )
}

fn format_schema_column_reference(reference: Option<&core::ColumnTypeReference>) -> String {
    reference.map_or_else(
        || "no schema column reference".to_owned(),
        |reference| {
            reference.database().map_or_else(
                || format!("{}.{}", reference.table(), reference.column()),
                |database| format!("{}.{}.{}", database, reference.table(), reference.column()),
            )
        },
    )
}
