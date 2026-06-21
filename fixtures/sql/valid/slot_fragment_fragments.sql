/* @sqlcomp
{
  type: fragment
  id: slotFixtureActiveOnly
}
*/
  AND p.bool_nn_col = TRUE

/* @sqlcomp
{
  type: fragment
  id: slotFixtureByText
}
*/
  AND p.varchar_320_nn_col = /* @sqlcomp { type: param id: textFilter } */
    'varchar-320-a'
    /* @sqlcomp { type: paramEnd } */

/* @sqlcomp
{
  type: fragment
  id: slotFixtureByAmount
}
*/
  AND EXISTS (
    SELECT 1
    FROM fixture_child AS c
    WHERE c.parent_bigint_nn_col = p.bigint_nn_col
      AND c.decimal_12_2_nn_col >= /* @sqlcomp { type: param id: minAmount valueType: decimal } */
        10.00
        /* @sqlcomp { type: paramEnd } */
  )

/* @sqlcomp
{
  type: fragment
  id: slotFixtureNullableCreated
}
*/
  AND p.datetime_6_col >= /* @sqlcomp { type: param id: createdAfter nullable: true } */
    '2026-01-02 03:04:05.123456'
    /* @sqlcomp { type: paramEnd } */

/* @sqlcomp
{
  type: fragment
  id: slotFixtureEqualsValue
}
*/
  = /* @sqlcomp { type: param id: value } */
    'varchar-320-a'
    /* @sqlcomp { type: paramEnd } */ THEN TRUE
  WHEN 1
