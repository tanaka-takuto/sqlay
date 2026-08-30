/* @sqlay
{
  type: mutation
  id: createObservation
}
*/
INSERT INTO field_journal_observations (
  id,
  site_id,
  species_name,
  observed_at,
  individual_count,
  notes,
  reviewed
) VALUES (
  /* @sqlay { type: param id: observationId } */ 'obs-100' /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: siteId } */ 'site-forest' /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: speciesName } */ 'Sika Deer' /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: observedAt } */ '2026-08-22T05:45:00+09:00' /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: individualCount } */ 3 /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: notes nullable: true } */ NULL /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: reviewed valueType: bool } */ FALSE /* @sqlay { type: paramEnd } */
);

/* @sqlay
{
  type: mutation
  id: markObservationReviewed
}
*/
UPDATE field_journal_observations
SET reviewed = /* @sqlay { type: param id: reviewed valueType: bool } */ TRUE /* @sqlay { type: paramEnd } */
WHERE field_journal_observations.id = /* @sqlay { type: param id: observationId } */ 'obs-100' /* @sqlay { type: paramEnd } */;

/* @sqlay
{
  type: mutation
  id: deleteDraftObservation
}
*/
DELETE FROM field_journal_observations
WHERE field_journal_observations.id = /* @sqlay { type: param id: observationId } */ 'obs-100' /* @sqlay { type: paramEnd } */
  AND field_journal_observations.reviewed = FALSE;

/* @sqlay
{
  type: mutation
  id: addObservationTags
}
*/
INSERT INTO field_journal_observation_tags (
  observation_id,
  tag
)
VALUES
/* @sqlay { type: repeat id: tags separator: ", " } */
(
  /* @sqlay { type: param id: observationId } */ 'obs-100' /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: tag } */ 'forest' /* @sqlay { type: paramEnd } */
)
/* @sqlay { type: repeatEnd } */;
