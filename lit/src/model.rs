use std::marker;

pub struct Objects<T: ModelStruct> {
    _marker: marker::PhantomData<T>,
}

impl<T: ModelStruct> Objects<T> {}

pub trait ModelStruct: Sized {
    fn model_name() -> &'static str;

    fn table_name() -> String {
        Self::model_name().to_lowercase() + "s"
    }

    fn objects() -> Objects<Self> {
        Objects {
            _marker: marker::PhantomData,
        }
    }
}
