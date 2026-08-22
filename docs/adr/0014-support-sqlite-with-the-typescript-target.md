# ADR 0014: Support SQLite with the TypeScript Target

## Status

Accepted

## Context

`sqlay` currently supports MySQL 8.x analysis and metadata extraction with
driver-independent TypeScript SQL builder generation. The architecture already
separates database-specific analysis and metadata adapters from language-neutral
Core IR and target-language generation, so a second database dialect should extend
those boundaries instead of creating a SQLite-specific TypeScript generator.

SQLite differs materially from MySQL. It is an embedded database, its connection
URL identifies a local file, its declared types use affinity rather than enforcing a
single runtime value type, and expression type and nullability metadata may be
unknown. Its mutation syntax also overlaps with MySQL without being interchangeable.
The initial SQLite boundary therefore needs explicit connection, statement,
metadata, and type-mapping rules.

## Decision

### Version and Connection Boundary

Support SQLite 3.35.0 and later through the SQLite implementation bundled by the
`sqlx` dependency used by `sqlay`.

SQLite projects keep the existing configuration contract:

```jsonc
{
  "database": {
    "dialect": "sqlite",
    "urlEnv": "DATABASE_URL",
  },
  "target": {
    "language": "typescript",
  },
}
```

`database.urlEnv` names the process environment variable that contains the SQLite
file URL. The CLI does not load `.env` files implicitly.

The initial accepted URLs are file-backed SQLite URLs:

- `sqlite://relative/path.db` resolves the path relative to the process working
  directory.
- `sqlite:///absolute/path.db` names an absolute path.

The resolved path must identify an existing regular SQLite database file. `check`
and `compile` do not create a missing database or parent directory. Connection and
path diagnostics identify the configured environment variable name but do not print
the URL value.

In-memory databases, temporary databases, attached databases, non-`main` schemas,
and SQLCipher or other encrypted SQLite variants are outside the initial boundary.
Only objects in the file's `main` schema participate in analysis and metadata
resolution.

### Statement Surface

SQLite query source units use the existing `type: query` annotation and must contain
exactly one SQLite `SELECT` statement ending in `;`.

SQLite mutation source units use the existing `type: mutation` annotation. Initial
support accepts exactly one of these forms:

- `INSERT ... VALUES`, including multiple value rows.
- single-table `UPDATE ... SET ... WHERE ...`.
- single-table `DELETE ... WHERE ...`.
- `REPLACE ... VALUES`, including multiple value rows.

`UPDATE` and `DELETE` require a `WHERE` clause in every expanded validation case.
The compiler checks for the presence of `WHERE`; it does not prove that the
predicate is selective. Mutation SQL is parsed and inspected but is never executed
during `check` or `compile`.

The initial SQLite surface excludes:

- `RETURNING` on any mutation.
- SQLite UPSERT forms using `ON CONFLICT`.
- `INSERT ... SELECT` and `REPLACE ... SELECT`.
- top-level CTE mutation forms.
- `UPDATE ... FROM`.
- `UPDATE` or `DELETE` extensions using `ORDER BY` or `LIMIT`.
- `INSERT OR ...` conflict-clause variants, including `INSERT OR REPLACE`.
- `DEFAULT VALUES`.
- multi-table or multi-statement source units.
- DDL, `ATTACH`, `DETACH`, `PRAGMA`, transaction control, and administrative
  statements as generated builders.

Unsupported forms produce SQLite-specific actionable diagnostics. `sqlay` does not
translate MySQL syntax to SQLite syntax, translate SQLite syntax to MySQL syntax, or
rewrite a statement into a supported form.

### Param, Slot, Fragment, and Repeat Behavior

SQLite uses the existing `Param`, `Slot`, `Fragment`, and `Repeat` source and Core IR
contracts.

Each paired Param range is replaced with one positional `?` placeholder for SQLite
analysis, metadata lookup, and generated SQL. Raw placeholders in source SQL remain
unsupported. Param input order and generated params order follow the existing source
and expanded-SQL rules.

The initial optional single-select Slot model is unchanged. Every SQLite query Slot
variant must preserve query cardinality and result row shape. Every SQLite mutation
Slot variant must remain a supported statement of the same mutation kind and must
preserve the `WHERE` requirement for `UPDATE` and `DELETE`.

Repeat ranges keep the existing non-empty input and two-item representative
validation rules. Repeat expansion is validated together with every Slot selection
case. SQLite does not add dialect-specific separator inference, empty-array SQL, or
SQL normalization.

### Metadata and Core Type Mapping

SQLite query metadata uses `sqlx` prepare/describe behavior backed by its bundled
SQLite implementation. It does not fetch application rows. Schema-backed metadata
uses inspection of real tables and columns in the `main` schema. SQLite mutation
metadata prepares every expanded statement on the existing read-only connection so
SQLite itself validates syntax and name resolution. Preparing a mutation does not
step or execute it. Param inference continues to use only `main` schema inspection.

Direct schema column identity may drive existing Param inference and TypeScript type
mapping overrides. The SQLite forms are:

- `table.column` for a table in `main`.
- `main.table.column` for an explicitly qualified table in `main`.

Other schema qualifiers are rejected for SQLite projects.

SQLite declared types are mapped conservatively into Core metadata:

- integer declarations map to `CoreType::Int64`.
- real, float, and double declarations map to `CoreType::Float64`.
- text, character, CLOB, and varchar declarations map to `CoreType::String`.
- blob declarations map to `CoreType::Bytes`.
- explicit boolean declarations map to `CoreType::Unknown` for result columns
  because SQLite stores them as integers and does not enforce `0` or `1`.
- declarations with numeric or decimal affinity, date/time declarations, JSON
  declarations, missing declarations, and unrecognized declarations map to
  `CoreType::Unknown` initially because SQLite does not guarantee one compatible
  runtime representation for them.

An expression result type reported unambiguously by SQLite describe metadata may map
to the corresponding Core type. A missing, ambiguous, or conflicting declared or
expression type maps to `CoreType::Unknown`; sqlay does not guess from a sample Param
literal. Params without supported direct-column inference require an explicit inline
`valueType` as in the existing workflow.

SQLite boolean Params use an explicit inline `valueType: bool`. Their generated
TypeScript input remains `boolean`, while Core IR carries a language-neutral
boolean-to-integer Param encoding. The TypeScript builder applies that encoding at
runtime and returns `0 | 1` in the ordered params array (`null` is preserved for a
nullable Param). This is SQLite value binding normalization, not a driver-specific
execution dependency. The same rule applies to direct, Slot/Fragment, and Repeat
Params. MySQL Bool Params keep their existing identity encoding.

Result nullability is non-null only when describe and schema context establish it
for the expanded query. Unknown or conflicting nullability is nullable. In
particular, schema `NOT NULL` alone must not make an outer-join result overconfident.
SQLite primary-key and expression nullability are treated as unknown unless the
metadata path proves the result is non-null.

SQLite-specific declared types, affinity rules, schema lookup results, nullability
decisions, and Param value encodings stop at the database adapter and Core IR. The
TypeScript target generator consumes the same language-neutral Core metadata used
for MySQL and does not branch on SQLite itself.

### Generated TypeScript

SQLite uses the existing TypeScript target generator without SQLite-specific SQL
parsing or metadata branches. Generated builders return SQL text and ordered params
only. They do not execute SQLite, import a SQLite package, create database files,
manage transactions, or parse result values. The target generator renders the
language-neutral Param encoding from Core IR; for SQLite boolean Params this keeps
the public input as `boolean` while binding `false` as `0` and `true` as `1`.

Generated SQL preserves the accepted source SQL semantics and existing Param,
Slot/Fragment, and Repeat emission order. Generated names use source IDs exactly as
written and receive no automatic dialect or naming transformation.

Configured TypeScript type mapping overrides remain annotations only. They may
narrow an intentionally conservative SQLite Core type for a project whose runtime
driver contract is known, but they do not add result conversion or validation and
do not change a Param encoding already carried by Core IR.

## Consequences

- Add a SQLite query and mutation analyzer rather than branching in the TypeScript
  generator.
- Add a SQLite `sqlx` metadata adapter for query describe, `main` schema inspection,
  non-executing mutation prepare, direct-column Param inference, declared-type
  mapping, and conservative nullability.
- Select the analyzer and metadata provider from `database.dialect` at the CLI
  composition root while keeping application ports dialect-neutral.
- Configuration validation and CLI help must describe both `mysql` and `sqlite` and
  the SQLite file URL boundary.
- SQLite examples and fixtures must create their database files before invoking
  `sqlay`, regenerate expected TypeScript through the real compiler, and type-check
  the output with `tsc --noEmit`.
- Unit, integration, and end-to-end tests must cover SQLite configuration, URL
  diagnostics, query and mutation analysis, schema metadata, ambiguous type and
  nullability fallback, dynamic SQL behavior, and generated output.
- Existing MySQL behavior and checks remain unchanged and green.

## Out of Scope

This ADR does not add:

- another target language.
- generated SQLite execution functions or a generated SQLite driver dependency.
- database creation, migrations, schema management, or seed loading.
- in-memory, temporary, attached, encrypted, or non-`main` SQLite databases.
- automatic SQL dialect translation.
- mutation forms excluded by the initial statement surface.
- stronger type or nullability claims for ambiguous SQLite metadata.

## Alternatives Considered

Add a SQLite-specific TypeScript generator. This was rejected because it would
create the database-dialect by target-language implementation matrix that Core IR and
the target-generator boundary are designed to avoid.

Allow SQLite to create a missing file from the configured URL. This was rejected
because `check` and `compile` validate against an existing target schema and are not
schema-management commands.

Support in-memory databases initially. This was rejected because metadata operations
may use multiple connections, while an ordinary in-memory database is scoped to one
connection and would not provide a stable project schema.

Map every SQLite affinity to a precise Core type. This was rejected because SQLite
can store values whose runtime representation differs from the declared affinity.
Ambiguous metadata must remain `Unknown` unless the project supplies an explicit
type hint or TypeScript annotation override.
