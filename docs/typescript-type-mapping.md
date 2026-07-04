# TypeScript Type Mapping

`sqlay` maps MySQL metadata to conservative TypeScript annotations by default. Projects can use
`target.typescript.typeMapping` when those annotations should match a local domain model or a known
database-driver configuration.

Type mapping overrides are static TypeScript annotations only. They do not parse SELECT result
values, validate inputs at runtime, configure `mysql2`, execute SQL, or change generated SQL text.
Application code remains responsible for database-driver options and runtime conversion.

For the accepted design record, see
[ADR 0012](./adr/0012-define-configurable-typescript-type-mapping-overrides.md).

## Configuration Shape

Put overrides under `target.typescript.typeMapping`:

```jsonc
{
  "target": {
    "language": "typescript",
    "typescript": {
      "typeMapping": {
        "core": {
          "decimal": "number",
          "int64": "number",
        },
        "columns": {
          "orders.total_amount": {
            "type": "MoneyAmount",
            "import": {
              "from": "@/domain/money",
              "name": "MoneyAmount",
            },
          },
          "billing.orders.status": "BillingOrderStatus",
        },
        "builders": {
          "listOrders": {
            "fields": {
              "totalAmount": "MoneyAmount",
            },
            "params": {
              "minimumAmount": {
                "type": "MoneyAmount",
                "import": {
                  "from": "@/domain/money",
                  "name": "MoneyAmount",
                },
              },
            },
            "repeats": {
              "lineItems": {
                "fields": {
                  "unitPrice": "MoneyAmount",
                },
              },
            },
          },
        },
      },
    },
  },
}
```

Override targets:

- `core.<core-type>` changes the broad mapping for a sqlay Core type such as `decimal` or `int64`.
- `columns.<table.column>` targets a schema-backed column in the current database.
- `columns.<database.table.column>` targets a schema-backed column in an explicit MySQL database.
- `builders.<id>.fields.<field>` targets a generated SELECT result row field.
- `builders.<id>.params.<param>` targets a direct Param input field and that Param's fixed params
  tuple entries.
- `builders.<id>.repeats.<repeat>.fields.<field>` targets one direct Repeat item field.

Overrides use this priority, from narrowest to broadest:

1. Builder-local overrides.
2. Column overrides.
3. Schema-backed MySQL `ENUM` literal union defaults.
4. Core type overrides.
5. sqlay's built-in conservative mapping.

Unknown builders, fields, Params, Repeats, schema columns, and Core types are configuration errors.
Unused overrides are also errors, so stale config does not silently stop matching generated code.

## Override Values

Use a string shorthand when the type name is already available without an import:

```jsonc
{
  "target": {
    "language": "typescript",
    "typescript": {
      "typeMapping": {
        "columns": {
          "orders.status": "OrderStatus",
        },
      },
    },
  },
}
```

Use an object when generated files should import the type:

```jsonc
{
  "target": {
    "language": "typescript",
    "typescript": {
      "typeMapping": {
        "columns": {
          "orders.total_amount": {
            "type": "MoneyAmount",
            "import": {
              "from": "@/domain/money",
              "name": "MoneyAmount",
            },
          },
        },
      },
    },
  },
}
```

Generated imports are type-only:

```ts
import type { MoneyAmount } from "@/domain/money";
```

`import.from` must be a non-relative module specifier such as `@/domain/money` or
`@acme/domain-types`. Relative paths such as `./money` and `../money` are rejected because generated
files preserve SQL source directory structure, so one relative path cannot be correct for every
generated file.

The configured `type` must be a supported TypeScript primitive or a portable TypeScript identifier.
For complex branded or generic types, define a named type alias in application code and reference
that name from the sqlay config.

## Database Type Defaults

sqlay keeps precision-sensitive values conservative by default:

- MySQL `DECIMAL` maps to `string`.
- MySQL `BIGINT` and unsigned 64-bit values map to `string`.
- Date and time values map to `string`.
- Unknown metadata maps to `unknown`.
- JSON-derived computed result expressions such as `JSON_UNQUOTE(JSON_EXTRACT(...))` map to
  `unknown` when sqlay cannot prove a stable scalar contract from database metadata alone.

`DECIMAL` and 64-bit integer values default to `string` because JavaScript `number` cannot represent
every value in those MySQL domains without precision loss, and driver-level conversion is an
application decision.

Mapping `decimal` or `int64` to `number` is an explicit project opt-in:

```jsonc
{
  "target": {
    "language": "typescript",
    "typescript": {
      "typeMapping": {
        "core": {
          "decimal": "number",
          "int64": "number",
        },
      },
    },
  },
}
```

This changes generated TypeScript annotations only. It does not make `mysql2` return JavaScript
numbers, prevent precision loss, or convert strings to numbers. Configure and test the application's
execution path before using number overrides for precision-sensitive values.

## mysql2 Runtime Compatibility

When application code executes generated builders through `mysql2`, keep the driver
configuration aligned with the generated type annotations:

- Use `supportBigNumbers: true` and `bigNumberStrings: true` when generated 64-bit
  integer fields remain strings. Without `bigNumberStrings`, `mysql2` can return a
  JavaScript `number` for values inside the safe range and a `string` for larger
  values, which does not match one stable generated field type.
- Use `dateStrings: true` when generated date/time fields remain strings.
- Keep `decimalNumbers` unset or `false` while generated `DECIMAL` fields remain
  strings. Set `decimalNumbers: true` only with an explicit project decision, such
  as mapping `core.decimal` to `number`, and after accepting JavaScript number
  precision risk.
- If `target.typescript.typeMapping` changes a generated annotation to a custom
  project type, configure and test the application's execution path so runtime
  values really satisfy that type.

These settings belong in application code. sqlay does not parse result rows,
configure `mysql2`, or generate driver wrappers.

Schema-backed MySQL `ENUM` columns generate inline literal unions by default:

```ts
status: "draft" | "paid" | "shipped";
```

The enum default applies only when sqlay can tie the generated field or Param to a real schema
column. A broad `core.string` override does not erase schema-backed enum unions; use a column or
builder-local override when one enum field needs a different annotation.

MySQL `SET` remains `string` in the initial design. MySQL drivers expose SET values as strings,
including comma-separated combinations and the empty string, so sqlay does not emit arrays or SET
aliases.

## JSON Expression Results

Computed expressions derived from JSON can return values whose runtime shape depends on the stored
JSON value and expression shape. sqlay therefore keeps those generated result fields conservative by
default:

```ts
export type findBooksByShelf_Row = {
  shelf: unknown | null;
};
```

When an application owns a narrower contract for a specific generated field, use a builder-local
field override:

```jsonc
{
  "target": {
    "language": "typescript",
    "typescript": {
      "typeMapping": {
        "builders": {
          "findBooksByShelf": {
            "fields": {
              "shelf": "string",
            },
          },
        },
      },
    },
  },
}
```

This changes only the generated TypeScript annotation. Application code remains responsible for
ensuring the query, stored JSON data, and database-driver behavior match that narrower type.

## Nullability

Overrides preserve sqlay nullability. A nullable database column or `nullable: true` Param becomes
`CustomType | null` after the base type is overridden:

```ts
export type listOrders_Row = {
  totalAmount: MoneyAmount | null;
};
```

The same rule applies to direct Param inputs and Repeat item fields. Dynamic builders with Slots or
Repeats keep their runtime `params` array as `readonly SqlParam[]`; type mapping narrows input and
row annotations, not runtime SQL parameter packing.

## Param `valueType` Is Different

Inline Param `valueType` is a sqlay Core type hint, not a TypeScript annotation:

```sql
WHERE o.total_amount >= /* @sqlay { type: param id: minimumAmount valueType: decimal } */ 10.00 /* @sqlay { type: paramEnd } */
```

`valueType: decimal` tells sqlay how to classify the SQL value when schema inference is unavailable.
The generated TypeScript type may still be `string`, `number`, `MoneyAmount`, or `MoneyAmount | null`
depending on `target.typescript.typeMapping` and `nullable: true`.
