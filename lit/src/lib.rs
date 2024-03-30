pub mod model;
pub mod query_set;

pub use model::Model;
pub use query_set::QuerySet;

#[cfg(test)]
mod tests;

pub type Result<T> = anyhow::Result<T>;
