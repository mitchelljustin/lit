use lit_derive::Model;
use lit::model::ModelStruct;

#[derive(Model)]
struct Person {
    id: i64,
    first_name: String,
    last_name: String,
    is_staff: bool,
    x: f64,
}

#[test]
fn test_simple_model() {
    assert_eq!(
        Person::table_name(),
        "persons",
    );
}
