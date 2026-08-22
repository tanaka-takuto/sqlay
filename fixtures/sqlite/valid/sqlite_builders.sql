/* @sqlay
{
  type: fragment
  id: sqliteByStatus
}
*/
  AND o.status = /* @sqlay { type: param id: status } */ 'paid' /* @sqlay { type: paramEnd } */

/* @sqlay
{
  type: fragment
  id: sqliteByOrderId
}
*/
  AND fixture_order_items.order_id = /* @sqlay { type: param id: orderId } */ 10 /* @sqlay { type: paramEnd } */

/* @sqlay
{
  type: query
  id: sqliteListOrders
}
*/
SELECT
  o.id AS id,
  o.status AS status,
  o.note AS note,
  o.total AS total
FROM fixture_orders AS o
WHERE o.id >= /* @sqlay { type: param id: minId } */ 1 /* @sqlay { type: paramEnd } */
/* @sqlay { type: slot id: statusFilter targets: [sqliteByStatus] } */
ORDER BY o.id;

/* @sqlay
{
  type: query
  id: sqliteFindOrdersByIds
}
*/
SELECT
  o.id AS id,
  o.customer_email AS customerEmail,
  o.active AS active
FROM fixture_orders AS o
WHERE o.id IN (
  /* @sqlay { type: repeat id: ids separator: ", " } */
  /* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */
  /* @sqlay { type: repeatEnd } */
)
ORDER BY o.id;

/* @sqlay
{
  type: mutation
  id: sqliteInsertOrder
}
*/
INSERT INTO fixture_orders (
  id,
  customer_email,
  status,
  note,
  total,
  active
) VALUES (
  /* @sqlay { type: param id: id } */ 10 /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: customerEmail } */ 'buyer@example.test' /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: status } */ 'paid' /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: note nullable: true } */ 'gift' /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: total } */ 42.5 /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: active } */ TRUE /* @sqlay { type: paramEnd } */
);

/* @sqlay
{
  type: mutation
  id: sqliteUpdateOrder
}
*/
UPDATE fixture_orders
SET status = /* @sqlay { type: param id: status } */ 'shipped' /* @sqlay { type: paramEnd } */
WHERE fixture_orders.id = /* @sqlay { type: param id: id } */ 10 /* @sqlay { type: paramEnd } */;

/* @sqlay
{
  type: mutation
  id: sqliteDeleteOrderItem
}
*/
DELETE FROM fixture_order_items
WHERE fixture_order_items.id = /* @sqlay { type: param id: id } */ 100 /* @sqlay { type: paramEnd } */
/* @sqlay { type: slot id: orderFilter targets: [sqliteByOrderId] } */;

/* @sqlay
{
  type: mutation
  id: sqliteReplaceOrder
}
*/
REPLACE INTO fixture_orders (
  id,
  customer_email,
  status,
  note,
  total,
  active
) VALUES (
  /* @sqlay { type: param id: id } */ 10 /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: customerEmail } */ 'replacement@example.test' /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: status } */ 'paid' /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: note nullable: true } */ 'replacement' /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: total } */ 84.5 /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: active } */ TRUE /* @sqlay { type: paramEnd } */
);

/* @sqlay
{
  type: mutation
  id: sqliteBulkInsertOrderItems
}
*/
INSERT INTO fixture_order_items (
  id,
  order_id,
  sku,
  quantity
) VALUES
/* @sqlay { type: repeat id: items separator: ", " } */
(
  /* @sqlay { type: param id: id } */ 100 /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: orderId } */ 10 /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: sku } */ 'SKU-001' /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: quantity } */ 2 /* @sqlay { type: paramEnd } */
)
/* @sqlay { type: repeatEnd } */;
