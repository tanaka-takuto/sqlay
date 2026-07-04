/* @sqlay
{
  type: query
  id: typeMappingOverrides
  cardinality: one
}
*/
SELECT
  p.bigint_nn_col AS idNumber,
  p.decimal_18_4_nn_col AS decimalNumber,
  p.enum_nn_col AS enumState,
  p.enum_col AS nullableEnumState,
  p.set_nn_col AS setFlags,
  p.varchar_255_nn_col AS fixtureLabel,
  p.varchar_255_col AS nullableFixtureLabel,
  p.decimal_18_4_col AS builderLocalAmount,
  c.varchar_32_nn_col AS qualifiedChildLabel
FROM fixture_all_column_type AS p
INNER JOIN sqlay.fixture_child AS c
  ON c.parent_bigint_nn_col = p.bigint_nn_col
WHERE p.decimal_18_4_nn_col >= /* @sqlay { type: param id: minimumAmount } */ 10.5000 /* @sqlay { type: paramEnd } */
  AND p.bigint_nn_col = /* @sqlay { type: param id: parentId } */ 1 /* @sqlay { type: paramEnd } */
LIMIT 1;

/* @sqlay
{
  type: mutation
  id: typeMappingOverrideRows
}
*/
INSERT INTO sqlay.fixture_child (
  child_bigint_nn_col,
  parent_bigint_nn_col,
  varchar_32_nn_col,
  decimal_12_2_nn_col
) VALUES
/* @sqlay { type: repeat id: rows separator: "," } */
(
  /* @sqlay { type: param id: childId } */ 700 /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: parentId } */ 1 /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: childLabel } */ 'child-type-mapping' /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: childAmount } */ 12.34 /* @sqlay { type: paramEnd } */
)
/* @sqlay { type: repeatEnd } */;

/* @sqlay
{
  type: fragment
  id: typeMappingFixtureLabel
}
*/
AND p.varchar_255_nn_col = /* @sqlay { type: param id: fixtureLabel } */ 'varchar-255-nn-a' /* @sqlay { type: paramEnd } */

/* @sqlay
{
  type: query
  id: typeMappingOverrideSlotSearch
}
*/
SELECT
  p.bigint_nn_col AS idNumber,
  p.enum_nn_col AS enumState
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col IS NOT NULL
/* @sqlay { type: slot id: labelFilter targets: [typeMappingFixtureLabel] } */
ORDER BY p.bigint_nn_col;
