use marker::PhantomData;
use std::marker;
use std::sync::RwLock;

use rusqlite::Connection;

use crate::query_set::QuerySet;

#[derive(Debug)]
pub struct ModelField<M: Model> {
    pub name: &'static str,
    pub col_type: rusqlite::types::Type,
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

    fn as_params(&self) -> Vec<rusqlite::types::Value>;

    fn from_row(
        row: impl IntoIterator<Item = rusqlite::types::Value>,
    ) -> rusqlite::types::FromSqlResult<Self>;

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
            .map(|ModelField { name, col_type, .. }| {
                format!("{name} {}", col_type.to_string().to_uppercase())
            })
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
        *self = Self::objects().upsert(self)?;
        Ok(())
    }
}
