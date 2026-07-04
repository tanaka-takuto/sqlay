import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { isDeepStrictEqual } from "node:util";

import mysql, {
  type ExecuteValues,
  type Pool,
  type PoolOptions,
  type RowDataPacket,
} from "mysql2/promise";

import {
  findBookDetail,
  listAvailableBooks,
  listBooksNeedingRestock,
} from "./generated/sql/books";
import {
  listOrderLineItems,
  listRevenueByAuthor,
} from "./generated/sql/orders";

type SqlStatement = {
  sql: string;
  params: readonly unknown[];
};

type AssertionCase = {
  name: string;
  fixture: string;
  run: (pool: Pool, caseName: string) => Promise<unknown>;
};

const exampleRoot = dirname(fileURLToPath(import.meta.url));
const expectedResultsDir = join(exampleRoot, "expected-results");

const assertionCases: AssertionCase[] = [
  {
    name: "available paperback books",
    fixture: "list-available-books-paperback.json",
    run: (pool, caseName) =>
      executeMany(
        caseName,
        pool,
        listAvailableBooks({
          discoveryFilter: { $fragment: "byBookFormat", format: "paperback" },
        }),
      ),
  },
  {
    name: "left hand book detail",
    fixture: "find-book-detail-left-hand.json",
    run: (pool, caseName) =>
      executeOne(caseName, pool, findBookDetail({ isbn: "9780441478125" })),
  },
  {
    name: "missing book detail",
    fixture: "find-book-detail-missing.json",
    run: (pool, caseName) =>
      executeOne(caseName, pool, findBookDetail({ isbn: "9780000000000" })),
  },
  {
    name: "order line items with nullable discounts",
    fixture: "list-order-line-items-bk-1000.json",
    run: (pool, caseName) =>
      executeMany(
        caseName,
        pool,
        listOrderLineItems({ orderNumber: "BK-1000" }),
      ),
  },
  {
    name: "books needing restock",
    fixture: "list-books-needing-restock.json",
    run: (pool, caseName) =>
      executeMany(caseName, pool, listBooksNeedingRestock()),
  },
  {
    name: "revenue by author",
    fixture: "list-revenue-by-author.json",
    run: (pool, caseName) =>
      executeMany(caseName, pool, listRevenueByAuthor()),
  },
];

async function executeMany(
  caseName: string,
  pool: Pool,
  statement: SqlStatement,
): Promise<unknown> {
  const [rows] = await pool.execute<RowDataPacket[]>(
    statement.sql,
    toExecuteValues(caseName, statement.params),
  );
  return normalizeValue(rows);
}

async function executeOne(
  caseName: string,
  pool: Pool,
  statement: SqlStatement,
): Promise<unknown> {
  const [rows] = await pool.execute<RowDataPacket[]>(
    statement.sql,
    toExecuteValues(caseName, statement.params),
  );
  return normalizeValue(rows[0] ?? null);
}

function toExecuteValues(
  caseName: string,
  params: readonly unknown[],
): ExecuteValues[] {
  return params.map((param, index) => {
    if (isExecuteValue(param)) {
      return param;
    }

    throw new Error(
      `Generated query param for ${caseName} at index ${index} is not supported by mysql2: ${typeof param}`,
    );
  });
}

function isExecuteValue(value: unknown): value is ExecuteValues {
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

function normalizeValue(value: unknown): unknown {
  if (value instanceof Date) {
    return value.toISOString();
  }

  if (Buffer.isBuffer(value)) {
    return value.toString("base64");
  }

  if (Array.isArray(value)) {
    return value.map((item) => normalizeValue(item));
  }

  if (value !== null && typeof value === "object") {
    const normalized: Record<string, unknown> = {};
    for (const [key, nested] of Object.entries(value)) {
      normalized[key] = normalizeValue(nested);
    }
    return normalized;
  }

  return value;
}

async function readExpectedResult(fileName: string): Promise<unknown> {
  const content = await readFile(join(expectedResultsDir, fileName), "utf8");
  return JSON.parse(content);
}

function readDatabaseOptions(): PoolOptions {
  const databaseUrl = process.env.DATABASE_URL;
  if (!databaseUrl) {
    throw new Error("DATABASE_URL is required");
  }

  const parsed = new URL(databaseUrl);
  if (parsed.protocol !== "mysql:") {
    throw new Error("DATABASE_URL must use the mysql:// scheme");
  }
  if (!parsed.username || !parsed.password || !parsed.hostname) {
    throw new Error("DATABASE_URL must include user, password, and host");
  }

  const database = parsed.pathname.replace(/^\/+/, "");
  if (!database) {
    throw new Error("DATABASE_URL must include a database name");
  }

  return {
    host: parsed.hostname,
    port: parsed.port ? Number(parsed.port) : 3306,
    user: decodeURIComponent(parsed.username),
    password: decodeURIComponent(parsed.password),
    database: decodeURIComponent(database),
    supportBigNumbers: true,
    bigNumberStrings: true,
    decimalNumbers: true,
    dateStrings: true,
  };
}

async function main(): Promise<void> {
  const pool = mysql.createPool(readDatabaseOptions());

  try {
    for (const assertionCase of assertionCases) {
      const expected = await readExpectedResult(assertionCase.fixture);
      const actual = await assertionCase.run(pool, assertionCase.name);

      if (!isDeepStrictEqual(actual, expected)) {
        throw new Error(
          [
            `Result mismatch for ${assertionCase.name}.`,
            "Expected:",
            JSON.stringify(expected, null, 2),
            "Actual:",
            JSON.stringify(actual, null, 2),
          ].join("\n"),
        );
      }
    }
  } finally {
    await pool.end();
  }
}

main().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`bookstore query result assertions failed: ${message}`);
  process.exitCode = 1;
});
