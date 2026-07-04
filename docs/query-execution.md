# Query Execution with mysql2

SELECT query builders return SQL text, params, and generated TypeScript row/output
types. They do not execute queries, manage connections, load `.env` files, or
depend on a database driver. Application code chooses a driver and executes the
builder output.

The example below uses the generated `findBookDetail` and `listAvailableBooks`
builders from
[`examples/bookstore/sql/books.sql`](../examples/bookstore/sql/books.sql) with
`mysql2/promise`.

## SELECT Execution

Read the same environment variable configured by `database.urlEnv`, pass the
generated SQL and params to `mysql2`, and keep row typing tied to the generated
builder types. The bookstore example config uses `DATABASE_URL`; pass a different
name when your project sets a different `database.urlEnv` value.

```ts
import mysql, {
  type ExecuteValues,
  type Pool,
  type RowDataPacket,
} from "mysql2/promise";
import {
  findBookDetail,
  type findBookDetail_Input,
  type findBookDetail_Output,
  type findBookDetail_Row,
  listAvailableBooks,
  type listAvailableBooks_Output,
  type listAvailableBooks_Row,
} from "../examples/bookstore/generated/sql/books";

type FindBookDetailMysqlRow = findBookDetail_Row & RowDataPacket;
type ListAvailableBooksMysqlRow = listAvailableBooks_Row & RowDataPacket;

function readDatabaseUrl(envName: string): string {
  const databaseUrl = process.env[envName];
  if (!databaseUrl) {
    throw new Error(`${envName} is required`);
  }
  return databaseUrl;
}

function toMysqlExecuteValues(
  statementName: string,
  params: readonly unknown[],
): ExecuteValues[] {
  return params.map((param, index) => {
    if (isMysqlExecuteValue(param)) {
      return param;
    }

    throw new Error(
      `Parameter ${index} for ${statementName} is not supported by mysql2: ${typeof param}`,
    );
  });
}

function isMysqlExecuteValue(value: unknown): value is ExecuteValues {
  return (
    value === null ||
    typeof value === "string" ||
    typeof value === "number" ||
    typeof value === "bigint" ||
    typeof value === "boolean" ||
    value instanceof Date ||
    Buffer.isBuffer(value) ||
    value instanceof Uint8Array
  );
}

async function loadBookDetail(
  pool: Pool,
  input: findBookDetail_Input,
): Promise<findBookDetail_Output> {
  const statement = findBookDetail(input);
  const [rows] = await pool.execute<FindBookDetailMysqlRow[]>(
    statement.sql,
    toMysqlExecuteValues("findBookDetail", statement.params),
  );
  return rows[0] ?? null;
}

async function loadAvailableBooksByIds(
  pool: Pool,
): Promise<listAvailableBooks_Output> {
  const statement = listAvailableBooks({
    discoveryFilter: {
      $fragment: "byBookIds",
      ids: [{ id: "100" }, { id: "102" }],
    },
  });
  const [rows] = await pool.execute<ListAvailableBooksMysqlRow[]>(
    statement.sql,
    toMysqlExecuteValues("listAvailableBooks", statement.params),
  );
  return rows;
}

async function main(): Promise<void> {
  const pool = mysql.createPool(readDatabaseUrl("DATABASE_URL"));

  try {
    const book = await loadBookDetail(pool, { isbn: "9780441478125" });
    console.log(book?.title ?? "not found");
    const availableBooks = await loadAvailableBooksByIds(pool);
    console.log(`available books: ${availableBooks.length}`);
  } finally {
    await pool.end();
  }
}
```

`mysql2` accepts `ExecuteValues[]` in its TypeScript surface. Slot, Fragment, and
Repeat builders can return `readonly unknown[]` because selected SQL branches can
change the runtime parameter shape. Narrow those params at the application driver
boundary instead of casting them to `any[]` or weakening generated sqlay types.
The helper above accepts the value types `mysql2` can execute and reports the
statement name plus parameter index when an unsupported value reaches the driver
boundary.

The row aliases combine generated row types with `RowDataPacket`, which lets
`mysql2` type the returned rows without replacing the generated sqlay types with
`any` or a hand-written duplicate row shape. Use the same pattern with each
generated `<builder>_Row` and return rows as the generated `<builder>_Output`
type.

Do not print database URLs, params that may contain secrets, or full connection
diagnostics in normal logs. The example above reports only the missing environment
variable name when configuration is absent.
