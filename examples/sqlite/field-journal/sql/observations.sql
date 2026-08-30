/* @sqlay
{
  type: query
  id: listSiteObservations
}
*/
SELECT
  o.id AS observationId,
  s.name AS siteName,
  o.species_name AS speciesName,
  o.observed_at AS observedAt,
  o.individual_count AS individualCount,
  o.notes AS notes,
  o.reviewed AS reviewed
FROM field_journal_observations AS o
INNER JOIN field_journal_sites AS s
  ON s.id = o.site_id
WHERE o.site_id = /* @sqlay { type: param id: siteId } */ 'site-wetland' /* @sqlay { type: paramEnd } */
/* @sqlay { type: slot id: quickFilter targets: [unreviewedOnly, bySpeciesName, byObservationIds] } */
ORDER BY o.observed_at DESC, o.id;

/* @sqlay
{
  type: query
  id: findObservationById
  cardinality: one
}
*/
SELECT
  o.id AS observationId,
  s.name AS siteName,
  o.species_name AS speciesName,
  o.observed_at AS observedAt,
  o.individual_count AS individualCount,
  o.notes AS notes,
  o.reviewed AS reviewed
FROM field_journal_observations AS o
INNER JOIN field_journal_sites AS s
  ON s.id = o.site_id
WHERE o.id = /* @sqlay { type: param id: observationId } */ 'obs-001' /* @sqlay { type: paramEnd } */
LIMIT 1;
