# sqlcomp

SQL Compose & Compile.

`sqlcomp` is a Rust CLI for writing plain SQL files while generating typed target
language builders. The current MVP focuses on MySQL 8.x `SELECT` queries and
TypeScript SQL builder generation.

See [`docs/vision.md`](./docs/vision.md),
[`docs/architecture.md`](./docs/architecture.md), and
[`docs/mvp.md`](./docs/mvp.md) for the product, architecture, and MVP decisions.

## MVP Usage

Create the starter project configuration from the directory that should contain
`sqlcomp.config.json`:

```sh
sqlcomp init
```

`sqlcomp init` writes a starter `sqlcomp.config.json` and refuses to overwrite an
existing config file. The starter config uses the MVP shape:

```json
{
  "source": {
    "include": ["sql/**/*.sql"],
    "exclude": []
  },
  "output": {
    "dir": "src/generated/sqlcomp"
  },
  "database": {
    "dialect": "mysql",
    "urlEnv": "DATABASE_URL"
  },
  "target": {
    "language": "typescript"
  }
}
```

The config is parsed as JSON with comments and trailing commas allowed. Source and
output paths are resolved relative to the directory containing
`sqlcomp.config.json`.

Run a database-backed dry run with:

```sh
DATABASE_URL='mysql://user:password@host:3306/database' sqlcomp check
```

`sqlcomp check` loads the config, reads SQL files, validates MySQL 8.x `SELECT`
queries, looks up MySQL metadata, and builds generated TypeScript in memory without
writing files. The database URL is read from the process environment variable named
by `database.urlEnv`; the CLI does not implicitly load `.env` files.

Write generated TypeScript with:

```sh
DATABASE_URL='mysql://user:password@host:3306/database' sqlcomp compile
```

`sqlcomp compile` runs the same pipeline as `check` and writes TypeScript SQL
builder files under `output.dir`. Generated paths preserve each input SQL path
relative to `sqlcomp.config.json`; for example, `sql/books.sql` generates
`src/generated/sqlcomp/sql/books.ts` with the starter config. Normal `compile`
overwrites same-path generated files. Use `sqlcomp compile --clean` to also remove
stale managed generated files.

Generated TypeScript includes a generated-code header, empty MVP input types,
database-backed row and output types, and builder functions that return SQL text and
an empty readonly `params` tuple. Generated code does not execute queries or depend
on a database driver.

## Local MySQL

The repository includes a Docker Compose service for local MySQL 8.x development:

```sh
script/mysql-up.sh
```

The service uses deterministic development-only credentials and waits for the
container healthcheck before the command exits.

Use this connection URL for local checks:

```sh
export DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp'
```

The local MySQL service starts an empty development database. Example and fixture
checks load their own prefix-scoped schema and seed data each time they run. Reset
the database volume with:

```sh
script/mysql-reset.sh
```

Stop the service without removing the database volume with:

```sh
script/mysql-down.sh
```

## Local Checks

Run the same non-database baseline checks used by CI with:

```sh
npm ci
script/check-baseline.sh
```

Type-check committed generated TypeScript artifacts directly with:

```sh
npm run typecheck:examples
npm run typecheck:fixtures
```

Run the MySQL-backed example E2E check against a running MySQL service with:

```sh
DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' script/check-examples.sh
```

Run the MySQL-backed fixture checks with:

```sh
DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' script/check-mysql-fixtures.sh
```
