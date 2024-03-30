use marker::PhantomData;
use std::fmt::{Display, Formatter};
use std::marker;
use std::sync::RwLock;

use rusqlite::{Connection, params_from_iter, Row, ToSql};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Type, ValueRef};

#[derive(Debug, Copy, Clone)]
pub enum SqliteColumnType {
    TEXT,
    INTEGER,
    REAL,
    NULL,
    BLOB,
}

#[derive(Debug, Clone)]
pub enum SqliteValue {
    TEXT(String),
    INTEGER(i64),
    REAL(f64),
}

impl ToSql for SqliteValue {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match self {
            SqliteValue::TEXT(v) => v.to_sql(),
            SqliteValue::INTEGER(v) => v.to_sql(),
            SqliteValue::REAL(v) => v.to_sql(),
        }
    }
}

impl FromSql for SqliteValue {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.data_type() {
            Type::Integer => Ok(SqliteValue::INTEGER(value.as_i64().unwrap())),
            Type::Real => Ok(SqliteValue::REAL(value.as_f64().unwrap())),
            Type::Text => Ok(SqliteValue::TEXT(value.as_str().unwrap().to_string())),
            _ => Err(FromSqlError::Other("unsupported".into())),
        }
    }
}

impl From<SqliteValue> for Option<f64> {
    fn from(value: SqliteValue) -> Self {
        let SqliteValue::REAL(v) = value else {
            return None;
        };
        Some(v)
    }
}

impl From<SqliteValue> for Option<i64> {
    fn from(value: SqliteValue) -> Self {
        let SqliteValue::INTEGER(v) = value else {
            return None;
        };
        Some(v)
    }
}

impl From<SqliteValue> for Option<String> {
    fn from(value: SqliteValue) -> Self {
        let SqliteValue::TEXT(v) = value else {
            return None;
        };
        Some(v)
    }
}

impl From<SqliteValue> for Option<bool> {
    fn from(value: SqliteValue) -> Self {
        let SqliteValue::INTEGER(v) = value else {
            return None;
        };
        Some(v != 0)
    }
}

impl Display for SqliteColumnType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SqliteColumnType::INTEGER => "INTEGER",
                SqliteColumnType::REAL => "REAL",
                SqliteColumnType::TEXT => "TEXT",
                SqliteColumnType::NULL => "NULL",
                SqliteColumnType::BLOB => "BLOB",
            }
        )
    }
}

#[derive(Debug)]
pub struct ModelField<M: Model> {
    pub name: &'static str,
    pub col_type: SqliteColumnType,
    pub _marker: PhantomData<M>,
}

pub struct ModelFields<M: Model>(pub Vec<ModelField<M>>);

pub struct Objects<M: Model> {
    connection: rusqlite::Connection,
    _marker: PhantomData<M>,
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
        self.get(id)
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

    fn _convert_row_to_model(row: &Row) -> rusqlite::Result<M> {
        let columns = (0..Self::field_count() + 1)
            .map(|i| row.get::<_, SqliteValue>(i))
            .collect::<Result<Vec<_>, _>>()?;
        let Some(model) = M::from_row(columns) else {
            return Err(rusqlite::Error::InvalidQuery);
        };
        Ok(model)
    }

    pub fn field_count() -> usize {
        M::fields().0.len()
    }

    pub fn get(&self, id: i64) -> anyhow::Result<M> {
        let table_name = M::table_name();
        Ok(self.connection.query_row(
            &format!("SELECT * FROM {table_name} WHERE id=?;"),
            (id,),
            Self::_convert_row_to_model,
        )?)
    }
}

static FILEPATH: RwLock<Option<String>> = RwLock::new(None);

pub static INIT_STMT_REGISTRY: RwLock<Vec<String>> = RwLock::new(Vec::new());

fn new_db_connection() -> anyhow::Result<Connection> {
    let filepath = FILEPATH
        .read()
        .unwrap()
        .clone()
        .unwrap_or("db.sqlite".to_string());
    Ok(rusqlite::Connection::open(filepath)?)
}

pub fn setup_db(path: &str) -> anyhow::Result<()> {
    *FILEPATH.write().unwrap() = Some(path.to_string());
    let connection = new_db_connection()?;
    for stmt in INIT_STMT_REGISTRY.read().unwrap().iter() {
        connection.execute(stmt, ())?;
    }
    Ok(())
}

pub trait Model: Sized + Clone {
    fn id(&self) -> Option<i64>;

    fn model_name() -> &'static str;

    fn fields() -> ModelFields<Self>;

    fn as_params(&self) -> Vec<SqliteValue>;

    fn from_row(row: impl IntoIterator<Item = SqliteValue>) -> Option<Self>;

    fn table_name() -> String {
        Self::model_name().to_lowercase() + "s"
    }

    fn objects() -> Objects<Self> {
        Objects {
            connection: new_db_connection().unwrap(),
            _marker: PhantomData,
        }
    }

    fn register() {
        INIT_STMT_REGISTRY.write().unwrap().push(Self::init_stmt());
    }

    fn init_stmt() -> String {
        let table_name = Self::table_name();
        let fields_inner = Self::fields()
            .0
            .iter()
            .map(|f| format!("{} {}", f.name, f.col_type))
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            r#"
                CREATE TABLE IF NOT EXISTS {table_name} 
                (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    {fields_inner}                    
                );
            "#
        )
    }

    fn save(&mut self) -> anyhow::Result<()> {
        *self = Self::objects().save(self)?;
        Ok(())
    }
}
