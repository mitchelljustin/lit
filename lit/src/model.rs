use marker::PhantomData;
use std::fmt::{Display, Formatter};
use std::marker;
use std::sync::RwLock;

use rusqlite::Connection;

#[derive(Debug)]
pub enum SqliteColumnType {
    TEXT,
    INTEGER,
    REAL,
    NULL,
    BLOB,
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

impl<Model: ModelStruct> Objects<Model> {}

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
