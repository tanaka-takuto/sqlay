use std::collections::{BTreeMap, BTreeSet};

use sqlay_core as core;
use sqlx::{Row, SqliteConnection};

use super::result_mapping::sqlite_declared_type_to_core_type;

const MAIN_SCHEMA_COLUMNS_SQL: &str = "\
    SELECT schema_table.name AS table_name, table_info.name AS column_name, \
           schema_table.sql AS table_sql, \
           table_info.type AS declared_type, table_info.[notnull] AS not_null, \
           table_info.pk AS primary_key, table_info.hidden AS hidden \
    FROM main.sqlite_schema AS schema_table \
    JOIN pragma_table_xinfo(schema_table.name, 'main') AS table_info \
    WHERE schema_table.type = 'table' \
      AND schema_table.name NOT LIKE 'sqlite_%' \
    ORDER BY schema_table.name, table_info.cid";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SqliteSchemaColumn {
    pub(super) table_name: String,
    pub(super) column_name: String,
    pub(super) ty: core::CoreType,
    pub(super) nullable: Option<bool>,
    pub(super) declared_type_missing: bool,
}

impl SqliteSchemaColumn {
    pub(super) fn reference(&self) -> core::ColumnTypeReference {
        core::ColumnTypeReference::new(None, self.table_name.clone(), self.column_name.clone())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct SqliteSchema {
    columns: BTreeMap<(String, String), SqliteSchemaColumn>,
    tables: BTreeSet<String>,
}

impl SqliteSchema {
    pub(super) fn column(
        &self,
        table_name: &str,
        column_name: &str,
    ) -> Option<&SqliteSchemaColumn> {
        self.columns.get(&(
            normalized_identifier(table_name),
            normalized_identifier(column_name),
        ))
    }

    pub(super) fn has_table(&self, table_name: &str) -> bool {
        self.tables.contains(&normalized_identifier(table_name))
    }

    fn insert(&mut self, column: SqliteSchemaColumn) {
        let table_key = normalized_identifier(&column.table_name);
        let column_key = normalized_identifier(&column.column_name);
        self.tables.insert(table_key.clone());
        self.columns.insert((table_key, column_key), column);
    }
}

pub(super) async fn fetch_main_schema(
    connection: &mut SqliteConnection,
) -> Result<SqliteSchema, sqlx::Error> {
    let rows = sqlx::query(MAIN_SCHEMA_COLUMNS_SQL)
        .fetch_all(connection)
        .await?;
    let mut columns = Vec::new();

    for row in rows {
        let hidden: i64 = row.try_get("hidden")?;
        if hidden != 0 {
            continue;
        }

        let table_name: String = row.try_get("table_name")?;
        let column_name: String = row.try_get("column_name")?;
        let table_sql: String = row.try_get("table_sql")?;
        let declared_type: String = row.try_get("declared_type")?;
        let not_null: i64 = row.try_get("not_null")?;
        let primary_key: i64 = row.try_get("primary_key")?;
        columns.push(RawSqliteSchemaColumn {
            table_name,
            column_name,
            table_sql,
            declared_type,
            not_null,
            primary_key,
        });
    }

    let primary_key_column_counts = columns.iter().fold(BTreeMap::new(), |mut counts, column| {
        if column.primary_key > 0 {
            *counts
                .entry(normalized_identifier(&column.table_name))
                .or_insert(0_usize) += 1;
        }
        counts
    });
    let mut schema = SqliteSchema::default();
    for column in columns {
        let primary_key_column_count = primary_key_column_counts
            .get(&normalized_identifier(&column.table_name))
            .copied()
            .unwrap_or_default();
        let nullable = Some(!column_is_proven_non_null(
            &column,
            primary_key_column_count,
        ));
        schema.insert(SqliteSchemaColumn {
            table_name: column.table_name,
            column_name: column.column_name,
            ty: sqlite_declared_type_to_core_type(&column.declared_type),
            nullable,
            declared_type_missing: column.declared_type.trim().is_empty(),
        });
    }

    Ok(schema)
}

struct RawSqliteSchemaColumn {
    table_name: String,
    column_name: String,
    table_sql: String,
    declared_type: String,
    not_null: i64,
    primary_key: i64,
}

fn column_is_proven_non_null(column: &RawSqliteSchemaColumn, primary_key_count: usize) -> bool {
    if column.not_null != 0 {
        return true;
    }

    let table_sql = column.table_sql.to_ascii_uppercase();
    column.primary_key == 1
        && primary_key_count == 1
        && column.declared_type.trim().eq_ignore_ascii_case("INTEGER")
        && !table_sql.contains("WITHOUT ROWID")
        && !table_sql.contains("DESC")
}

pub(super) fn normalized_identifier(identifier: &str) -> String {
    identifier.to_ascii_lowercase()
}
