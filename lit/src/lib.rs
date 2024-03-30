pub mod model;
pub mod objects;

#[cfg(test)]
mod tests;

pub type Result<T> = anyhow::Result<T>;
