use std::marker::PhantomData;

use rusqlite::{params_from_iter, Row};

use crate::model::{Model, SqliteValue};

pub struct Objects<M: Model> {
    pub connection: rusqlite::Connection,
    pub _marker: PhantomData<M>,
}

impl<M: Model> Objects<M> {
    pub fn insert(&self, instance: &M) -> anyhow::Result<M> {
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

    pub fn save(&self, instance: &M) -> anyhow::Result<M> {
        if instance.id().is_none() {
            return self.insert(instance);
        }
        self.upsert(instance)?;
        Ok(instance.clone())
    }

    pub fn upsert(&self, instance: &M) -> anyhow::Result<()> {
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
        Ok(())
    }

    fn _convert_row_to_model(row: &Row) -> anyhow::Result<M> {
        let columns = (0..Self::field_count() + 1)
            .map(|i| row.get::<_, SqliteValue>(i))
            .collect::<Result<Vec<_>, _>>()?;
        let Some(model) = M::from_row(columns) else {
            anyhow::bail!("unable to convert row to model");
        };
        Ok(model)
    }

    pub fn field_count() -> usize {
        M::fields().0.len()
    }

    pub fn get(&self, id: i64) -> anyhow::Result<M> {
        self.select("id=?", (id,))?
            .pop()
            .ok_or(rusqlite::Error::QueryReturnedNoRows.into())
    }

    pub fn select(
        &self,
        r#where: &str,
        params: impl rusqlite::Params + Clone,
    ) -> anyhow::Result<Vec<M>> {
        let table_name = M::table_name();
        self.connection
            .prepare(&format!(
                r##"
                    SELECT * FROM {table_name}
                    WHERE {where}
                    LIMIT 500;
                "##
            ))?
            .query(params)?
            .and_then(|row| Self::_convert_row_to_model(row))
            .collect()
    }
}
