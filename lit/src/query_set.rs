use std::marker::PhantomData;

use rusqlite::{params_from_iter, Row};

use crate::model::Model;

pub struct QuerySet<M: Model> {
    pub connection: rusqlite::Connection,
    pub _marker: PhantomData<M>,
}

impl<M: Model> QuerySet<M> {
    pub fn insert(&self, instance: &M) -> crate::Result<M> {
        if let Some(id) = instance.id() {
            anyhow::bail!("instance already has an ID: {id}");
        }
        let table_name = M::table_name();
        let fields = Self::sql_fields();
        let placeholders = Self::sql_placeholders();
        self.connection.execute(
            &format!(r#"INSERT INTO {table_name} ({fields}) VALUES ({placeholders});"#),
            params_from_iter(&instance.as_params()[1..]),
        )?;
        let id = self.connection.last_insert_rowid();
        self.get(id).map_err(Into::into)
    }

    fn sql_fields() -> String {
        M::fields()
            .0
            .iter()
            .map(|f| f.name.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn sql_placeholders() -> String {
        (0..Self::field_count())
            .map(|_| "?".to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn upsert(&self, instance: &M) -> crate::Result<M> {
        if instance.id().is_none() {
            return self.insert(instance);
        }
        let table_name = M::table_name();
        let fields = Self::sql_fields();
        let placeholders = Self::sql_placeholders();
        let value_overwrites = M::fields()
            .0
            .iter()
            .map(|f| {
                let name = f.name;
                format!("{name} = excluded.{name}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        self.connection.execute(
            &format!(
                r#"
                    INSERT INTO {table_name} (id, {fields}) VALUES (?, {placeholders})
                    ON CONFLICT(id) DO UPDATE SET {value_overwrites};
                "#
            ),
            params_from_iter(instance.as_params()),
        )?;
        Ok(instance.clone())
    }

    fn _convert_row_to_model(row: &Row) -> crate::Result<M> {
        let columns = (0..Self::field_count() + 1)
            .map(|i| row.get::<_, rusqlite::types::Value>(i))
            .collect::<Result<Vec<_>, _>>()?;
        M::from_row(columns).map_err(Into::into)
    }

    pub fn field_count() -> usize {
        M::fields().0.len()
    }

    pub fn get(&self, id: i64) -> crate::Result<M> {
        self.find_by_id(id)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows.into())
    }

    pub fn find_by_id(&self, id: i64) -> crate::Result<Option<M>> {
        Ok(self.select("id=?", (id,))?.pop())
    }

    pub fn select(&self, r#where: &str, params: impl rusqlite::Params) -> crate::Result<Vec<M>> {
        let table_name = M::table_name();
        self.connection
            .prepare(&format!(
                r##"
                    SELECT * FROM {table_name}
                    WHERE {where};
                "##
            ))?
            .query(params)?
            .and_then(|row| Self::_convert_row_to_model(row))
            .collect()
    }
}
