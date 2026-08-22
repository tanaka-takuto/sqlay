PRAGMA foreign_keys = OFF;

DROP TABLE IF EXISTS field_journal_observation_tags;
DROP TABLE IF EXISTS field_journal_observations;
DROP TABLE IF EXISTS field_journal_sites;

CREATE TABLE field_journal_sites (
  id TEXT NOT NULL PRIMARY KEY,
  name TEXT NOT NULL,
  latitude REAL NOT NULL,
  longitude REAL NOT NULL
);

CREATE TABLE field_journal_observations (
  id TEXT NOT NULL PRIMARY KEY,
  site_id TEXT NOT NULL,
  species_name TEXT NOT NULL,
  observed_at TEXT NOT NULL,
  individual_count INTEGER NOT NULL,
  notes TEXT NULL,
  reviewed BOOLEAN NOT NULL,
  FOREIGN KEY (site_id) REFERENCES field_journal_sites (id)
);

CREATE TABLE field_journal_observation_tags (
  observation_id TEXT NOT NULL,
  tag TEXT NOT NULL,
  PRIMARY KEY (observation_id, tag),
  FOREIGN KEY (observation_id)
    REFERENCES field_journal_observations (id)
    ON DELETE CASCADE
);

PRAGMA foreign_keys = ON;
