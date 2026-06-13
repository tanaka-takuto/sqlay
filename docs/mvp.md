# MVP

The first `sqlcomp` implementation is intentionally small. It should prove the full
compile path from SQL files to typed TypeScript SQL builders without implementing
dynamic query composition.

## Scope

The MVP supports:

- MySQL 8.x.
- TypeScript generation.
- query annotations with Hjson `@sqlcomp` comments.
- `SELECT` statements only.
- one or more queries per SQL file.
- exactly one SQL statement per query block.
- output TypeScript files generated per SQL file while preserving input-relative
  directory structure.

The MVP does not support:

- `INSERT`, `UPDATE`, `DELETE`, DDL, `CALL`, or multi-statement query blocks.
- `Param`, `Slot`, or `Fragment`.
- generated database execution functions.
- automatic naming transformation.
- non-MySQL dialects.

## Query Blocks

Each query starts with a `type: query` annotation. The SQL body continues until the
next `type: query` annotation or the end of the file.

Each query body must contain exactly one `SELECT` statement and must end with `;`.

```sql
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id, name FROM users;

/* @sqlcomp
{
  type: query
  id: findLatestUser
}
*/
SELECT id, name FROM users ORDER BY id DESC LIMIT 1;
```

## Query Metadata

`id` is required. It is never inferred from the file name, SQL text, or output path.

Valid IDs must match:

```text
^[A-Za-z_][A-Za-z0-9_]*$
```

`cardinality` is optional.

MVP cardinality inference:

- `SELECT ... LIMIT 1` infers `one`.
- other `SELECT` statements infer `many`.
- `one` means `Row | null`.
- `many` means `Row[]`.
- `exec` is reserved for future non-SELECT support and must be rejected in the MVP.

An explicit `cardinality` value overrides inference when the value is supported by
the MVP.

## Generated TypeScript

Generated TypeScript uses the query `id` exactly as written. It does not convert
between camelCase, PascalCase, or snake_case.

For `id: listUsers`, generated symbols are:

- `listUsers`
- `listUsers_Input`
- `listUsers_Row`
- `listUsers_Output`

Generated functions return SQL builder data:

```ts
export type listUsers_Input = Record<string, never>;

export type listUsers_Row = {
  id: number;
  name: string | null;
};

export type listUsers_Output = listUsers_Row[];

export function listUsers(
  _input: listUsers_Input = {},
): { sql: string; params: readonly [] } {
  return {
    sql: "SELECT id, name FROM users;",
    params: [] as const,
  };
}
```

The `input` parameter exists to keep the public shape stable for future `Param`
support. In the MVP, generated functions should name the unused parameter `_input`
so projects with `noUnusedParameters` enabled can type-check generated code. Query
inputs are always `Record<string, never>`, and `params` is always an empty readonly
tuple.

Generated SQL must be emitted as a valid JavaScript string literal. The generator
must escape SQL text instead of copying raw SQL into an unescaped template literal,
because valid MySQL SQL may contain backtick identifiers or `${...}` text that would
otherwise break generated TypeScript.

## Acceptance Scenarios

The implementation should cover these scenarios:

- multiple queries in one `.sql` file are generated into one corresponding `.ts`
  file.
- duplicate query IDs are rejected.
- invalid query IDs are rejected.
- non-`SELECT` statements are rejected.
- query blocks with multiple SQL statements are rejected.
- `LIMIT 1` infers `one`.
- ordinary `SELECT` infers `many`.
- explicit `cardinality` overrides inference.
- `cardinality: exec` is rejected in the MVP.
- MySQL nullable metadata maps to `T | null`.
- unknown nullability maps to `T | null`.

See also:

- [ADR 0004: Limit the MVP to Query-only SELECT support](./adr/0004-limit-mvp-to-query-only-select.md)
