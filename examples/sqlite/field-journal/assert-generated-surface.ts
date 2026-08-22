import {
  type findObservationById_Input,
  type findObservationById_Output,
  findObservationById,
  type listSiteObservations_Input,
  type listSiteObservations_Output,
  listSiteObservations,
} from "./generated/sql/observations";
import {
  type addObservationTags_Input,
  addObservationTags,
  type createObservation_Input,
  createObservation,
  type deleteDraftObservation_Input,
  deleteDraftObservation,
  type markObservationReviewed_Input,
  markObservationReviewed,
} from "./generated/sql/mutations";

type Assert<T extends true> = T;
type IsExact<A, B> =
  (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2
    ? (<T>() => T extends B ? 1 : 2) extends <T>() => T extends A ? 1 : 2
      ? true
      : false
    : false;

type ListSiteObservationsInputContract = Assert<
  IsExact<
    listSiteObservations_Input,
    {
      siteId: string;
      quickFilter?:
        | { $fragment: "unreviewedOnly" }
        | { $fragment: "bySpeciesName"; speciesName: string }
        | {
          $fragment: "byObservationIds";
          observationIds: readonly [
            { observationId: string },
            ...{ observationId: string }[],
          ];
        };
    }
  >
>;
type ListSiteObservationsOutputContract = Assert<
  IsExact<
    listSiteObservations_Output,
    {
      observationId: string | null;
      siteName: string | null;
      speciesName: string | null;
      observedAt: string | null;
      individualCount: number | null;
      notes: string | null;
      reviewed: unknown | null;
    }[]
  >
>;
type ListSiteObservationsReturnContract = Assert<
  IsExact<
    ReturnType<typeof listSiteObservations>,
    { sql: string; params: readonly unknown[] }
  >
>;

type FindObservationByIdInputContract = Assert<
  IsExact<findObservationById_Input, { observationId: string }>
>;
type FindObservationByIdOutputContract = Assert<
  IsExact<
    findObservationById_Output,
    {
      observationId: string | null;
      siteName: string | null;
      speciesName: string | null;
      observedAt: string | null;
      individualCount: number | null;
      notes: string | null;
      reviewed: unknown | null;
    } | null
  >
>;
type FindObservationByIdReturnContract = Assert<
  IsExact<
    ReturnType<typeof findObservationById>,
    { sql: string; params: readonly [string] }
  >
>;

type CreateObservationInputContract = Assert<
  IsExact<
    createObservation_Input,
    {
      observationId: string;
      siteId: string;
      speciesName: string;
      observedAt: string;
      individualCount: number;
      notes: string | null;
      reviewed: boolean;
    }
  >
>;
type CreateObservationReturnContract = Assert<
  IsExact<
    ReturnType<typeof createObservation>,
    {
      sql: string;
      params: readonly [string, string, string, string, number, string | null, 0 | 1];
    }
  >
>;

type MarkObservationReviewedInputContract = Assert<
  IsExact<
    markObservationReviewed_Input,
    { reviewed: boolean; observationId: string }
  >
>;
type MarkObservationReviewedReturnContract = Assert<
  IsExact<
    ReturnType<typeof markObservationReviewed>,
    { sql: string; params: readonly [0 | 1, string] }
  >
>;

type DeleteDraftObservationInputContract = Assert<
  IsExact<deleteDraftObservation_Input, { observationId: string }>
>;
type DeleteDraftObservationReturnContract = Assert<
  IsExact<
    ReturnType<typeof deleteDraftObservation>,
    { sql: string; params: readonly [string] }
  >
>;

type AddObservationTagsInputContract = Assert<
  IsExact<
    addObservationTags_Input,
    {
      tags: readonly [
        { observationId: string; tag: string },
        ...{ observationId: string; tag: string }[],
      ];
    }
  >
>;
type AddObservationTagsReturnContract = Assert<
  IsExact<
    ReturnType<typeof addObservationTags>,
    { sql: string; params: readonly unknown[] }
  >
>;

export type FieldJournalSurfaceContracts = [
  ListSiteObservationsInputContract,
  ListSiteObservationsOutputContract,
  ListSiteObservationsReturnContract,
  FindObservationByIdInputContract,
  FindObservationByIdOutputContract,
  FindObservationByIdReturnContract,
  CreateObservationInputContract,
  CreateObservationReturnContract,
  MarkObservationReviewedInputContract,
  MarkObservationReviewedReturnContract,
  DeleteDraftObservationInputContract,
  DeleteDraftObservationReturnContract,
  AddObservationTagsInputContract,
  AddObservationTagsReturnContract,
];
