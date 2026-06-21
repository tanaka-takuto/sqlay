/* @sqlcomp
{
  type: fragment
  id: slotFixtureByState
}
*/
  AND p.char_16_nn_col = /* @sqlcomp { type: param id: state } */
    'state_a'
    /* @sqlcomp { type: paramEnd } */

/* @sqlcomp
{
  type: query
  id: slotFragmentSearch
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.varchar_320_nn_col AS varchar320NnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col >= /* @sqlcomp { type: param id: minId } */
  1
  /* @sqlcomp { type: paramEnd } */
/* @sqlcomp { type: slot id: filter targets: [slotFixtureActiveOnly, slotFixtureByText, slotFixtureByAmount, slotFixtureNullableCreated, slotFixtureByState] } */
  AND EXISTS (
    SELECT 1
    FROM fixture_child AS c
    WHERE c.parent_bigint_nn_col = p.bigint_nn_col
/* @sqlcomp { type: slot id: repeatFilter targets: [slotFixtureActiveOnly] } */
  )
/* @sqlcomp { type: slot id: repeatFilter targets: [slotFixtureActiveOnly] } */
ORDER BY p.bigint_nn_col;

/* @sqlcomp
{
  type: query
  id: slotFragmentContextualParam
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.int_nn_col AS intNnCol,
  p.varchar_320_nn_col AS varchar320NnCol
FROM fixture_all_column_type AS p
WHERE CASE
  WHEN p.varchar_320_nn_col
/* @sqlcomp { type: slot id: textComparator targets: [slotFixtureEqualsValue] } */
  IS NULL THEN TRUE
  ELSE TRUE
END
  AND CASE
    WHEN p.int_nn_col
/* @sqlcomp { type: slot id: numberComparator targets: [slotFixtureEqualsValue] } */
    IS NULL THEN TRUE
    ELSE TRUE
  END
ORDER BY p.bigint_nn_col;
