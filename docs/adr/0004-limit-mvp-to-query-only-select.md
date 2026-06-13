# ADR 0004: Limit the MVP to Query-only SELECT Support

## Status

Accepted

## Context

The long-term design includes `Query`, `Param`, `Slot`, and `Fragment`. Implementing
all dynamic SQL composition concepts at once would increase the initial design and
testing surface before the compile pipeline is proven.

## Decision

The MVP supports `Query` only, and each query block must contain exactly one MySQL
`SELECT` statement ending in `;`.

`Param`, `Slot`, `Fragment`, and non-`SELECT` statements are outside the MVP.

## Consequences

- The MVP focuses on SQL discovery, MySQL metadata extraction, result type mapping,
  and TypeScript SQL builder generation.
- `cardinality: exec` is reserved but rejected until non-`SELECT` support exists.
- Dynamic SQL design can evolve after the basic compile path is working.
- Future `Slot` design should use `targets: [...]` to support both single and
  exclusive choices.
