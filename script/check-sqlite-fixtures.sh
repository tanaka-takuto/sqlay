#!/bin/sh

set -eu

if [ -n "${HOME:-}" ] && [ -f "$HOME/.cargo/env" ]; then
  . "$HOME/.cargo/env"
fi

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
repo_root=${SQLAY_REPO_ROOT:-$(CDPATH= cd "$script_dir/.." && pwd)}

require_command() {
  command_name=$1
  install_hint=$2

  if command -v "$command_name" >/dev/null 2>&1; then
    return 0
  fi

  cat >&2 <<EOF
check-sqlite-fixtures: $command_name is required.

Install:
  $install_hint
EOF
  exit 1
}

compare_directories() {
  expected_dir=$1
  actual_dir=$2
  expected_list=$tmp_root/expected-files.txt
  actual_list=$tmp_root/actual-files.txt

  if [ ! -d "$expected_dir" ]; then
    echo "check-sqlite-fixtures: expected directory does not exist: $expected_dir" >&2
    exit 1
  fi

  if [ ! -d "$actual_dir" ]; then
    echo "check-sqlite-fixtures: actual directory does not exist: $actual_dir" >&2
    exit 1
  fi

  (cd "$expected_dir" && find . -type f | LC_ALL=C sort) > "$expected_list"
  (cd "$actual_dir" && find . -type f | LC_ALL=C sort) > "$actual_list"

  if ! diff -u "$expected_list" "$actual_list"; then
    echo "check-sqlite-fixtures: generated file list differs" >&2
    exit 1
  fi

  while IFS= read -r relative_path; do
    if ! cmp -s "$expected_dir/$relative_path" "$actual_dir/$relative_path"; then
      echo "check-sqlite-fixtures: generated file differs: ${relative_path#./}" >&2
      diff -u "$expected_dir/$relative_path" "$actual_dir/$relative_path" || true
      exit 1
    fi
  done < "$expected_list"
}

require_command "cargo" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
require_command "npm" "install Node.js from https://nodejs.org/"
require_command "sqlite3" "install the SQLite command-line tools from https://sqlite.org/"

tmp_root=$(mktemp -d "${TMPDIR:-/tmp}/sqlay-sqlite-fixtures.XXXXXX")
trap 'rm -rf "$tmp_root"' EXIT HUP INT TERM

fixture_root=$repo_root/fixtures/sqlite
tmp_fixture=$tmp_root/sqlite
database_file=$tmp_fixture/fixture.sqlite3

cp -R "$fixture_root" "$tmp_fixture"
rm -rf "$tmp_fixture/generated"

sqlite3 "$database_file" < "$fixture_root/schema.sql"
if [ ! -f "$database_file" ]; then
  echo "check-sqlite-fixtures: sqlite3 did not create the temporary database" >&2
  exit 1
fi

sqlite_database_url=sqlite://$database_file

SQLITE_DATABASE_URL=$sqlite_database_url \
  cargo run --manifest-path "$repo_root/Cargo.toml" --locked -- \
  check --config "$tmp_fixture/sqlay.config.json"

if [ -e "$tmp_fixture/generated" ]; then
  echo "check-sqlite-fixtures: sqlay check wrote generated files" >&2
  exit 1
fi

SQLITE_DATABASE_URL=$sqlite_database_url \
  cargo run --manifest-path "$repo_root/Cargo.toml" --locked -- \
  compile --config "$tmp_fixture/sqlay.config.json"

compare_directories "$fixture_root/generated" "$tmp_fixture/generated"
(
  cd "$repo_root"
  npm exec -- tsc --noEmit --project "$tmp_fixture/tsconfig.json"
  npm exec -- tsx "$tmp_fixture/assert-builder-runtime.ts"
)
