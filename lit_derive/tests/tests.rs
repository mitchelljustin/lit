use lit_derive::Model;
use lit::model::{setup_db, ModelStruct};

#[derive(Model)]
struct Person {
    id: i64,
    first_name: String,
    last_name: String,
    is_staff: bool,
    x: f64,
}

fn init_db() {
    Person::register();
    setup_db("./out/test.sqlite").unwrap();
}

#[test]
fn test_simple_model() {
    init_db();
    assert_eq!(
        Person::table_name(),
        "persons",
    );
}
