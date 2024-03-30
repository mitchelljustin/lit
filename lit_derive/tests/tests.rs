use lit::model::{Model, setup_db};
use lit_derive::ModelStruct;

#[derive(Default, Clone, ModelStruct)]
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
    assert_eq!(Person::table_name(), "persons",);
    let mut yolo = Person {
        x: 0.99,
        first_name: "Yolo".to_string(),
        last_name: "Swag".to_string(),
        is_staff: false,
        ..Default::default()
    };
    yolo.save().unwrap();
}
