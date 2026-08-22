PRAGMA foreign_keys = ON;

DROP TABLE IF EXISTS sqlite_fixture_order_items;
DROP TABLE IF EXISTS sqlite_fixture_orders;

CREATE TABLE sqlite_fixture_orders (
  id INTEGER PRIMARY KEY,
  customer_email TEXT NOT NULL,
  status TEXT NOT NULL,
  note TEXT,
  total REAL NOT NULL,
  active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE sqlite_fixture_order_items (
  id INTEGER PRIMARY KEY,
  order_id INTEGER NOT NULL,
  sku TEXT NOT NULL,
  quantity INTEGER NOT NULL,
  FOREIGN KEY (order_id) REFERENCES sqlite_fixture_orders (id)
);
