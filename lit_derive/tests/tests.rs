use lit::model::{Model, setup_db};
use lit_derive::ModelStruct;

#[derive(Default, Debug, Clone, ModelStruct)]
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
fn test_simple_model() -> lit::Result<()> {
    init_db();
    assert_eq!(Person::table_name(), "persons",);
    let mut yolo = Person {
        first_name: "Yolo".to_string(),
        last_name: "Swag".to_string(),
        ..Default::default()
    };
    assert!(yolo.id().is_none());
    yolo.save()?;
    assert!(yolo.id().is_some());
    yolo.last_name = "Swaggins".to_string();
    yolo.save()?;
    let yolos = Person::objects().find_by_first_name("Yolo");
    println!("{yolos:#?}");
    Ok(())
}
