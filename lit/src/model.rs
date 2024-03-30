use marker::PhantomData;
use std::fmt::{Display, Formatter};
use std::marker;
use std::sync::RwLock;

use rusqlite::{Connection, ToSql};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Type, ValueRef};

use crate::query_set::QuerySet;

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

impl From<i64> for SqliteValue {
    fn from(value: i64) -> Self {
        Self::INTEGER(value)
    }
}

impl From<bool> for SqliteValue {
    fn from(value: bool) -> Self {
        Self::INTEGER(value.into())
    }
}

impl From<f64> for SqliteValue {
    fn from(value: f64) -> Self {
        Self::REAL(value)
    }
}

impl From<String> for SqliteValue {
    fn from(value: String) -> Self {
        Self::TEXT(value)
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

static FILEPATH: RwLock<Option<String>> = RwLock::new(None);

pub static INIT_STMT_REGISTRY: RwLock<Vec<String>> = RwLock::new(Vec::new());

fn new_db_connection() -> crate::Result<Connection> {
    let filepath = FILEPATH
        .read()
        .unwrap()
        .clone()
        .unwrap_or("db.sqlite".to_string());
    Ok(rusqlite::Connection::open(filepath)?)
}

pub fn setup_db(path: &str) -> crate::Result<()> {
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

    fn objects() -> QuerySet<Self> {
        QuerySet {
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
            .map(|ModelField { name, col_type, .. }| format!("{name} {col_type}"))
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

    fn save(&mut self) -> crate::Result<()> {
        *self = Self::objects().save(self)?;
        Ok(())
    }
}
