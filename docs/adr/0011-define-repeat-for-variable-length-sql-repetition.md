# ADR 0011: Define Repeat for Variable-Length SQL Repetition

## Status

Accepted

## Context

`sqlay` supports typed SQL builder generation with inline `Param` value binding and
initial `Slot`/`Fragment` composition. ADR 0010 defines initial MySQL mutation
builders, but deliberately keeps variable-length SQL repetition out of the initial
mutation rollout.

Bulk `VALUES` rows and dynamic `IN` lists need one shared mechanism that repeats a
SQL item template and its Params based on a caller-provided array. Treating this as
a mutation-only bulk insert feature would miss SELECT use cases such as dynamic
`IN` filters. Reusing Slot multi-select would not model input-array-driven
repetition.

The feature must preserve sqlay's 2-way SQL philosophy: source SQL remains readable
and executable with representative sample values, and `@sqlay` comments describe
compiler behavior without turning the source into a custom SQL DSL.

## Decision

Add inline `Repeat` markers that define one variable-length list item template.
Repeat support applies to SELECT query builders, mutation builders, and fragments
used by query or mutation slots.

### Source Syntax

A Repeat range starts with `type: repeat` and ends with `type: repeatEnd`.
Initial Repeat metadata accepts only `type`, `id`, and `separator`.
`repeatEnd` accepts only `type`.

```sql
AND u.id IN (
  /* @sqlay { type: repeat id: ids separator: "," } */
  /* @sqlay { type: param id: id valueType: int64 } */
  1
  /* @sqlay { type: paramEnd } */
  /* @sqlay { type: repeatEnd } */
)
```

The Repeat range encloses exactly one list item template. The surrounding list
syntax, such as `IN (` and `)`, or the `VALUES` keyword and statement terminator,
stays outside the Repeat range. The compiler inserts `separator` only between
expanded items, never before the first item or after the last item.

For bulk `VALUES`, the Repeat range encloses one row tuple:

```sql
/* @sqlay
{
  type: mutation
  id: createUsers
}
*/
INSERT INTO users (email, name)
VALUES
/* @sqlay { type: repeat id: rows separator: "," } */
(
  /* @sqlay { type: param id: email } */
  'ada@example.test'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: name } */
  'Ada'
  /* @sqlay { type: paramEnd } */
)
/* @sqlay { type: repeatEnd } */;
```

`separator` is a required string literal and is inserted as raw SQL text. sqlay does
not infer commas, whitespace, or newlines.

Repeat marker pairing is strict:

- every `repeat` marker must have a matching `repeatEnd`.
- top-level `repeatEnd` markers are rejected.
- nested Repeat ranges are rejected.
- Repeat ranges may contain Param ranges.
- Param ranges may not contain Repeat markers.
- Repeat ranges may not contain Slot markers.

Repeat ranges are allowed inside query bodies, mutation bodies, and fragment
bodies. A Fragment may therefore contain a dynamic `IN` list and still be selected
by a Slot. A Repeat range must contain at least one Param marker. Param-less Repeat
ranges are rejected because the initial feature is input-array-driven repetition,
not count-based duplication of fixed SQL text.

Generated SQL removes `@sqlay` comments. Ordinary SQL comments inside a Repeat item
template are preserved and repeated as part of the item SQL text.

### Input Shape and Namespaces

Each unique Repeat ID contributes one generated input field whose value is a
non-empty readonly array of item objects. Repeat item types are not exported as
separate aliases; they are rendered inline inside the builder input type.

```ts
export type findUsers_Input = {
  ids: readonly [{ id: string }, ...{ id: string }[]];
};
```

Even a Repeat item with one Param uses an item object rather than a scalar array.
This keeps dynamic `IN` lists and bulk row tuples under one stable API rule.

Within one query or mutation source unit, direct Param IDs, Slot IDs, and Repeat
IDs share the generated top-level input namespace and must not collide. Repeat item
Param IDs live in that Repeat item's nested namespace, so an outer direct Param
`id` may coexist with `ids[number].id`.

Within a selected Slot branch, Fragment direct Param IDs and Fragment Repeat IDs
share that branch object's top-level namespace and must not collide. Fragment
Repeat item Param IDs live inside the Repeat item namespace.

Generated input field order follows first-seen input-producing marker order in the
source unit. Repeat item field order follows Param first-seen order in the first
Repeat occurrence that defines the input shape.

### Repeated Repeat IDs

Repeated Repeat IDs are allowed within the same query or mutation source unit. They
share one generated array input when their item shapes are compatible.

The first occurrence fixes the generated item field order. Later occurrences with
the same Repeat ID must use the same Param ID set with matching CoreType and
nullability. Later occurrences may use different SQL text, a different separator,
and a different Param occurrence order.

Each Repeat occurrence emits params in that occurrence's SQL placeholder order.
Input field order is an API presentation rule; it does not drive params order.

The same rule applies inside Fragment branches by Slot input identity. Repeated
occurrences of the same Slot ID share the same branch input and therefore share the
same Fragment Repeat inputs. Different Slot IDs that target the same Fragment have
separate branch inputs, because different Slot IDs represent different caller
intent. The same Fragment Repeat may therefore infer different types in different
Slot contexts.

### Empty Arrays and Runtime Checks

Repeat arrays are non-empty. sqlay never emits `IN ()`, empty `VALUES`, or an empty
repetition.

Generated TypeScript uses a non-empty readonly array type and also performs a
runtime guard so JavaScript callers or `any` casts cannot generate invalid empty
SQL:

```ts
if (input.ids.length === 0) {
  throw new Error("Repeat `ids` requires at least one item");
}
```

Each unique Repeat input is checked once before it is expanded. For Repeat inputs
inside optional Slot branches, the empty-array check runs only when the branch is
selected and emitted.

Initial Repeat support does not define `maxItems`. Very large inputs remain caller
responsibility; database or driver limits such as too many placeholders are allowed
to surface outside the generated builder.

### Expansion and Validation

`check` and `compile` validate Repeat source units by expanding each Repeat
occurrence to a two-item representative SQL form. This validates the separator as
well as the repeated item template:

```text
IN (?, ?)
VALUES (?, ?), (?, ?)
```

Repeat item Param type inference runs on the two-item representative SQL using the
existing direct column-context inference rules. All occurrences that feed the same
Repeat item field must agree on CoreType and nullability. If inference is not
available in the expanded SQL context, the Param requires `valueType` as usual.

Repeat representative expansion is combined with Slot expansion during validation.
Each Slot selection variant is validated with Repeat ranges expanded to the
two-item representative form. Repeat does not create user-selectable SQL shape
choices.

The existing 256 Slot variant limit is generalized to a validation case count
limit:

```text
validation_case_count = slot_variant_count * repeat_representative_case_count
```

Initial Repeat support has one representative case, the two-item form, so this is
equivalent to the current Slot variant count. If a later ADR adds one-item plus
two-item validation, both representative cases count toward the same 256 validation
case limit.

Diagnostics involving Repeat should include the Repeat ID and the owning query or
mutation ID. When Repeat is reached through Slot/Fragment expansion, diagnostics
should also include the Slot ID and Fragment ID where that context explains the
failure.

### Generated TypeScript

Builders that contain Repeat return variable-length params:

```ts
params: readonly SqlParam[]
```

This applies even when the builder has no Slot, because Repeat input length changes
the number of placeholders at runtime. A generated TypeScript file that contains at
least one Slot or Repeat builder emits one private file-level helper alias:

```ts
type SqlParam = unknown;
```

Generated SQL assembly uses the same dynamic style as Slot builders: append SQL
segments to `sqlParts` and params to `params` in SQL emission order. Repeat
expansion uses a loop and emits the separator before every item after the first.

```ts
let idsIndex = 0;
for (const idsItem of input.ids) {
  if (idsIndex > 0) {
    sqlParts.push(",");
  }

  sqlParts.push("?");
  params.push(idsItem.id);

  idsIndex += 1;
}
```

Builders with Repeat inputs require the normal input object parameter. sqlay does
not generate a direct array argument or an empty default input for Repeat builders.

### CLI and Summaries

`check` and `compile` summaries include Repeat counts alongside Param, Slot, and
validation case counts. Aggregate summaries should report total Repeat count, and
per-query or per-mutation summaries should report the Repeat count for that
builder. Runtime item counts are not known during `check` or `compile` and are not
reported.

## Consequences

Repeat can be implemented in focused stages:

- parse `repeat` and `repeatEnd` inline markers and validate pairing rules.
- extend raw source-unit representations with Repeat occurrences and nested Repeat
  item Param usages.
- validate input namespace collisions for direct Params, Slots, and Repeats.
- expand Repeat ranges to two-item representative SQL for query and mutation
  validation.
- preserve Slot expansion behavior while applying Repeat representative expansion
  inside each validation case.
- extend Core IR for dynamic Repeat body generation without merging SELECT and
  mutation IR.
- update TypeScript generation for non-empty Repeat input arrays, runtime guards,
  loop-based SQL assembly, and `readonly SqlParam[]` params.
- add valid and invalid fixtures for dynamic `IN` lists, bulk `VALUES` rows,
  Fragment-contained Repeats, repeated Repeat IDs, namespace collisions, empty-array
  guards, and diagnostic contexts.
- update docs and examples for dynamic `IN` lists and bulk `VALUES` rows.

Repeat keeps generated code driver-independent. It only constructs SQL text and
params; it does not execute SQL, batch database calls, infer inserted IDs, or add
transaction helpers.

## Out of Scope

Initial Repeat support does not include:

- empty-array fallback SQL.
- `maxItems`, `minItems`, or project-level length limits.
- scalar array inputs for single-Param Repeat items.
- exported Repeat item type aliases.
- nested Repeat ranges.
- Slot markers inside Repeat ranges.
- Param-less count-based SQL duplication.
- generated database execution functions or batch execution helpers.
- automatic separator inference, SQL formatting, or whitespace normalization.

## Alternatives Considered

Treat Repeat as a mutation-only bulk insert feature. This was rejected because
dynamic SELECT `IN` lists need the same input-array-driven SQL repetition model.

Reuse Slot multi-select for repeated SQL. This was rejected because Slot selection
models optional SQL shape choices, while Repeat models a runtime input array whose
length determines emitted SQL and params.

Allow arbitrary free-form Repeat ranges including Slots. This was rejected for the
initial design because Slot selection inside each array item would make input shape,
validation cases, and runtime assembly substantially more complex. The initial
range is the list item template only.

Allow scalar arrays for single-Param Repeat items. This was rejected because it
would create a special API case and make later additions to the item template
breaking.

Generate fallback SQL for empty arrays. This was rejected because `IN ()`, empty
`VALUES`, and caller-specific fallback semantics should not be inferred by sqlay.
Callers should branch before invoking the builder when they need different behavior
for empty input.

Require `maxItems`. This was rejected for the initial design because sqlay should
not guess per-query operational limits. Large generated SQL and database placeholder
limits remain caller responsibility.

## See Also

- [ADR 0008: Define SELECT Param Support](./0008-define-select-param-support.md)
- [ADR 0009: Define Initial SELECT Slot/Fragment Support](./0009-define-initial-select-slot-fragment-support.md)
- [ADR 0010: Define Initial MySQL Mutation Builder Support](./0010-define-initial-mysql-mutation-builder-support.md)
