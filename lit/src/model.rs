use std::fmt::{Display, Formatter};
use std::marker;

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
        write!(f, "{}", match self {
            SqliteColumnType::INTEGER => "INTEGER",
            SqliteColumnType::REAL => "REAL",
            SqliteColumnType::TEXT => "TEXT",
            SqliteColumnType::NULL => "NULL",
            SqliteColumnType::BLOB => "BLOB",
        })
    }
}

#[derive(Debug)]
pub struct ModelField<Model: ModelStruct> {
    pub name: &'static str,
    pub col_type: SqliteColumnType,
    pub _marker: marker::PhantomData<Model>,
}

impl<Model: ModelStruct> ModelField<Model> {
    fn to_sql(&self) -> String {
        let Self { name, col_type, .. } = self;
        format!("{name} {col_type}")
    }
}

pub struct ModelFields<Model: ModelStruct>(pub Vec<ModelField<Model>>);

pub struct Objects<Model: ModelStruct> {
    _marker: marker::PhantomData<Model>,
}

impl<Model: ModelStruct> Objects<Model> {}

pub trait ModelStruct: Sized {
    fn model_name() -> &'static str;

    fn fields() -> ModelFields<Self>;

    fn table_name() -> String {
        Self::model_name().to_lowercase() + "s"
    }

    fn objects() -> Objects<Self> {
        Objects {
            _marker: marker::PhantomData,
        }
    }
}
