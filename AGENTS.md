# AGENTS.md

## Source of Truth

Treat `docs/` as the authoritative source for product, architecture, and MVP
decisions.

Before making design-sensitive changes, read:

- `docs/vision.md`
- `docs/architecture.md`
- `docs/mvp.md`
- relevant files in `docs/adr/`

Do not duplicate detailed design rules in this file. If a product or architecture
decision changes, update the relevant document or add a new ADR.

## Current MVP Boundary

The current MVP is Query-only, SELECT-only, MySQL 8.x, and TypeScript SQL builder
generation. `Param`, `Slot`, `Fragment`, non-SELECT statements, and additional target
generators are future work unless a later ADR changes that scope.

## Development Notes

- Keep generated behavior explicit and predictable.
- Do not add automatic naming transformations without an ADR.
- Run the relevant checks for changed files.
- Markdown, JSON, and YAML files are formatted with dprint.
