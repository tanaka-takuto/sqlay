# ADR 0003: Use Hjson `@sqlcomp` Comments

## Status

Accepted

## Context

`sqlcomp` needs metadata that can live inside SQL files without preventing those
files from being read as SQL. The metadata also needs room to grow from `Query` to
future `Param`, `Slot`, and `Fragment` concepts.

## Decision

Use SQL block comments with an `@sqlcomp` marker and an Hjson payload.

The canonical form is:

```sql
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id, name FROM users;
```

## Consequences

- SQL tools should treat `@sqlcomp` metadata as comments.
- Metadata can remain readable as attributes grow.
- The first implementation should validate that the chosen Rust Hjson parser is
  reliable enough for diagnostics and typed deserialization.
- If Hjson parser support is not practical, the project should record a later ADR
  before narrowing the accepted syntax.
