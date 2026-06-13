# Vision

`sqlcomp` is SQL Compose & Compile.

It is a CLI tool for writing plain SQL files while gaining compile-time type safety
and predictable query composition for statically typed languages.

## Tagline

Write Pure SQL. Feel Type Safety. No Magic.

## Core Philosophy

### 2-Way SQL

SQL files should remain usable as SQL. A developer should be able to copy or open a
query in a normal database tool and understand what will run without first
understanding generated code.

`@sqlcomp` metadata is carried in SQL comments. The metadata may guide compilation,
but it must not require the SQL text to become a custom DSL.

### Explicit Design

`sqlcomp` should prefer explicit user intent over implicit compiler behavior.

The compiler must not silently rewrite SQL structure, replace table aliases, infer
public API names, or apply language-specific naming conventions. If a name matters
to generated code, the user should choose a suitable name in the source query.

### Static Type Safety

Generated code should represent database result metadata in the target language's
type system. For the MVP, this means generating TypeScript types for MySQL `SELECT`
result rows.

When metadata is uncertain, generated types should be conservative rather than
overconfident. For example, unknown nullability should be treated as nullable.

### Minimal Runtime Surface

Generated code should have a small runtime surface. The MVP generates SQL builder
functions that return SQL text and parameters. It does not execute queries or require
a database driver in generated TypeScript code.

### Flat Result Mapping

Rows are mapped directly to language-level object types. The MVP does not generate
nested object graphs or ORM-style models.

## Non-Goals for the MVP

The MVP does not implement dynamic SQL composition, `Param`, `Slot`, or `Fragment`.
Those concepts remain part of the long-term design, but the first implementation is
limited to query metadata, result type extraction, and TypeScript SQL builder output.

Future `Slot` design should use `targets: [...]` rather than a single `target`, so
exclusive choices and single choices can share one representation.
