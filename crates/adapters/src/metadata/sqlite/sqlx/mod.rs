//! sqlx-backed `SQLite` metadata adapter.

mod connection;
mod describe;
mod diagnostics;
mod inference;
mod result_mapping;
mod schema;

pub use describe::SqlxSqliteMetadataProvider;
