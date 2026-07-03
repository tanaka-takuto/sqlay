declare module "@fixtures/domain-types" {
  export type FixtureAmount = number & { readonly __brand: "FixtureAmount" };
  export type FixtureLabel = string & { readonly __brand: "FixtureLabel" };
  export type QualifiedChildLabel = string & {
    readonly __brand: "QualifiedChildLabel";
  };
}
