PRAGMA foreign_keys = ON;

DELETE FROM field_journal_observation_tags;
DELETE FROM field_journal_observations;
DELETE FROM field_journal_sites;

INSERT INTO field_journal_sites (
  id,
  name,
  latitude,
  longitude
) VALUES
  ('site-wetland', 'North Marsh Wetland', 43.0642, 141.3469),
  ('site-forest', 'Cedar Ridge Forest', 42.9849, 141.2464);

INSERT INTO field_journal_observations (
  id,
  site_id,
  species_name,
  observed_at,
  individual_count,
  notes,
  reviewed
) VALUES
  (
    'obs-001',
    'site-wetland',
    'Red-crowned Crane',
    '2026-08-20T06:15:00+09:00',
    2,
    'Two adults feeding near the eastern reed bed.',
    TRUE
  ),
  (
    'obs-002',
    'site-wetland',
    'Common Kingfisher',
    '2026-08-20T07:05:00+09:00',
    1,
    NULL,
    FALSE
  ),
  (
    'obs-003',
    'site-forest',
    'Sika Deer',
    '2026-08-21T05:40:00+09:00',
    4,
    'Tracks continued north beyond the observation point.',
    FALSE
  ),
  (
    'obs-004',
    'site-wetland',
    'Common Kingfisher',
    '2026-08-21T17:20:00+09:00',
    1,
    'Observed from the public boardwalk.',
    TRUE
  );

INSERT INTO field_journal_observation_tags (
  observation_id,
  tag
) VALUES
  ('obs-001', 'wetland'),
  ('obs-001', 'bird'),
  ('obs-002', 'bird'),
  ('obs-003', 'forest'),
  ('obs-003', 'mammal'),
  ('obs-004', 'bird');
