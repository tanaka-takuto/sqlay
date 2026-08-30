use sqlay_adapters::dialect_sqlite::SqliteDialectAnalyzer;
use sqlay_app::DialectAnalyzer;
use sqlay_core as core;

#[test]
fn sqlite_dialect_analyzer_is_public_and_accepts_select() {
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT [id] FROM [users];".to_owned(),
    );

    let analysis = SqliteDialectAnalyzer
        .analyze(&query)
        .expect("SQLite SELECT should be accepted");

    assert_eq!(analysis.cardinality(), core::Cardinality::Many);
}
