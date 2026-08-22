import {
  sqliteBulkInsertOrderItems,
  sqliteDeleteOrderItem,
  sqliteFindOrderByMainId,
  sqliteFindOrdersByIds,
  sqliteInsertOrder,
  sqliteListOrderIdsAcrossStatuses,
  sqliteListOrders,
} from "./generated/valid/sqlite_builders";

const listWithoutSlot = sqliteListOrders({ minId: "10" });
assertEqual(
  normalizeSql(listWithoutSlot.sql),
  "SELECT o.id AS id, o.status AS status, o.note AS note, o.total AS total FROM fixture_orders AS o WHERE o.id >= ? ORDER BY o.id;",
);
assertDeepEqual(listWithoutSlot.params, ["10"]);

const listWithSlot = sqliteListOrders({
  minId: "10",
  statusFilter: { $fragment: "sqliteByStatus", status: "paid" },
});
assertEqual(
  normalizeSql(listWithSlot.sql),
  "SELECT o.id AS id, o.status AS status, o.note AS note, o.total AS total FROM fixture_orders AS o WHERE o.id >= ? AND o.status = ? ORDER BY o.id;",
);
assertDeepEqual(listWithSlot.params, ["10", "paid"]);

const repeatedQuery = sqliteFindOrdersByIds({
  ids: [{ id: "10" }, { id: "20" }],
});
assertEqual(
  normalizeSql(repeatedQuery.sql),
  "SELECT o.id AS id, o.customer_email AS customerEmail, o.active AS active FROM fixture_orders AS o WHERE o.id IN ( ? , ? ) ORDER BY o.id;",
);
assertDeepEqual(repeatedQuery.params, ["10", "20"]);

const explicitlyMainQualifiedQuery = sqliteFindOrderByMainId({ id: 10 });
assertEqual(
  normalizeSql(explicitlyMainQualifiedQuery.sql),
  "SELECT o.id AS id, o.status AS status FROM main.fixture_orders AS o WHERE o.id = ?;",
);
assertDeepEqual(explicitlyMainQualifiedQuery.params, [10]);

const compoundQuery = sqliteListOrderIdsAcrossStatuses({
  primaryStatus: "paid",
  fallbackStatus: "shipped",
});
assertEqual(
  normalizeSql(compoundQuery.sql),
  "SELECT o.id AS id FROM fixture_orders AS o WHERE o.status = ? UNION ALL SELECT o.id AS id FROM fixture_orders AS o WHERE o.status = ?;",
);
assertDeepEqual(compoundQuery.params, ["paid", "shipped"]);

const fixedMutation = sqliteInsertOrder({
  id: "10",
  customerEmail: "buyer@example.test",
  status: "paid",
  note: null,
  total: 42.5,
  active: true,
});
assertEqual(
  normalizeSql(fixedMutation.sql),
  "INSERT INTO fixture_orders ( id, customer_email, status, note, total, active ) VALUES ( ?, ?, ?, ?, ?, ? );",
);
assertDeepEqual(fixedMutation.params, [
  "10",
  "buyer@example.test",
  "paid",
  null,
  42.5,
  true,
]);

const mutationWithSlot = sqliteDeleteOrderItem({
  id: "100",
  orderFilter: { $fragment: "sqliteByOrderId", orderId: "10" },
});
assertEqual(
  normalizeSql(mutationWithSlot.sql),
  "DELETE FROM fixture_order_items WHERE fixture_order_items.id = ? AND fixture_order_items.order_id = ? ;",
);
assertDeepEqual(mutationWithSlot.params, ["100", "10"]);

const repeatedMutation = sqliteBulkInsertOrderItems({
  items: [
    { id: "100", orderId: "10", sku: "SKU-001", quantity: "2" },
    { id: "101", orderId: "10", sku: "SKU-002", quantity: "3" },
  ],
});
assertEqual(
  normalizeSql(repeatedMutation.sql),
  "INSERT INTO fixture_order_items ( id, order_id, sku, quantity ) VALUES ( ?, ?, ?, ? ) , ( ?, ?, ?, ? ) ;",
);
assertDeepEqual(repeatedMutation.params, [
  "100",
  "10",
  "SKU-001",
  "2",
  "101",
  "10",
  "SKU-002",
  "3",
]);

function normalizeSql(sql: string): string {
  return sql.replace(/\s+/g, " ").trim();
}

function assertEqual(actual: unknown, expected: unknown): void {
  if (actual !== expected) {
    throw new Error(`Expected ${String(expected)}, got ${String(actual)}`);
  }
}

function assertDeepEqual(actual: unknown, expected: unknown): void {
  const actualJson = JSON.stringify(actual);
  const expectedJson = JSON.stringify(expected);
  if (actualJson !== expectedJson) {
    throw new Error(`Expected ${expectedJson}, got ${actualJson}`);
  }
}
