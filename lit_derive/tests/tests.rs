use lit::Model;

#[derive(Model)]
struct Person {
    id: i64,
    first_name: String,
    last_name: String,
    is_staff: bool,
}

#[test]
fn test_simple_model() {
    println!("{swag}");
}
