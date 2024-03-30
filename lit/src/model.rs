use marker::PhantomData;
use std::fmt::{Display, Formatter};
use std::marker;
use std::sync::RwLock;

use rusqlite::types::ToSqlOutput;
use rusqlite::{Connection, params_from_iter, ToSql};

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
pub struct ModelField<Model: ModelStruct> {
    pub name: &'static str,
    pub col_type: SqliteColumnType,
    pub _marker: PhantomData<Model>,
}

impl<Model: ModelStruct> ModelField<Model> {
    pub fn to_sql(&self) -> String {
        let Self { name, col_type, .. } = self;
        format!("{name} {col_type}")
    }
}

pub struct ModelFields<Model: ModelStruct>(pub Vec<ModelField<Model>>);

pub struct Objects<Model: ModelStruct> {
    connection: rusqlite::Connection,
    _marker: PhantomData<Model>,
}

impl<Model: ModelStruct> Objects<Model> {
    pub fn insert(&self, m: Model) -> rusqlite::Result<()> {
        let table_name = Model::table_name();
        let placeholders = Model::fields()
            .0
            .iter()
            .map(|_| "?".to_string())
            .collect::<Vec<_>>()
            .join(", ");
        self.connection.execute(
            &format!(r#"INSERT INTO {table_name} VALUES ({placeholders});"#),
            params_from_iter(m.as_params()),
        )?;
        Ok(())
    }
}

static FILEPATH: RwLock<Option<String>> = RwLock::new(None);

pub static INIT_STMT_REGISTRY: RwLock<Vec<String>> = RwLock::new(Vec::new());

fn new_db_connection() -> rusqlite::Result<Connection> {
    let filepath = FILEPATH
        .read()
        .unwrap()
        .clone()
        .unwrap_or("db.sqlite".to_string());
    rusqlite::Connection::open(filepath)
}

pub fn setup_db(path: &str) -> rusqlite::Result<()> {
    *FILEPATH.write().unwrap() = Some(path.to_string());
    let connection = new_db_connection()?;
    for stmt in INIT_STMT_REGISTRY.read().unwrap().iter() {
        connection.execute(stmt, ())?;
    }
    Ok(())
}

pub trait ModelStruct: Sized {
    fn model_name() -> &'static str;

    fn fields() -> ModelFields<Self>;

    fn as_params(&self) -> Vec<SqliteValue>;

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
            .map(ModelField::to_sql)
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            r#"
                CREATE TABLE IF NOT EXISTS {table_name} 
                (
                    {fields_inner}                    
                );
            "#
        )
    }
}
