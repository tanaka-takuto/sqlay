PRAGMA foreign_keys = ON;

DROP TABLE IF EXISTS fixture_order_items;
DROP TABLE IF EXISTS fixture_orders;

CREATE TABLE fixture_orders (
  id INTEGER PRIMARY KEY,
  customer_email TEXT NOT NULL,
  status TEXT NOT NULL,
  note TEXT,
  total REAL NOT NULL,
  active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE fixture_order_items (
  id INTEGER PRIMARY KEY,
  order_id INTEGER NOT NULL,
  sku TEXT NOT NULL,
  quantity INTEGER NOT NULL,
  FOREIGN KEY (order_id) REFERENCES fixture_orders (id)
);
