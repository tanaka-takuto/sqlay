# Mutation Execution with mysql2

Mutation builders return SQL text and params. They do not execute statements,
manage transactions, or wrap driver result objects. Application code chooses a
driver and executes the builder output.

The examples below use generated builders from
[`examples/bookstore/sql/mutations.sql`](../examples/bookstore/sql/mutations.sql)
with `mysql2/promise`.

## Shared Helper

Use a small application-side helper to narrow generated params before handing them
to `mysql2`:

```ts
import type {
  ExecuteValues,
  Pool,
  PoolConnection,
  ResultSetHeader,
  RowDataPacket,
} from "mysql2/promise";
import {
  createOrder,
  type createOrder_Input,
  createOrderItems,
  deleteUnapprovedReview,
  findOrderById,
  type findOrderById_Output,
  findOrderByNumber,
  type findOrderByNumber_Output,
  markOrderPaid,
  replaceCategory,
  upsertOrderStatus,
} from "../examples/bookstore/generated/sql/mutations";

type BuiltStatement = {
  sql: string;
  params: readonly unknown[];
};

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

async function executeMutation(
  connection: PoolConnection,
  statementName: string,
  statement: BuiltStatement,
): Promise<ResultSetHeader> {
  const [result] = await connection.execute<ResultSetHeader>(
    statement.sql,
    toMysqlExecuteValues(statementName, statement.params),
  );
  return result;
}

async function loadOrderById(
  connection: PoolConnection,
  orderId: string,
): Promise<findOrderById_Output> {
  const statement = findOrderById({ orderId });
  const [rows] = await connection.execute<RowDataPacket[]>(
    statement.sql,
    toMysqlExecuteValues("findOrderById", statement.params),
  );
  return (rows[0] ?? null) as findOrderById_Output;
}

async function loadOrderByNumber(
  connection: PoolConnection,
  orderNumber: string,
): Promise<findOrderByNumber_Output> {
  const statement = findOrderByNumber({ orderNumber });
  const [rows] = await connection.execute<RowDataPacket[]>(
    statement.sql,
    toMysqlExecuteValues("findOrderByNumber", statement.params),
  );
  return (rows[0] ?? null) as findOrderByNumber_Output;
}
```

`mysql2` accepts `ExecuteValues[]` in its TypeScript surface. Slot, Fragment, and
Repeat builders can return `readonly unknown[]` because selected SQL branches or
input array lengths can change the runtime parameter shape. Narrow those params at
the application driver boundary instead of casting them to `any[]` or weakening
generated sqlay types. The helper above accepts the value types `mysql2` can
execute and reports the statement name plus parameter index when an unsupported
value reaches the driver boundary.

The row casts above are separate application-boundary choices. sqlay can generate
the SELECT row type, but the database driver still owns the runtime row object.

## Single-Row Insert

Read `insertId` from the driver result for a single-row insert when the table uses
an auto-increment key. If the application needs the created row, execute an
explicit SELECT builder after the insert:

```ts
async function createOrderAndLoad(
  connection: PoolConnection,
  input: createOrder_Input,
): Promise<findOrderById_Output> {
  const insertResult = await executeMutation(
    connection,
    "createOrder",
    createOrder(input),
  );
  const createdOrderId = String(insertResult.insertId);
  return loadOrderById(connection, createdOrderId);
}
```

Generated mutation builders intentionally do not return row objects. Selecting the
row is explicit user code, so the transaction boundary and isolation behavior stay
visible.

## Affected Rows

For `UPDATE` and `DELETE`, inspect the driver's affected row count and decide what
your application considers success:

```ts
async function payDraftOrder(
  connection: PoolConnection,
  orderNumber: string,
): Promise<void> {
  const result = await executeMutation(
    connection,
    "markOrderPaid",
    markOrderPaid({
      orderNumber,
      paidAt: "2026-04-20 12:01:00.000000",
    }),
  );

  if (result.affectedRows !== 1) {
    throw new Error(`Expected to pay one draft order, paid ${result.affectedRows}`);
  }
}

async function deletePendingReview(
  connection: PoolConnection,
  reviewId: string,
): Promise<boolean> {
  const result = await executeMutation(
    connection,
    "deleteUnapprovedReview",
    deleteUnapprovedReview({ reviewId }),
  );
  return result.affectedRows === 1;
}
```

## Transactions

Use the same `mysql2` connection for every builder executed inside a transaction:

```ts
async function createPaidOrder(pool: Pool): Promise<findOrderById_Output> {
  const connection = await pool.getConnection();

  try {
    await connection.beginTransaction();

    const orderResult = await executeMutation(
      connection,
      "createOrder",
      createOrder({
        customerId: "1000",
        orderNumber: "BK-2000",
        status: "draft",
        currency: "USD",
        placedAt: "2026-04-20 12:00:00.000000",
        paidAt: null,
        shippedAt: null,
        shippingMethod: "priority",
        giftMessage: null,
      }),
    );

    const orderId = String(orderResult.insertId);

    await executeMutation(
      connection,
      "createOrderItems",
      createOrderItems({
        items: [
          {
            orderId,
            bookId: "100",
            quantity: 1,
            unitPrice: 16.99,
            discountAmount: null,
          },
          {
            orderId,
            bookId: "102",
            quantity: 1,
            unitPrice: 18,
            discountAmount: 2,
          },
        ],
      }),
    );

    await payDraftOrder(connection, "BK-2000");
    const createdOrder = await loadOrderById(connection, orderId);

    await connection.commit();
    return createdOrder;
  } catch (error) {
    await connection.rollback();
    throw error;
  } finally {
    connection.release();
  }
}
```

sqlay does not generate transaction helpers because connection ownership, retry
policy, isolation level, and error mapping belong to the application.

## Multi-Row Inserts

Do not derive per-row IDs with `insertId + index` after a multi-row insert:

```ts
const result = await executeMutation(
  connection,
  "createOrderItems",
  createOrderItems({
    items: [
      {
        orderId: "5004",
        bookId: "100",
        quantity: 1,
        unitPrice: 16.99,
        discountAmount: null,
      },
      {
        orderId: "5004",
        bookId: "102",
        quantity: 1,
        unitPrice: 18,
        discountAmount: 2,
      },
    ],
  }),
);

void result.insertId;
```

`insertId` is not a robust list of generated row IDs. When the application needs
the inserted rows after a multi-row insert, use application-generated unique keys
or natural keys in the inserted data, then issue an explicit SELECT by those stable
keys.

## Upserts

Do not use `insertId` or affected row counts as the official classifier for
`INSERT ... ON DUPLICATE KEY UPDATE`. MySQL and driver settings can make those
values surprising. If the caller needs the final row, select it by a stable unique
key:

```ts
async function upsertOrderAndLoad(
  connection: PoolConnection,
): Promise<findOrderByNumber_Output> {
  await executeMutation(
    connection,
    "upsertOrderStatus",
    upsertOrderStatus({
      customerId: "1000",
      orderNumber: "BK-2001",
      initialStatus: "draft",
      currency: "USD",
      placedAt: "2026-04-20 12:00:00.000000",
      nextStatus: "paid",
      paidAt: "2026-04-20 12:01:00.000000",
    }),
  );

  return loadOrderByNumber(connection, "BK-2001");
}
```

## REPLACE

`REPLACE` is supported as MySQL SQL, but it is not a general-purpose update
replacement. MySQL may delete an existing row and insert a new one. That can affect
foreign keys, triggers, auto-increment values, and affected row counts:

```ts
await executeMutation(
  connection,
  "replaceCategory",
  replaceCategory({
    categoryId: "13",
    slug: "staff-picks",
    displayName: "Staff Picks",
  }),
);
```

Prefer `UPDATE` or `INSERT ... ON DUPLICATE KEY UPDATE` when the application wants
ordinary update semantics. Use `REPLACE` only when its delete-plus-insert behavior
is intentional.
