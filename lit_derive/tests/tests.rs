use lit::model::{Model, setup_db};
use lit_derive::ModelStruct;

#[derive(Default, Debug, Clone, ModelStruct)]
struct Person {
    id: i64,

    first_name: String,
    last_name: String,
    is_staff: bool,

    #[foreign_key(Company)]
    company_id: i64,
}

#[derive(Default, Debug, Clone, ModelStruct)]
struct Company {
    id: i64,
    name: String,
}

fn init_database() {
    Person::register();
    Company::register();
    setup_db("./out/test.sqlite").unwrap();
}

#[test]
fn test_simple_model() -> lit::Result<()> {
    init_database();
    assert_eq!(Person::table_name(), "person",);
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
    let mut yolocorp = Company {
        name: "Yolocorp".to_string(),
        ..Default::default()
    };
    yolocorp.save()?;
    yolo.company_id = yolocorp.id;
    assert_eq!(&yolo.company()?.unwrap().name, &yolocorp.name,);
    Ok(())
}
