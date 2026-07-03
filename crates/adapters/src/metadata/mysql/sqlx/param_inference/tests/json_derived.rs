use super::super::resolve_result_column_hints;
use super::*;

#[test]
fn identifies_json_derived_projection_expressions() {
    let query = raw_param_query(
        "SELECT JSON_EXTRACT(metadata, '$.tier') AS tierJson, JSON_UNQUOTE(JSON_EXTRACT(metadata, '$.tier')) AS tier, CAST(JSON_UNQUOTE(JSON_EXTRACT(metadata, '$.tier')) AS CHAR(255)) AS tierText FROM orders;",
        Vec::<core::ParamUsage>::new(),
    );

    let result_hints = resolve_result_column_hints(&query, &[])
        .expect("JSON projection expression detection should parse valid SELECT");

    assert_eq!(result_hints.json_derived_columns, [true, true, false]);
}
