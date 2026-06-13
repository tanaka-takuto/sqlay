# Architecture

`sqlcomp` is implemented as a Rust CLI with a small set of components connected by
explicit intermediate representations. The components are named by responsibility so
their database and target-language dependencies stay visible.

The authoritative product rules live in [Vision](./vision.md), and the current MVP
boundary lives in [MVP](./mvp.md).

## Component Flow

```text
Source Intake
  -> RawQuery

Dialect Analyzer
  RawQuery + dialect rules
  -> AnalyzedQuery

Metadata Provider
  RawQuery.sql + database connection
  -> DbQueryMetadata

Compilation Core
  RawQuery + AnalyzedQuery + DbQueryMetadata
  -> CompiledQuery / Core IR

Target Generator
  CompiledQuery / Core IR
  -> generated files
```

This structure avoids a direct `database dialect x target language` implementation
matrix. Database-specific logic maps database behavior into the Core IR. Target
generators map the Core IR into language-specific code.

## Source Intake

Source Intake reads SQL files and extracts sqlcomp source units. It does not decide
whether the SQL is valid MySQL, PostgreSQL, or another dialect.

Responsibilities:

- read `.sql` files.
- find `@sqlcomp` comments.
- parse Hjson metadata payloads.
- split files into raw query blocks.
- preserve each query block's raw SQL string.

Source Intake is not fully independent from SQL syntax because it must scan SQL
comments and avoid corrupting string literals or comment-like text. However, it
should avoid database semantic decisions. It should produce `RawQuery` values for the
configured dialect analyzer to interpret.

The canonical query annotation form is:

```sql
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id, name FROM users;
```

For the MVP:

- `type: query` is required.
- `id` is required and is never inferred.
- `id` must match `^[A-Za-z_][A-Za-z0-9_]*$`.
- `cardinality` is optional and may override compiler inference.
- one SQL file may contain multiple query annotations.

## Dialect Analyzer

The Dialect Analyzer interprets a `RawQuery` as SQL for one configured database
dialect.

For the MVP, the only dialect analyzer is MySQL 8.x.

Responsibilities:

- parse the raw SQL according to dialect rules.
- reject unsupported statement forms.
- verify that each MVP query block contains exactly one `SELECT` statement.
- infer dialect-dependent query facts such as `LIMIT 1` cardinality.
- produce `AnalyzedQuery` without target-language concerns.

Future PostgreSQL or SQLite support should add new dialect analyzers rather than
branching inside target generators.

## Metadata Provider

The Metadata Provider obtains database metadata for an analyzed query.

For the MVP, the provider connects to MySQL 8.x and derives result column metadata
without executing user data queries. The default Rust database client is `sqlx`,
pending implementation validation. If `sqlx` cannot expose the required MySQL
statement and column metadata, the project should record a follow-up ADR before
changing the client.

Responsibilities:

- connect to the configured database.
- describe a query without fetching user data.
- return database-native column names, database types, and nullability metadata.

See also:

- [ADR 0001: Use MySQL 8.x as the MVP dialect](./adr/0001-use-mysql-8-for-mvp.md)
- [ADR 0003: Use Hjson `@sqlcomp` comments](./adr/0003-use-hjson-sqlcomp-comments.md)

## Compilation Core

Compilation Core is the main pure component. It combines source metadata, dialect
analysis, and database metadata into a language-neutral Core IR.

IR means intermediate representation: an internal data structure that is no longer
raw SQL input, but is not yet TypeScript, Go, Rust, or any other generated language.

Example Core IR shape:

```rust
struct CompiledQuery {
    id: QueryId,
    sql: String,
    cardinality: Cardinality,
    input: Vec<InputField>,
    row: Vec<ResultColumn>,
}

struct ResultColumn {
    name: String,
    ty: CoreType,
    nullable: bool,
}

enum CoreType {
    Bool,
    Int32,
    Int64,
    Float64,
    Decimal,
    String,
    Bytes,
    Date,
    DateTime,
    Json,
    Unknown,
}
```

Database-specific type mapping should stop at Core IR:

```text
MySQL BIGINT -> CoreType::Int64
PostgreSQL int8 -> CoreType::Int64
```

Target-language type mapping should start from Core IR:

```text
CoreType::Int64 -> TypeScript string
CoreType::Int64 -> Go int64
```

This keeps MySQL-to-TypeScript, PostgreSQL-to-TypeScript, MySQL-to-Go, and
PostgreSQL-to-Go from becoming separate hard-coded paths.

Core metadata should be conservative:

- database nullability metadata is used when available.
- unknown nullability maps to nullable output.
- precision-sensitive types such as `BIGINT`, `DECIMAL`, and date/time values should
  avoid lossy JavaScript conversions in the MVP target generator.

## Target Generator

Target Generators convert Core IR into generated files for a target language. They
should not parse or reinterpret database-specific SQL syntax. The SQL text inside a
generated file may be MySQL or another dialect, but the generator treats that SQL as
validated text carried by the Core IR.

The MVP target generator emits TypeScript SQL builder code. Generated code returns
SQL text and parameter arrays, not database execution behavior.

Generated TypeScript is emitted per SQL file while preserving the input-root-relative
directory structure. If one SQL file contains multiple queries, the corresponding
TypeScript file contains multiple generated query functions and type aliases.

Generated names are not case-converted. The query `id` is used exactly as written,
with fixed suffixes for generated TypeScript types:

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

Generated SQL must be emitted as a valid JavaScript string literal. The TypeScript
target generator should escape the SQL text rather than embedding raw SQL in an
unescaped template literal, because MySQL backtick identifiers and SQL text
containing `${...}` must not break generated TypeScript. MVP examples use ordinary
double-quoted string literals; multiline SQL may use any representation that is
semantically equivalent after JavaScript string escaping.

See also:

- [ADR 0002: Use TypeScript SQL builders as the first target generator](./adr/0002-use-typescript-target-generator-first.md)
- [ADR 0005: Do not automatically transform generated names](./adr/0005-do-not-transform-generated-names.md)
