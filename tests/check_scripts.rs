use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DATABASE_URL: &str = "mysql://sqlay:sqlay@127.0.0.1:3306/sqlay";

#[test]
fn pre_push_accepts_issue_scoped_stack_branch() {
    let fixture = ScriptFixture::new("sqlay-pre-push-stacked-branch");
    write_executable(&fixture.fake_bin.join("cargo"), "#!/bin/sh\nexit 0\n");
    let path = format!(
        "{}:{}",
        fixture.fake_bin.display(),
        std::env::var("PATH").expect("PATH should be set")
    );
    let mut child = Command::new(repo_root().join(".githooks/pre-push"))
        .env("HOME", &fixture.home)
        .env("PATH", path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("pre-push hook should run");

    child
        .stdin
        .take()
        .expect("pre-push stdin should be piped")
        .write_all(
            b"refs/heads/issue/#334-sqlite-adr 1111111111111111111111111111111111111111 refs/heads/issue/#334-sqlite-adr 0000000000000000000000000000000000000000\n",
        )
        .expect("pre-push update should be written");
    let output = child
        .wait_with_output()
        .expect("pre-push hook should finish");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn example_check_typechecks_temporary_generated_project() {
    let fixture = ScriptFixture::new("sqlay-check-examples");

    let output = fixture.run_script("script/check-examples.sh");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let npm_calls = std::fs::read_to_string(fixture.root.join("npm-calls.log"))
        .expect("npm call log should be written");
    assert!(
        npm_calls.contains("exec -- tsx "),
        "example check should execute bookstore result assertions, got: {npm_calls}"
    );
    assert!(
        npm_calls.contains("assert-query-results.ts"),
        "example check should run the query result assertion script, got: {npm_calls}"
    );
}

#[test]
fn mysql_fixture_check_typechecks_temporary_generated_project() {
    let fixture = ScriptFixture::new("sqlay-check-mysql-fixtures");

    let output = fixture.run_script("script/check-mysql-fixtures.sh");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn sqlite_fixture_check_uses_existing_database_and_typechecks_generated_project() {
    let fixture = ScriptFixture::new("sqlay-check-sqlite-fixtures");
    let fixture_repo = fixture.root.join("repo");
    write_sqlite_fixture_contract_repo(&fixture_repo);

    let output =
        fixture.run_script_with_repo_root("script/check-sqlite-fixtures.sh", &fixture_repo);

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let cargo_calls = std::fs::read_to_string(fixture.root.join("cargo-calls.log"))
        .expect("cargo call log should be written");
    let check_position = cargo_calls
        .find(" -- check --config ")
        .expect("SQLite fixture check should run sqlay check");
    let compile_position = cargo_calls
        .find(" -- compile --config ")
        .expect("SQLite fixture check should run sqlay compile");
    assert!(
        check_position < compile_position,
        "sqlay check should run before compile: {cargo_calls}"
    );

    let sqlite_calls = std::fs::read_to_string(fixture.root.join("sqlite3-calls.log"))
        .expect("sqlite3 call log should be written");
    assert!(
        sqlite_calls.contains("/sqlite/fixture.sqlite3"),
        "SQLite fixture check should create the temporary database: {sqlite_calls}"
    );

    let npm_calls = std::fs::read_to_string(fixture.root.join("npm-calls.log"))
        .expect("npm call log should be written");
    assert!(
        npm_calls.contains("exec -- tsc --noEmit --project "),
        "SQLite fixture check should typecheck the generated project: {npm_calls}"
    );
    assert!(
        npm_calls.contains("exec -- tsx ") && npm_calls.contains("assert-builder-runtime.ts"),
        "SQLite fixture check should execute generated builder assertions: {npm_calls}"
    );
    let npm_cwds = std::fs::read_to_string(fixture.root.join("npm-cwds.log"))
        .expect("npm cwd log should be written");
    let expected_npm_cwd = fixture_repo.display().to_string();
    assert!(
        npm_cwds.lines().all(|cwd| cwd == expected_npm_cwd),
        "SQLite npm checks should run from the repository root: {npm_cwds}"
    );
}

#[test]
fn sqlite_fixture_check_rejects_files_written_by_check() {
    let fixture = ScriptFixture::new("sqlay-check-sqlite-fixtures-write-detection");
    let fixture_repo = fixture.root.join("repo");
    write_sqlite_fixture_contract_repo(&fixture_repo);

    let output = fixture
        .script_command("script/check-sqlite-fixtures.sh", &fixture_repo)
        .env("SQLAY_FAKE_CHECK_WRITES", "1")
        .output()
        .expect("check script should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "SQLite fixture check should reject output written by sqlay check"
    );
    assert!(
        stderr.contains("sqlay check wrote generated files"),
        "stderr should explain the no-write contract: {stderr}"
    );

    let cargo_calls = std::fs::read_to_string(fixture.root.join("cargo-calls.log"))
        .expect("cargo call log should be written");
    assert!(
        cargo_calls.contains(" -- check --config "),
        "SQLite fixture check should run sqlay check: {cargo_calls}"
    );
    assert!(
        !cargo_calls.contains(" -- compile --config "),
        "SQLite fixture check should stop before compile: {cargo_calls}"
    );
}

#[test]
fn coverage_check_uses_line_percent_threshold_and_writes_lcov() {
    let fixture = ScriptFixture::new("sqlay-check-coverage");

    let output = fixture.run_script("script/check-coverage.sh");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn structure_check_accepts_committed_baseline() {
    let fixture = ScriptFixture::new("sqlay-check-structure");

    let output = fixture.run_script("script/check-structure.sh");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn structure_check_rejects_unbaselined_large_source_file() {
    let fixture = ScriptFixture::new("sqlay-check-structure-large-file");
    let repo = fixture.root.join("repo");
    write_structure_baseline(&repo, r#"{"version":1,"files":[],"directories":[]}"#);
    write_file(
        &repo.join("crates/app/src/new_large.rs"),
        &rust_comment_lines("// production line ", 601),
    );

    let output = fixture.run_script_with_repo_root("script/check-structure.sh", &repo);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "structure check should fail for an unbaselined large file"
    );
    assert!(
        stderr.contains("crates/app/src/new_large.rs"),
        "stderr should identify the large file: {stderr}"
    );
    assert!(
        stderr.contains("exceeds production soft limit"),
        "stderr should explain the threshold failure: {stderr}"
    );
}

#[test]
fn structure_check_rejects_baseline_growth() {
    let fixture = ScriptFixture::new("sqlay-check-structure-ratchet");
    let repo = fixture.root.join("repo");
    write_structure_baseline(
        &repo,
        r#"{
  "version": 1,
  "files": [
    {
      "path": "crates/app/src/lib.rs",
      "lineCount": 2,
      "kind": "production",
      "splitPlan": "Keep this test fixture small."
    }
  ],
  "directories": []
}"#,
    );
    write_file(
        &repo.join("crates/app/src/lib.rs"),
        "// one\n// two\n// three\n",
    );

    let output = fixture.run_script_with_repo_root("script/check-structure.sh", &repo);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "structure check should fail when a baselined file grows"
    );
    assert!(
        stderr.contains("grew beyond baseline"),
        "stderr should describe the ratchet failure: {stderr}"
    );
}

#[test]
fn structure_check_rejects_unbaselined_large_module_directory() {
    let fixture = ScriptFixture::new("sqlay-check-structure-large-directory");
    let repo = fixture.root.join("repo");
    write_structure_baseline(&repo, r#"{"version":1,"files":[],"directories":[]}"#);
    for index in 0..9 {
        write_file(
            &repo.join(format!("crates/app/src/module_{index}.rs")),
            "// small module\n",
        );
    }

    let output = fixture.run_script_with_repo_root("script/check-structure.sh", &repo);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "structure check should fail when a module directory grows too wide"
    );
    assert!(
        stderr.contains("crates/app/src"),
        "stderr should identify the wide module directory: {stderr}"
    );
    assert!(
        stderr.contains("private subdirectory"),
        "stderr should suggest directory splitting: {stderr}"
    );
}

#[test]
fn structure_check_rejects_generic_module_names() {
    let fixture = ScriptFixture::new("sqlay-check-structure-generic-name");
    let repo = fixture.root.join("repo");
    write_structure_baseline(&repo, r#"{"version":1,"files":[],"directories":[]}"#);
    write_file(&repo.join("crates/app/src/utils.rs"), "// escape hatch\n");

    let output = fixture.run_script_with_repo_root("script/check-structure.sh", &repo);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "structure check should fail for generic module names"
    );
    assert!(
        stderr.contains("uses generic module name utils.rs"),
        "stderr should identify the forbidden filename: {stderr}"
    );
}

#[test]
fn structure_check_ignores_symlinked_rust_files_outside_repo() {
    let fixture = ScriptFixture::new("sqlay-check-structure-external-symlink");
    let repo = fixture.root.join("repo");
    let outside = fixture.root.join("outside.rs");
    write_structure_baseline(&repo, r#"{"version":1,"files":[],"directories":[]}"#);
    write_file(&outside, "// outside repo\n");
    std::fs::create_dir_all(repo.join("crates/app/src"))
        .expect("fixture module directory should be created");
    symlink(&outside, repo.join("crates/app/src/outside.rs"))
        .expect("fixture symlink should be created");

    let output = fixture.run_script_with_repo_root("script/check-structure.sh", &repo);

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

struct ScriptFixture {
    root: PathBuf,
    fake_bin: PathBuf,
    home: PathBuf,
}

impl ScriptFixture {
    fn new(prefix: &str) -> Self {
        let root = unique_temp_dir(prefix);
        let fake_bin = root.join("bin");
        let home = root.join("home");
        std::fs::create_dir_all(&fake_bin).expect("fake bin directory should be created");
        std::fs::create_dir_all(&home).expect("fake home directory should be created");

        let fixture = Self {
            root,
            fake_bin,
            home,
        };
        fixture.write_fake_cargo();
        fixture.write_fake_mysql();
        fixture.write_fake_npm();
        fixture.write_fake_sqlite3();
        fixture
    }

    fn run_script(&self, script_path: &str) -> std::process::Output {
        self.run_script_with_repo_root(script_path, &repo_root())
    }

    fn run_script_with_repo_root(
        &self,
        script_path: &str,
        target_repo_root: &Path,
    ) -> std::process::Output {
        self.script_command(script_path, target_repo_root)
            .output()
            .expect("check script should run")
    }

    fn script_command(&self, script_path: &str, target_repo_root: &Path) -> Command {
        let repo_root = repo_root();
        let path = format!(
            "{}:{}",
            self.fake_bin.display(),
            std::env::var("PATH").expect("PATH should be set")
        );

        let mut command = Command::new(repo_root.join(script_path));
        command
            .env("DATABASE_URL", DATABASE_URL)
            .env("HOME", &self.home)
            .env("PATH", path)
            .env("SQLAY_REPO_ROOT", target_repo_root)
            .env("TMPDIR", &self.root);
        command
    }

    fn write_fake_cargo(&self) {
        write_executable(
            &self.fake_bin.join("cargo"),
            r#"#!/bin/sh
set -eu

copy_generated() {
  expected_dir=$1
  generated_dir=$2

  mkdir -p "$generated_dir"
  cp -R "$expected_dir/." "$generated_dir/"
}

case "$1" in
  run)
    printf '%s\n' "$*" >> "$TMPDIR/cargo-calls.log"
    config_path=
    sqlay_command=
    after_separator=0
    previous=
    for arg in "$@"; do
      if [ "$previous" = "--config" ]; then
        config_path=$arg
        previous=
        continue
      fi
      if [ "$arg" = "--config" ]; then
        previous=--config
        continue
      fi
      if [ "$after_separator" -eq 1 ] && [ -z "$sqlay_command" ]; then
        sqlay_command=$arg
      fi
      if [ "$arg" = "--" ]; then
        after_separator=1
      fi
    done

    if [ -n "$config_path" ]; then
      project_dir=$(CDPATH= cd "$(dirname "$config_path")" && pwd)
      case "$config_path" in
        "$TMPDIR"/sqlay-sqlite-fixtures.*/sqlite/sqlay.config.json)
          case "$sqlay_command" in
            check)
              if [ "${SQLAY_FAKE_CHECK_WRITES:-0}" = "1" ]; then
                mkdir -p "$project_dir/generated"
                printf '%s\n' '// unexpected check output' > "$project_dir/generated/unexpected.ts"
              fi
              ;;
            compile)
              copy_generated "$SQLAY_REPO_ROOT/fixtures/sqlite/generated" "$project_dir/generated"
              ;;
            *)
              echo "unexpected sqlay command for SQLite fixture: $sqlay_command" >&2
              exit 64
              ;;
          esac
          ;;
        *)
          copy_generated "$SQLAY_REPO_ROOT/examples/bookstore/generated" "$project_dir/generated"
          ;;
      esac
      exit 0
    fi

    project_dir=$(CDPATH= cd "../.." && pwd)
    if grep -q '"dir": "generated-type-mapping"' "$project_dir/sqlay.config.json"; then
      copy_generated "$SQLAY_REPO_ROOT/fixtures/sql/generated-type-mapping" "$project_dir/generated-type-mapping"
    else
      copy_generated "$SQLAY_REPO_ROOT/fixtures/sql/generated" "$project_dir/generated"
    fi
    ;;
  test)
    ;;
  llvm-cov)
    if [ "$#" -eq 2 ] && [ "$2" = "--version" ]; then
      exit 0
    fi

    expected_args="llvm-cov --workspace --all-targets --all-features --fail-under-lines 85 --lcov --output-path coverage/lcov.info"
    if [ "$*" != "$expected_args" ]; then
      echo "expected cargo coverage args: $expected_args, got: $*" >&2
      exit 64
    fi
    ;;
  *)
    echo "unexpected cargo args: $*" >&2
    exit 64
    ;;
esac
"#,
        );
    }

    fn write_fake_sqlite3(&self) {
        write_executable(
            &self.fake_bin.join("sqlite3"),
            r#"#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "expected sqlite3 database path, got: $*" >&2
  exit 64
fi

database_file=$1
schema=$(cat)
case "$schema" in
  *"CREATE TABLE fixture_orders"*) ;;
  *)
    echo "expected SQLite fixture schema on stdin" >&2
    exit 64
    ;;
esac

printf '%s\n' "$database_file" >> "$TMPDIR/sqlite3-calls.log"
mkdir -p "$(dirname "$database_file")"
: > "$database_file"
"#,
        );
    }

    fn write_fake_mysql(&self) {
        write_executable(
            &self.fake_bin.join("mysql"),
            r#"#!/bin/sh
set -eu

for arg in "$@"; do
  case "$arg" in
    --execute)
      echo 1
      exit 0
      ;;
  esac
done

cat >/dev/null
"#,
        );
    }

    fn write_fake_npm(&self) {
        write_executable(
            &self.fake_bin.join("npm"),
            r#"#!/bin/sh
set -eu

printf '%s\n' "$*" >> "$TMPDIR/npm-calls.log"
pwd >> "$TMPDIR/npm-cwds.log"

if [ "$#" -eq 4 ] \
  && [ "$1" = "exec" ] \
  && [ "$2" = "--" ] \
  && [ "$3" = "tsx" ]; then
  case "$4" in
    "$TMPDIR"/sqlay-examples.*/bookstore/assert-query-results.ts)
      if ! grep -q 'rows.length > 1' "$4"; then
        echo "expected result assertion script to reject multi-row single-result queries" >&2
        exit 64
      fi
      ;;
    "$TMPDIR"/sqlay-sqlite-fixtures.*/sqlite/assert-builder-runtime.ts)
      if ! grep -q 'sqliteBulkInsertOrderItems' "$4" \
        || ! grep -q 'sqliteDeleteOrderItem' "$4"; then
        echo "expected SQLite runtime assertions for Repeat and mutation Slot builders" >&2
        exit 64
      fi
      ;;
    *)
      echo "expected npm to run a temporary bookstore result assertion, got: $*" >&2
      exit 64
      ;;
  esac
  exit 0
fi

if [ "$#" -ne 6 ] \
  || [ "$1" != "exec" ] \
  || [ "$2" != "--" ] \
  || [ "$3" != "tsc" ] \
  || [ "$4" != "--noEmit" ] \
  || [ "$5" != "--project" ]; then
  echo "expected npm to typecheck a temporary generated project, got: $*" >&2
  exit 64
fi

case "$6" in
  "$TMPDIR"/sqlay-examples.*/bookstore/tsconfig.json)
    project_dir=$(dirname "$6")
    if [ ! -L "$project_dir/node_modules" ]; then
      echo "expected temporary bookstore project to link repo node_modules" >&2
      exit 64
    fi
    ;;
  "$TMPDIR"/sqlay-mysql-fixtures.*/sql/tsconfig.json) ;;
  "$TMPDIR"/sqlay-mysql-fixtures.*/sql-type-mapping/tsconfig.json) ;;
  "$TMPDIR"/sqlay-sqlite-fixtures.*/sqlite/tsconfig.json) ;;
  *)
    echo "expected npm to typecheck a temporary generated project, got: $*" >&2
    exit 64
    ;;
esac
"#,
        );
    }
}

impl Drop for ScriptFixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

fn write_executable(path: &Path, content: &str) {
    let mut file = std::fs::File::create(path).expect("fake command should be created");
    file.write_all(content.as_bytes())
        .expect("fake command should be written");
    let mut permissions = file
        .metadata()
        .expect("fake command metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("fake command should be executable");
}

fn write_file(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().expect("fixture path should have a parent"))
        .expect("fixture parent directory should be created");
    std::fs::write(path, content).expect("fixture file should be written");
}

fn write_structure_baseline(repo_root: &Path, content: &str) {
    write_file(&repo_root.join("docs/structure-baseline.json"), content);
}

fn write_sqlite_fixture_contract_repo(repo_root: &Path) {
    write_file(
        &repo_root.join("fixtures/sqlite/schema.sql"),
        "CREATE TABLE fixture_orders (id INTEGER PRIMARY KEY);\n",
    );
    write_file(
        &repo_root.join("fixtures/sqlite/sqlay.config.json"),
        r#"{
  "source": { "include": ["valid/**/*.sql"] },
  "output": { "dir": "generated" },
  "database": { "dialect": "sqlite", "urlEnv": "SQLITE_DATABASE_URL" },
  "target": { "language": "typescript" }
}
"#,
    );
    write_file(
        &repo_root.join("fixtures/sqlite/valid/sqlite_builders.sql"),
        "/* @sqlay { type: query id: sqliteFixtureQuery } */\nSELECT id FROM fixture_orders;\n",
    );
    write_file(
        &repo_root.join("fixtures/sqlite/tsconfig.json"),
        r#"{
  "compilerOptions": { "strict": true, "noEmit": true },
        "include": ["assert-builder-runtime.ts", "assert-generated-surface.ts", "generated/**/*.ts"]
}
"#,
    );
    write_file(
        &repo_root.join("fixtures/sqlite/assert-generated-surface.ts"),
        "import { sqliteFixtureQuery } from \"./generated/valid/sqlite_builders\";\nvoid sqliteFixtureQuery;\n",
    );
    write_file(
        &repo_root.join("fixtures/sqlite/assert-builder-runtime.ts"),
        "import { sqliteFixtureQuery } from \"./generated/valid/sqlite_builders\";\nvoid sqliteFixtureQuery;\n// sqliteBulkInsertOrderItems\n// sqliteDeleteOrderItem\n",
    );
    write_file(
        &repo_root.join("fixtures/sqlite/generated/valid/sqlite_builders.ts"),
        "// generated SQLite fixture contract\nexport const sqliteFixtureQuery = true;\n",
    );
}

fn rust_comment_lines(prefix: &str, line_count: usize) -> String {
    let mut content = String::new();
    for line in 0..line_count {
        content.push_str(prefix);
        content.push_str(&line.to_string());
        content.push('\n');
    }
    content
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
}
