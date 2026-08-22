mod expressions;
mod mutation;
mod query;
mod tables;

pub(super) use mutation::infer_mutation_params;
pub(super) use query::infer_query;
