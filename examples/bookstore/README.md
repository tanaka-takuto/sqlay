# Bookstore Example

This example is a small online bookstore project. It demonstrates realistic MySQL
queries, mutation builders, Param input binding, Slot/Fragment composition, Repeat
lists, configurable TypeScript type mapping, and the TypeScript builders generated
by `sqlay`.

`sql/book_filters.sql` contains reusable global fragments. It is a fragment-only
source file, so it contributes SQL to the queries in `sql/books.sql` without
generating `generated/sql/book_filters.ts`.

`sql/books.sql` uses a query-local `discoveryFilter` slot with `targets` for a
Param-less staff-picks filter, a format filter whose `format` Param is nested under
the selected `$fragment` branch in generated TypeScript, and a `byBookIds` fragment
that demonstrates Repeat inside a Slot-selected fragment.

`sql/mutations.sql` contains user-facing mutation examples for creating an order,
loading the created row with an explicit SELECT builder, handling update/delete
affected row counts, creating multiple order items with a Repeat bulk `VALUES`
row, upserting by a stable order number, and documenting `REPLACE` as a
MySQL-specific operation with delete-plus-insert semantics.

`sqlay.config.json` maps sqlay's Core `decimal` type to TypeScript `number` as an
explicit project choice. This changes generated type annotations only; application
code is still responsible for configuring its database driver and accepting any
decimal precision tradeoff at runtime.

The generated files under `generated/` are committed expected artifacts. They are
regenerated and compared byte for byte by the examples check.

`seed.sql` is deterministic and intentionally includes production-readiness
boundary cases: cancelled orders, a paid order with no items, repeated sort keys,
discount and no-discount line items, zero and large prices, a large `BIGINT`
identifier, long text, nullable-column combinations, and JSON metadata with nested
objects, arrays, numbers, booleans, missing keys, and JSON null values.

See [`../../docs/query-execution.md`](../../docs/query-execution.md) for a minimal
`mysql2/promise` SELECT execution example, and
[`../../docs/mutation-execution.md`](../../docs/mutation-execution.md) for mutation
execution examples that use these generated builders without adding
driver-specific code to the generated TypeScript.
