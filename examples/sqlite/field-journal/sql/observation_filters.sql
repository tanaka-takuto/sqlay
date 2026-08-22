/* @sqlay
{
  type: fragment
  id: unreviewedOnly
}
*/
  AND o.reviewed = FALSE

/* @sqlay
{
  type: fragment
  id: bySpeciesName
}
*/
  AND o.species_name = /* @sqlay { type: param id: speciesName } */ 'Common Kingfisher' /* @sqlay { type: paramEnd } */

/* @sqlay
{
  type: fragment
  id: byObservationIds
}
*/
  AND o.id IN (
    /* @sqlay { type: repeat id: observationIds separator: ", " } */
    /* @sqlay { type: param id: observationId } */ 'obs-001' /* @sqlay { type: paramEnd } */
    /* @sqlay { type: repeatEnd } */
  )
