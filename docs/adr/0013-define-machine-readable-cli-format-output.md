# ADR 0013: Define Machine-Readable CLI Format Output

## Status

Accepted

## Context

`sqlay check` and `sqlay compile` already print human-oriented summaries that are
useful in a terminal. CI integrations and automation need the same command results
without grepping human text for matched SQL file counts, compiled builder counts,
generated files, stale file cleanup, warnings, or errors.

The existing architecture keeps user-facing diagnostics structured until the CLI
boundary. Successful `check` and `compile` outcomes also already carry structured
summary data. Machine-readable output should use that boundary instead of pushing
JSON formatting into application services, analyzers, metadata providers, or target
generators.

## Decision

Add a `--format` option to `check` and `compile`:

```text
sqlay check --format json
sqlay check --format=json
sqlay compile --format json
sqlay compile --format=json
```

Supported values are initially:

- `human`
- `json`

The default is `human`. Unknown format values are CLI usage errors. Do not add a
`--json` alias in the initial design; the extension point is the `--format` value.

`--format` applies only to `check` and `compile`. `init` and help output remain
human-oriented initially.

### Human Output

`--format human` preserves the existing stream contract:

- success summaries are printed to stdout.
- warnings, errors, and notes are printed to stderr.
- process exit code remains the command's success or failure status.

### JSON Output

`--format json` prints one JSON document to stdout for both successful and failed
`check` and `compile` runs. Stderr is empty. Diagnostics are represented in the JSON
document instead of being duplicated as human text.

The top-level JSON envelope is:

```json
{
  "version": "0.1.0",
  "command": {
    "name": "check",
    "options": {
      "config": null,
      "failOnEmpty": false,
      "format": "json"
    }
  },
  "status": "success",
  "exitCode": 0,
  "summary": {},
  "diagnostics": []
}
```

`version` is the `sqlay` product version from the Cargo package version, not a
separately managed schema version. JSON compatibility is therefore governed by the
product version. Consumers should tolerate additive fields within a compatible
product line and pin the `sqlay` version range when they need strict behavior.

`status` is `"success"` or `"failure"`. `exitCode` is the process exit code that
the CLI returns for the run, initially `0` for success and `1` for failure.

`command` uses normalized command data rather than raw argv:

- `name` is `"check"` or `"compile"`.
- `options.config` is the user-provided config path string when supplied, otherwise
  `null`.
- `options.failOnEmpty` is `true` when `--fail-on-empty` was supplied.
- `options.clean` appears for `compile` and reflects `--clean`.
- `options.allowEmptyClean` appears for `compile` and reflects
  `--allow-empty-clean`.
- `options.format` is `"json"` for JSON output.

When command parsing has already established `--format json`, failures from config
loading, planning, database metadata, source intake, analysis, validation, target
generation, or generated-file writing should use the JSON envelope. CLI usage
errors where no valid JSON output mode can be established, such as a missing or
unknown `--format` value, may use the human diagnostic path.

### Success Summary Shape

On success, `summary` is an object with stable structured data. Common `check` and
`compile` fields include:

```json
{
  "sourceFileCount": 1,
  "builderCount": 2,
  "queryCount": 1,
  "mutationCount": 1,
  "fragmentCount": 0,
  "uniqueSlotCount": 0,
  "uniqueRepeatCount": 0,
  "validationCaseCount": 2,
  "outputDir": "/workspace/src/generated/sqlay",
  "queries": [
    {
      "id": "listUsers",
      "sourcePath": "sql/users.sql",
      "paramCount": 1,
      "inputFieldCount": 1,
      "slotCount": 0,
      "repeatCount": 0,
      "validationCaseCount": 1
    }
  ],
  "mutations": [
    {
      "id": "createUser",
      "sourcePath": "sql/users.sql",
      "kind": "insert",
      "paramCount": 2,
      "inputFieldCount": 2,
      "slotCount": 0,
      "repeatCount": 0,
      "validationCaseCount": 1
    }
  ]
}
```

For `compile`, `summary` also includes generated output details:

```json
{
  "generatedFileCount": 1,
  "generatedFiles": [
    {
      "path": "/workspace/src/generated/sqlay/sql/users.ts"
    }
  ],
  "staleGeneratedFileRemovalCount": null
}
```

`staleGeneratedFileRemovalCount` is `null` when stale generated file cleanup did
not run, and the removed stale generated file count when cleanup did run.

`sourcePath` values are paths relative to the directory containing
`sqlay.config.json` when known. `outputDir` and `generatedFiles[].path` use the
same resolved paths that human summaries currently display.

### Failure Summary Shape

On failure, `summary` is always `null`. The failure reason is represented by
`diagnostics`, and `exitCode` carries the process status.

Do not emit partial summaries for failed runs in the initial design. Different
failure stages expose different amounts of partial state, and stabilizing that
state would make the first JSON contract harder to evolve.

### Diagnostics Shape

Each diagnostic is structured as:

```json
{
  "severity": "error",
  "message": "environment variable `DATABASE_URL` configured by `database.urlEnv` is not set",
  "location": {
    "path": "sql/users.sql",
    "range": {
      "start": {
        "line": 12,
        "column": 5
      },
      "end": {
        "line": 12,
        "column": 18
      }
    }
  }
}
```

`severity` is `"error"`, `"warning"`, or `"note"`. `message` is the diagnostic
message without a rendered severity prefix. `location` is omitted when no location
is available. When present, `location.path` and `location.range` are included only
when the diagnostic has that information. Ranges use one-based line and column
numbers.

Do not include a `rendered` human string in JSON diagnostics. Human formatting is
the responsibility of `--format human`.

## Consequences

This feature should be implemented in small dependent slices:

1. Add `--format <human|json>` and `--format=<human|json>` parsing for `check` and
   `compile`, with default `human` and no `--json` alias.
2. Add CLI JSON serialization for diagnostics and normalized command options.
3. Add CLI JSON serialization for `check` success summaries.
4. Add CLI JSON serialization for `compile` success summaries, generated file
   paths, and stale generated file cleanup counts.
5. Route `--format json` success and failure output to stdout only, leaving stderr
   empty when JSON output mode is active.
6. Update CLI help, README or user-facing docs, and tests for human-output
   preservation and JSON-output stability.

The application and core layers should continue returning structured outcomes and
diagnostic primitives. The CLI boundary owns output format selection and final
serialization.

## Future Work

Later designs may add more `--format` values such as YAML. Unknown format values
remain errors until a format is implemented.

Later designs may extend JSON with additional additive fields, but consumers that
need strict compatibility should pin the `sqlay` product version.

## Alternatives Considered

Add only `--json`. This was rejected because `--format json` gives a clearer
extension point for future formats.

Apply `--format` to `init` and help output. This was rejected for the initial
design because `check` and `compile` are the CI-oriented commands with structured
outcomes and diagnostics. `init` and help are primarily human-facing setup
surfaces.

Emit JSON on stdout while keeping human diagnostics on stderr. This was rejected
because machine-readable callers should be able to parse stdout as the complete
command result without also handling duplicate human diagnostic text.

Emit JSON only for successful runs. This was rejected because CI integrations also
need parseable failure diagnostics.

Use a separate schema version field. This was rejected because the project should
not maintain a second version stream for CLI JSON output. The product version is
the compatibility reference for the initial design.

Emit partial summaries for failed runs. This was rejected because each failure
stage exposes different partial state, and diagnostics are the stable failure
contract for the initial JSON format.
