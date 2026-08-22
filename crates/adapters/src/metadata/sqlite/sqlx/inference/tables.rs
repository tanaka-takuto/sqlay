use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

use sqlparser::ast::{
    JoinOperator, ObjectName, Query as SqlQuery, Select, SetExpr, TableFactor, TableWithJoins,
};

use super::super::schema::{SqliteSchema, SqliteSchemaColumn, normalized_identifier};
use super::expressions::ColumnRef;

#[derive(Clone, Debug, Eq, PartialEq)]
enum TableResolution {
    Main {
        table_name: String,
        explicit_main: bool,
        outer_join_nullable: bool,
    },
    Unsupported,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct TableSources {
    by_qualifier: BTreeMap<String, TableResolution>,
    main_tables: BTreeMap<(String, bool), bool>,
}

impl TableSources {
    fn insert_resolution(&mut self, key: &str, resolution: TableResolution) {
        match self.by_qualifier.entry(normalized_identifier(key)) {
            Entry::Vacant(entry) => {
                entry.insert(resolution);
            }
            Entry::Occupied(mut entry) => match (entry.get_mut(), resolution) {
                (
                    TableResolution::Main {
                        table_name,
                        explicit_main,
                        outer_join_nullable,
                    },
                    TableResolution::Main {
                        table_name: new_table_name,
                        explicit_main: new_explicit_main,
                        outer_join_nullable: new_outer_join_nullable,
                    },
                ) if table_name == &new_table_name && *explicit_main == new_explicit_main => {
                    *outer_join_nullable |= new_outer_join_nullable;
                }
                (existing, new) if existing == &new => {}
                (existing, _) => *existing = TableResolution::Unsupported,
            },
        }
    }

    fn insert_main(&mut self, table_name: &str, alias: Option<String>, explicit_main: bool) {
        self.main_tables
            .entry((normalized_identifier(table_name), explicit_main))
            .or_insert(false);
        let resolution = TableResolution::Main {
            table_name: table_name.to_owned(),
            explicit_main,
            outer_join_nullable: false,
        };
        self.insert_resolution(table_name, resolution.clone());
        self.insert_resolution(&format!("main.{table_name}"), resolution.clone());
        if let Some(alias) = alias {
            self.insert_resolution(&alias, resolution);
        }
    }

    fn mark_outer_join_nullable(&mut self) {
        for resolution in self.by_qualifier.values_mut() {
            if let TableResolution::Main {
                outer_join_nullable,
                ..
            } = resolution
            {
                *outer_join_nullable = true;
            }
        }
        for outer_join_nullable in self.main_tables.values_mut() {
            *outer_join_nullable = true;
        }
    }

    fn extend(&mut self, other: Self) {
        for (qualifier, resolution) in other.by_qualifier {
            self.insert_resolution(&qualifier, resolution);
        }
        for (table, outer_join_nullable) in other.main_tables {
            self.main_tables
                .entry(table)
                .and_modify(|existing| *existing |= outer_join_nullable)
                .or_insert(outer_join_nullable);
        }
    }

    fn insert_unsupported(&mut self, table_name: Option<String>, alias: Option<String>) {
        if let Some(table_name) = table_name {
            self.insert_resolution(&table_name, TableResolution::Unsupported);
        }
        if let Some(alias) = alias {
            self.insert_resolution(&alias, TableResolution::Unsupported);
        }
    }

    pub(super) fn resolve_column(
        &self,
        schema: &SqliteSchema,
        column: &ColumnRef,
    ) -> Option<SqliteSchemaColumn> {
        let TableResolution::Main {
            table_name,
            explicit_main,
            outer_join_nullable,
        } = self
            .by_qualifier
            .get(&normalized_identifier(&column.qualifier))?
        else {
            return None;
        };
        let column_qualifies_main = column_uses_explicit_main(&column.qualifier);
        let mut schema_column = schema.column(table_name, &column.column)?.clone();
        if *explicit_main || column_qualifies_main {
            schema_column = schema_column.with_explicit_main();
        }
        if *outer_join_nullable {
            schema_column = schema_column.with_unknown_nullability();
        }
        Some(schema_column)
    }

    pub(super) fn resolve_unqualified_column(
        &self,
        schema: &SqliteSchema,
        column_name: &str,
    ) -> Option<SqliteSchemaColumn> {
        let mut matches = self.main_tables.iter().filter_map(
            |((table_name, explicit_main), outer_join_nullable)| {
                schema
                    .has_table(table_name)
                    .then(|| schema.column(table_name, column_name))
                    .flatten()
                    .cloned()
                    .map(|mut column| {
                        if *explicit_main {
                            column = column.with_explicit_main();
                        }
                        if *outer_join_nullable {
                            column = column.with_unknown_nullability();
                        }
                        column
                    })
            },
        );
        let column = matches.next()?;
        matches.next().is_none().then_some(column)
    }

    pub(super) fn qualifier_is_known(&self, qualifier: &str) -> bool {
        self.by_qualifier
            .contains_key(&normalized_identifier(qualifier))
    }
}

pub(super) fn direct_select_query(query: &SqlQuery) -> Option<(&SqlQuery, &Select)> {
    match query.body.as_ref() {
        SetExpr::Select(select) => Some((query, select)),
        SetExpr::Query(query) => direct_select_query(query),
        _ => None,
    }
}

pub(super) fn select_table_sources(query: &SqlQuery, select: &Select) -> TableSources {
    let cte_names = query
        .with
        .as_ref()
        .map(|with| {
            with.cte_tables
                .iter()
                .map(|cte| normalized_identifier(&cte.alias.name.value))
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    select_table_sources_with_cte_names(select, &cte_names)
}

pub(super) fn select_table_sources_with_cte_names(
    select: &Select,
    cte_names: &BTreeSet<String>,
) -> TableSources {
    let mut sources = TableSources::default();
    for table in &select.from {
        collect_table_with_joins(table, &mut sources, cte_names);
    }
    sources
}

pub(super) fn single_table_sources(table: &TableWithJoins) -> TableSources {
    let mut sources = TableSources::default();
    collect_table_with_joins(table, &mut sources, &BTreeSet::new());
    sources
}

pub(super) fn named_table_sources(name: &ObjectName, alias: Option<&str>) -> TableSources {
    let mut sources = TableSources::default();
    let alias = alias.map(str::to_owned);
    let parts = object_name_parts(name);
    if let Some(table_name) = main_table_name(&parts) {
        sources.insert_main(&table_name, alias, parts.len() == 2);
    } else {
        sources.insert_unsupported(parts.last().cloned(), alias);
    }
    sources
}

pub(super) fn table_with_joins_default_qualifier(table: &TableWithJoins) -> Option<String> {
    let TableFactor::Table { name, alias, .. } = &table.relation else {
        return None;
    };
    alias
        .as_ref()
        .map(|alias| alias.name.value.clone())
        .or_else(|| object_name_parts(name).last().cloned())
}

fn collect_table_with_joins(
    table: &TableWithJoins,
    sources: &mut TableSources,
    cte_names: &BTreeSet<String>,
) {
    collect_table_factor(&table.relation, sources, cte_names);
    for join in &table.joins {
        let mut right_sources = TableSources::default();
        collect_table_factor(&join.relation, &mut right_sources, cte_names);
        match &join.join_operator {
            JoinOperator::Left(_) | JoinOperator::LeftOuter(_) => {
                right_sources.mark_outer_join_nullable();
            }
            JoinOperator::Right(_) | JoinOperator::RightOuter(_) => {
                sources.mark_outer_join_nullable();
            }
            JoinOperator::FullOuter(_) => {
                sources.mark_outer_join_nullable();
                right_sources.mark_outer_join_nullable();
            }
            _ => {}
        }
        sources.extend(right_sources);
    }
}

fn collect_table_factor(
    table: &TableFactor,
    sources: &mut TableSources,
    cte_names: &BTreeSet<String>,
) {
    match table {
        TableFactor::Table {
            name, alias, args, ..
        } => {
            let alias = alias.as_ref().map(|alias| alias.name.value.clone());
            let parts = object_name_parts(name);
            if parts.len() == 1 && cte_names.contains(&normalized_identifier(&parts[0])) {
                sources.insert_unsupported(parts.last().cloned(), alias);
            } else if args.is_none()
                && let Some(table_name) = main_table_name(&parts)
            {
                sources.insert_main(&table_name, alias, parts.len() == 2);
            } else {
                sources.insert_unsupported(parts.last().cloned(), alias);
            }
        }
        TableFactor::NestedJoin {
            table_with_joins,
            alias,
        } => {
            let mut nested_sources = TableSources::default();
            collect_table_with_joins(table_with_joins, &mut nested_sources, cte_names);
            sources.extend(nested_sources);
            if let Some(alias) = alias {
                sources.insert_unsupported(None, Some(alias.name.value.clone()));
            }
        }
        TableFactor::Derived { alias, .. }
        | TableFactor::TableFunction { alias, .. }
        | TableFactor::Function { alias, .. }
        | TableFactor::JsonTable { alias, .. } => {
            sources.insert_unsupported(None, alias.as_ref().map(|alias| alias.name.value.clone()));
        }
        _ => {}
    }
}

pub(super) fn object_name_parts(name: &ObjectName) -> Vec<String> {
    name.0
        .iter()
        .filter_map(|part| part.as_ident().map(|ident| ident.value.clone()))
        .collect()
}

pub(super) fn main_table_name(parts: &[String]) -> Option<String> {
    if parts.iter().any(|part| part.contains('.')) {
        return None;
    }
    match parts {
        [table] => Some(table.clone()),
        [schema, table] if schema.eq_ignore_ascii_case("main") => Some(table.clone()),
        _ => None,
    }
}

fn column_uses_explicit_main(qualifier: &str) -> bool {
    qualifier
        .split_once('.')
        .is_some_and(|(schema, _table)| schema.eq_ignore_ascii_case("main"))
}

#[cfg(test)]
fn unsupported_schema_qualifier(parts: &[String]) -> Option<String> {
    match parts {
        [schema, _table] if !schema.eq_ignore_ascii_case("main") => Some(schema.clone()),
        [schema, _table] if schema.eq_ignore_ascii_case("main") => None,
        [qualifier @ .., _table] if qualifier.len() > 1 => Some(qualifier.join(".")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{main_table_name, unsupported_schema_qualifier};

    fn parts(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn accepts_only_bare_or_explicit_main_schema_tables() {
        assert_eq!(
            main_table_name(&parts(&["users"])),
            Some("users".to_owned())
        );
        assert_eq!(
            main_table_name(&parts(&["main", "users"])),
            Some("users".to_owned())
        );
        assert_eq!(main_table_name(&parts(&["temp", "users"])), None);
        assert_eq!(main_table_name(&parts(&["attached", "users"])), None);
    }

    #[test]
    fn identifies_only_explicit_non_main_schema_qualifiers() {
        assert_eq!(unsupported_schema_qualifier(&parts(&["users"])), None);
        assert_eq!(
            unsupported_schema_qualifier(&parts(&["main", "users"])),
            None
        );
        assert_eq!(
            unsupported_schema_qualifier(&parts(&["temp", "users"])),
            Some("temp".to_owned())
        );
        assert_eq!(
            unsupported_schema_qualifier(&parts(&["catalog", "schema", "users"])),
            Some("catalog.schema".to_owned())
        );
    }
}
