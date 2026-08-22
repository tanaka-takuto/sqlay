import { DatabaseSync } from "node:sqlite";

import {
  findObservationById,
  listSiteObservations,
} from "./generated/sql/observations";
import {
  addObservationTags,
  createObservation,
  deleteDraftObservation,
  markObservationReviewed,
} from "./generated/sql/mutations";

const listWithoutQuickFilter = listSiteObservations({ siteId: "site-wetland" });
assertEqual(
  normalizeSql(listWithoutQuickFilter.sql),
  "SELECT o.id AS observationId, s.name AS siteName, o.species_name AS speciesName, o.observed_at AS observedAt, o.individual_count AS individualCount, o.notes AS notes, o.reviewed AS reviewed FROM field_journal_observations AS o INNER JOIN field_journal_sites AS s ON s.id = o.site_id WHERE o.site_id = ? ORDER BY o.observed_at DESC, o.id;",
);
assertDeepEqual(listWithoutQuickFilter.params, ["site-wetland"]);

const listUnreviewed = listSiteObservations({
  siteId: "site-wetland",
  quickFilter: { $fragment: "unreviewedOnly" },
});
assertEqual(
  normalizeSql(listUnreviewed.sql),
  "SELECT o.id AS observationId, s.name AS siteName, o.species_name AS speciesName, o.observed_at AS observedAt, o.individual_count AS individualCount, o.notes AS notes, o.reviewed AS reviewed FROM field_journal_observations AS o INNER JOIN field_journal_sites AS s ON s.id = o.site_id WHERE o.site_id = ? AND o.reviewed = FALSE ORDER BY o.observed_at DESC, o.id;",
);
assertDeepEqual(listUnreviewed.params, ["site-wetland"]);

const listBySpecies = listSiteObservations({
  siteId: "site-wetland",
  quickFilter: {
    $fragment: "bySpeciesName",
    speciesName: "Common Kingfisher",
  },
});
assertEqual(
  normalizeSql(listBySpecies.sql),
  "SELECT o.id AS observationId, s.name AS siteName, o.species_name AS speciesName, o.observed_at AS observedAt, o.individual_count AS individualCount, o.notes AS notes, o.reviewed AS reviewed FROM field_journal_observations AS o INNER JOIN field_journal_sites AS s ON s.id = o.site_id WHERE o.site_id = ? AND o.species_name = ? ORDER BY o.observed_at DESC, o.id;",
);
assertDeepEqual(listBySpecies.params, ["site-wetland", "Common Kingfisher"]);

const listByIds = listSiteObservations({
  siteId: "site-wetland",
  quickFilter: {
    $fragment: "byObservationIds",
    observationIds: [
      { observationId: "obs-001" },
      { observationId: "obs-002" },
    ],
  },
});
assertEqual(
  normalizeSql(listByIds.sql),
  "SELECT o.id AS observationId, s.name AS siteName, o.species_name AS speciesName, o.observed_at AS observedAt, o.individual_count AS individualCount, o.notes AS notes, o.reviewed AS reviewed FROM field_journal_observations AS o INNER JOIN field_journal_sites AS s ON s.id = o.site_id WHERE o.site_id = ? AND o.id IN ( ? , ? ) ORDER BY o.observed_at DESC, o.id;",
);
assertDeepEqual(listByIds.params, ["site-wetland", "obs-001", "obs-002"]);

const detail = findObservationById({ observationId: "obs-001" });
assertEqual(
  normalizeSql(detail.sql),
  "SELECT o.id AS observationId, s.name AS siteName, o.species_name AS speciesName, o.observed_at AS observedAt, o.individual_count AS individualCount, o.notes AS notes, o.reviewed AS reviewed FROM field_journal_observations AS o INNER JOIN field_journal_sites AS s ON s.id = o.site_id WHERE o.id = ? LIMIT 1;",
);
assertDeepEqual(detail.params, ["obs-001"]);

const create = createObservation({
  observationId: "obs-100",
  siteId: "site-forest",
  speciesName: "Sika Deer",
  observedAt: "2026-08-22T05:45:00+09:00",
  individualCount: 3,
  notes: null,
  reviewed: false,
});
assertEqual(
  normalizeSql(create.sql),
  "INSERT INTO field_journal_observations ( id, site_id, species_name, observed_at, individual_count, notes, reviewed ) VALUES ( ?, ?, ?, ?, ?, ?, ? );",
);
assertDeepEqual(create.params, [
  "obs-100",
  "site-forest",
  "Sika Deer",
  "2026-08-22T05:45:00+09:00",
  3,
  null,
  0,
]);

const markReviewed = markObservationReviewed({
  reviewed: true,
  observationId: "obs-100",
});
assertEqual(
  normalizeSql(markReviewed.sql),
  "UPDATE field_journal_observations SET reviewed = ? WHERE field_journal_observations.id = ?;",
);
assertDeepEqual(markReviewed.params, [1, "obs-100"]);

const deleteDraft = deleteDraftObservation({ observationId: "obs-100" });
assertEqual(
  normalizeSql(deleteDraft.sql),
  "DELETE FROM field_journal_observations WHERE field_journal_observations.id = ? AND field_journal_observations.reviewed = FALSE;",
);
assertDeepEqual(deleteDraft.params, ["obs-100"]);

const addTags = addObservationTags({
  tags: [
    { observationId: "obs-100", tag: "forest" },
    { observationId: "obs-100", tag: "mammal" },
  ],
});
assertEqual(
  normalizeSql(addTags.sql),
  "INSERT INTO field_journal_observation_tags ( observation_id, tag ) VALUES ( ?, ? ) , ( ?, ? ) ;",
);
assertDeepEqual(addTags.params, [
  "obs-100",
  "forest",
  "obs-100",
  "mammal",
]);

const databaseFile = process.env.SQLAY_SQLITE_TEST_DATABASE_FILE;
if (databaseFile === undefined) {
  throw new Error("SQLAY_SQLITE_TEST_DATABASE_FILE is required");
}

const database = new DatabaseSync(databaseFile);
try {
  database.prepare(create.sql).run(...create.params);
  assertDeepEqual(
    database
      .prepare(
        "SELECT reviewed, typeof(reviewed) AS storageType FROM field_journal_observations WHERE id = ?",
      )
      .get("obs-100"),
    { reviewed: 0, storageType: "integer" },
  );

  database.prepare(markReviewed.sql).run(...markReviewed.params);
  assertDeepEqual(
    database
      .prepare(
        "SELECT reviewed, typeof(reviewed) AS storageType FROM field_journal_observations WHERE id = ?",
      )
      .get("obs-100"),
    { reviewed: 1, storageType: "integer" },
  );
} finally {
  database.close();
}

function normalizeSql(sql: string): string {
  return sql.replace(/\s+/g, " ").trim();
}

function assertEqual(actual: unknown, expected: unknown): void {
  if (actual !== expected) {
    throw new Error(`Expected ${String(expected)}, got ${String(actual)}`);
  }
}

function assertDeepEqual(actual: unknown, expected: unknown): void {
  const actualJson = JSON.stringify(actual);
  const expectedJson = JSON.stringify(expected);
  if (actualJson !== expectedJson) {
    throw new Error(`Expected ${expectedJson}, got ${actualJson}`);
  }
}
