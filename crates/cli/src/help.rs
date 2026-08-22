pub const HELP: &str = "\
SQL Inlay.

Usage:
  sqlay <command> [options]

Commands:
  sqlay init       Create a starter sqlay.config.json.
  sqlay check      Load config and run the compile pipeline without writing generated files.
  sqlay compile    Load config and write generated TypeScript files.

Options:
  -h, --help         Print this help.
  --config <path>    Use an explicit config path for check or compile.
  --format <human|json>
                     Select output format for check or compile. The default is human; JSON prints a stdout result envelope.
  --clean            Remove stale generated files during compile.
  --fail-on-empty    Exit with an error when source.include matches no SQL files after source.exclude.
  --allow-empty-clean
                     Allow compile --clean to remove stale generated files when no SQL files match.

Minimal query annotation:
  /* @sqlay
  {
    type: query
    id: listUsers
    // cardinality: one | many
  }
  */
  SELECT id, name FROM users;

Minimal mutation annotation:
  /* @sqlay
  {
    type: mutation
    id: createUser
  }
  */
  INSERT INTO users (email, name)
  VALUES (
    /* @sqlay { type: param id: email } */ 'ada@example.test' /* @sqlay { type: paramEnd } */,
    /* @sqlay { type: param id: name } */ 'Ada' /* @sqlay { type: paramEnd } */
  );

Query metadata:
  type: query is required.
  id is required and must match ^[A-Za-z_][A-Za-z0-9_]*$.
  cardinality is optional: one or many. cardinality: exec is rejected.

Mutation builders:
  type: mutation supports INSERT, UPDATE, DELETE, and REPLACE builders.
  check and compile validate mutation SQL and infer input Params, but never execute mutation statements.
  Mutation builders return { sql, params } only.
  affectedRows, insertId, changedRows, transactions, upserts, and REPLACE result interpretation belong to application/driver code.
  See docs/mutation-execution.md for mysql2/promise execution examples.

Directive boundary:
  Compiler directives are @sqlay Hjson block comments.
  Similar ordinary SQL comments such as /* @param tenantKey */ are ignored as SQL comments.
  Do not write raw `?` placeholders in source SQL; use paired @sqlay Param markers around a sample expression.
  Slot and Fragment composition is available for optional single-select query-local slots.
  Repeat ranges are available for variable-length SQL repetition inside queries, mutations, and fragments.

Config path boundary:
  source.include paths must stay inside the config directory.
  Place sqlay.config.json at the project root when SQL lives in sibling directories.
  Generated TypeScript preserves each input SQL path relative to the config directory under output.dir.

Database and generated-code boundary:
  database.dialect values: mysql | sqlite. database.urlEnv names the process environment variable containing the database URL.
  SQLite 3.35+ accepts sqlite://relative/path.db (relative to the process working directory) or sqlite:///absolute/path.db.
  SQLite URL query parameters are unsupported.
  The SQLite URL must identify an existing regular file; only the SQLite main schema is inspected.
  In-memory, temporary, attached, and encrypted SQLite databases are unsupported.
  sqlay check validates without writing generated files.
  Generated TypeScript builders are database-driver-independent and return SQL plus params only.

SQLite mutation boundary:
  SQLite mutations support INSERT ... VALUES, UPDATE ... WHERE, DELETE ... WHERE, and REPLACE ... VALUES.
  RETURNING and ON CONFLICT UPSERT, INSERT/REPLACE ... SELECT, top-level CTE mutations, and UPDATE ... FROM are unsupported for SQLite.

Optional filter Slot/Fragment example:
  /* @sqlay
  {
    type: fragment
    id: byStatus
  }
  */
  AND orders.status = /* @sqlay { type: param id: status } */ 'paid' /* @sqlay { type: paramEnd } */

  /* @sqlay
  {
    type: query
    id: listOrders
  }
  */
  SELECT orders.id, orders.status
  FROM orders
  WHERE 1 = 1
  /* @sqlay { type: slot id: statusFilter targets: [byStatus] } */;

Generated TypeScript input:
  export type listOrders_Input = {
    statusFilter?: {
      $fragment: \"byStatus\";
      status: string;
    };
  };

Param metadata:
  For optional filters that change whether a predicate exists, prefer Slot/Fragment composition over nullable sentinel predicates.
  valueType values: bool, int32, int64, float64, decimal, string, bytes, date, time, datetime, json.
  Use nullable: true for values that are semantically nullable in a stable SQL shape.
  Use nullable: true for T | null inputs; optional input properties are not supported.
  For non-null Param inputs, omit nullable; use only nullable: true for nullable inputs.
  Repeated Param ids share one generated input field.
  Each marker occurrence appends one params entry in source order.
  All occurrences of a repeated Param id must use the same valueType and nullability.
  For bool Params, use TRUE or FALSE as the sample expression.
";

pub const INIT_HELP: &str = "\
Create a starter sqlay.config.json.

Usage:
  sqlay init

Behavior:
  Writes a starter sqlay.config.json in the current directory and refuses to overwrite an existing config file.
  Prints a minimal @sqlay query annotation and the next check command.

Examples:
  sqlay init
";

pub const CHECK_HELP: &str = "\
Check SQL sources without writing generated files.

Usage:
  sqlay check [options]

Behavior:
  Loads sqlay.config.json, reads SQL files, validates MySQL or SQLite queries and mutations, and renders generated TypeScript output in memory.
  When --config is omitted, searches from the current working directory upward for sqlay.config.json.
  Reads the database URL from the environment variable named by database.urlEnv.
  No files are written.
  Generated TypeScript preserves each input SQL path relative to the config directory under output.dir.
  source.include paths must stay inside the config directory.
  Place sqlay.config.json at the project root when SQL lives in sibling directories.
  The success summary reports matched SQL files, compiled builders with query and mutation counts, Fragment, Slot, Repeat, validation case counts, output.dir, and per-query/per-mutation Param, Slot, Repeat, and validation case counts.
  Empty source matches are reported as warnings unless --fail-on-empty is provided.

Database and generated-code boundary:
  database.dialect values: mysql | sqlite. database.urlEnv names the process environment variable containing the database URL.
  SQLite 3.35+ accepts sqlite://relative/path.db (relative to the process working directory) or sqlite:///absolute/path.db.
  SQLite URL query parameters are unsupported.
  The SQLite URL must identify an existing regular file; only the SQLite main schema is inspected.
  In-memory, temporary, attached, and encrypted SQLite databases are unsupported.
  sqlay check validates without writing generated files.
  Generated TypeScript builders are database-driver-independent and return SQL plus params only.

SQLite mutation boundary:
  SQLite mutations support INSERT ... VALUES, UPDATE ... WHERE, DELETE ... WHERE, and REPLACE ... VALUES.
  RETURNING and ON CONFLICT UPSERT, INSERT/REPLACE ... SELECT, top-level CTE mutations, and UPDATE ... FROM are unsupported for SQLite.

Optional filter Slot/Fragment example:
  /* @sqlay
  {
    type: fragment
    id: byStatus
  }
  */
  AND orders.status = /* @sqlay { type: param id: status } */ 'paid' /* @sqlay { type: paramEnd } */

  /* @sqlay
  {
    type: query
    id: listOrders
  }
  */
  SELECT orders.id, orders.status
  FROM orders
  WHERE 1 = 1
  /* @sqlay { type: slot id: statusFilter targets: [byStatus] } */;

Generated TypeScript input:
  export type listOrders_Input = {
    statusFilter?: {
      $fragment: \"byStatus\";
      status: string;
    };
  };

Param metadata:
  For optional filters that change whether a predicate exists, prefer Slot/Fragment composition over nullable sentinel predicates.
  valueType values: bool, int32, int64, float64, decimal, string, bytes, date, time, datetime, json.
  Use nullable: true for values that are semantically nullable in a stable SQL shape.
  Use nullable: true for T | null inputs; optional input properties are not supported.
  For non-null Param inputs, omit nullable; use only nullable: true for nullable inputs.
  Repeated Param ids share one generated input field.
  Each marker occurrence appends one params entry in source order.
  All occurrences of a repeated Param id must use the same valueType and nullability.
  For bool Params, use TRUE or FALSE as the sample expression.

Minimal mutation annotation:
  /* @sqlay
  {
    type: mutation
    id: createUser
  }
  */
  INSERT INTO users (email, name)
  VALUES (
    /* @sqlay { type: param id: email } */ 'ada@example.test' /* @sqlay { type: paramEnd } */,
    /* @sqlay { type: param id: name } */ 'Ada' /* @sqlay { type: paramEnd } */
  );

Mutation builders:
  type: mutation supports INSERT, UPDATE, DELETE, and REPLACE builders.
  check and compile validate mutation SQL and infer input Params, but never execute mutation statements.
  Mutation builders return { sql, params } only.
  affectedRows, insertId, changedRows, transactions, upserts, and REPLACE result interpretation belong to application/driver code.
  See docs/mutation-execution.md for mysql2/promise execution examples.

Options:
  -h, --help         Print this help.
  --config <path>    Use an explicit config path.
  --format <human|json>
                     Select output format. The default is human. JSON prints a stdout result envelope with diagnostics and the check summary.
  --fail-on-empty    Exit with an error when source.include matches no SQL files after source.exclude.

Examples:
  DATABASE_URL=... sqlay check
  sqlay check --format json
  sqlay check --config ./sqlay.config.json
";

pub const COMPILE_HELP: &str = "\
Compile SQL sources to generated TypeScript files.

Usage:
  sqlay compile [options]

Behavior:
  Loads sqlay.config.json, validates MySQL or SQLite SQL sources, and writes generated TypeScript files under output.dir.
  When --config is omitted, searches from the current working directory upward for sqlay.config.json.
  Reads the database URL from the environment variable named by database.urlEnv.
  Generated TypeScript preserves each input SQL path relative to the config directory under output.dir.
  source.include paths must stay inside the config directory.
  Place sqlay.config.json at the project root when SQL lives in sibling directories.
  The success summary reports matched SQL files, compiled builders with query and mutation counts, Fragment, Slot, Repeat, validation case counts, generated file paths, stale-file cleanup, and per-query/per-mutation Param, Slot, Repeat, and validation case counts.
  Empty source matches are reported as warnings unless --fail-on-empty is provided.
  compile --clean skips stale generated file cleanup when no SQL files match unless --allow-empty-clean is also provided.
  TypeScript type mapping is conservative: BIGINT, DECIMAL, date/time, and enum values map conservatively to string; bytes map to Uint8Array; JSON and unknown types map to unknown; nullable metadata adds | null.

Database and generated-code boundary:
  database.dialect values: mysql | sqlite. database.urlEnv names the process environment variable containing the database URL.
  SQLite 3.35+ accepts sqlite://relative/path.db (relative to the process working directory) or sqlite:///absolute/path.db.
  SQLite URL query parameters are unsupported.
  The SQLite URL must identify an existing regular file; only the SQLite main schema is inspected.
  In-memory, temporary, attached, and encrypted SQLite databases are unsupported.
  sqlay check validates without writing generated files.
  Generated TypeScript builders are database-driver-independent and return SQL plus params only.

SQLite mutation boundary:
  SQLite mutations support INSERT ... VALUES, UPDATE ... WHERE, DELETE ... WHERE, and REPLACE ... VALUES.
  RETURNING and ON CONFLICT UPSERT, INSERT/REPLACE ... SELECT, top-level CTE mutations, and UPDATE ... FROM are unsupported for SQLite.

Optional filter Slot/Fragment example:
  /* @sqlay
  {
    type: fragment
    id: byStatus
  }
  */
  AND orders.status = /* @sqlay { type: param id: status } */ 'paid' /* @sqlay { type: paramEnd } */

  /* @sqlay
  {
    type: query
    id: listOrders
  }
  */
  SELECT orders.id, orders.status
  FROM orders
  WHERE 1 = 1
  /* @sqlay { type: slot id: statusFilter targets: [byStatus] } */;

Generated TypeScript input:
  export type listOrders_Input = {
    statusFilter?: {
      $fragment: \"byStatus\";
      status: string;
    };
  };

Param metadata:
  For optional filters that change whether a predicate exists, prefer Slot/Fragment composition over nullable sentinel predicates.
  valueType values: bool, int32, int64, float64, decimal, string, bytes, date, time, datetime, json.
  Use nullable: true for values that are semantically nullable in a stable SQL shape.
  Use nullable: true for T | null inputs; optional input properties are not supported.
  For non-null Param inputs, omit nullable; use only nullable: true for nullable inputs.
  Repeated Param ids share one generated input field.
  Each marker occurrence appends one params entry in source order.
  All occurrences of a repeated Param id must use the same valueType and nullability.
  For bool Params, use TRUE or FALSE as the sample expression.

Minimal mutation annotation:
  /* @sqlay
  {
    type: mutation
    id: createUser
  }
  */
  INSERT INTO users (email, name)
  VALUES (
    /* @sqlay { type: param id: email } */ 'ada@example.test' /* @sqlay { type: paramEnd } */,
    /* @sqlay { type: param id: name } */ 'Ada' /* @sqlay { type: paramEnd } */
  );

Mutation builders:
  type: mutation supports INSERT, UPDATE, DELETE, and REPLACE builders.
  check and compile validate mutation SQL and infer input Params, but never execute mutation statements.
  Mutation builders return { sql, params } only.
  affectedRows, insertId, changedRows, transactions, upserts, and REPLACE result interpretation belong to application/driver code.
  See docs/mutation-execution.md for mysql2/promise execution examples.

Options:
  -h, --help         Print this help.
  --config <path>    Use an explicit config path.
  --format <human|json>
                     Select output format. The default is human. JSON prints a stdout result envelope with diagnostics and the compile summary.
  --clean            Remove stale generated files that no longer correspond to input SQL files.
  --fail-on-empty    Exit with an error when source.include matches no SQL files after source.exclude.
  --allow-empty-clean
                     Allow --clean to remove stale generated files when source.include matches no SQL files.

Examples:
  DATABASE_URL=... sqlay compile
  sqlay compile --format json
  sqlay compile --config ./sqlay.config.json --clean
  sqlay compile --clean --allow-empty-clean
";

pub const INIT_NEXT_STEPS: &str = r"
Next:
  export DATABASE_URL=...
  sqlay check

Or run one command with the environment variable set:
  DATABASE_URL=... sqlay check

The starter config remains MySQL and uses database.dialect = mysql with
database.urlEnv = DATABASE_URL. If you change database.urlEnv, export that
variable instead. sqlay reads the URL from the process environment.
sqlay does not load .env files automatically.

database.dialect values: mysql | sqlite. database.urlEnv names the process environment variable containing the database URL.
SQLite 3.35+ accepts sqlite://relative/path.db (relative to the process working directory) or sqlite:///absolute/path.db.
SQLite URL query parameters are unsupported.
The SQLite URL must identify an existing regular file; only the SQLite main schema is inspected.
In-memory, temporary, attached, and encrypted SQLite databases are unsupported.
sqlay check validates without writing generated files.
Generated TypeScript builders are database-driver-independent and return SQL plus params only.

SQLite mutations support INSERT ... VALUES, UPDATE ... WHERE, DELETE ... WHERE, and REPLACE ... VALUES.
RETURNING and ON CONFLICT UPSERT, INSERT/REPLACE ... SELECT, top-level CTE mutations, and UPDATE ... FROM are unsupported for SQLite.

Run check and compile from your project directory. When --config is omitted,
sqlay searches from the current working directory upward for sqlay.config.json.

Compiler directives are @sqlay Hjson block comments. Ordinary SQL comments such as
/* @param tenantKey */ are ignored as SQL comments.

Add a query block such as:
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id, name FROM users;
";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelpTopic {
    TopLevel,
    Init,
    Check,
    Compile,
}

pub const fn help_text(topic: HelpTopic) -> &'static str {
    match topic {
        HelpTopic::TopLevel => HELP,
        HelpTopic::Init => INIT_HELP,
        HelpTopic::Check => CHECK_HELP,
        HelpTopic::Compile => COMPILE_HELP,
    }
}
