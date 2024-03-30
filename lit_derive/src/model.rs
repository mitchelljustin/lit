use std::marker;

pub struct Objects<T: ModelStruct> {
    _marker: marker::PhantomData<T>,
}

impl<T: ModelStruct> Objects<T> {
    pub fn table_name() -> String {
        T::model_name().to_lowercase() + "s"
    }
}

pub trait ModelStruct: Sized {
    fn model_name() -> &'static str;
    
    fn objects() -> Objects<Self> {
        Objects {
            _marker: marker::PhantomData,
        }
    }
}
