use serde_json::*;
use settings_schema::*;

#[derive(SettingsSchema)]
struct Test1 {
    #[schema(higher_order(
        name = "hello",
        data(boolean(default = false)),
        modifier(
            target = r#"hello1["0"].hello2"#,
            update_op = "assign",
            expr = "{} * {}",
            var = "input",
            var = "hello3"
        )
    ))]
    #[schema(advanced)]
    test: bool,

    #[schema(min = 10_f32, gui = "up_down")]
    float: f32,
}

// #[derive(SettingsSchema)]
// enum Test2 {
//     Hello(#[schema(advanced)] i32),
//     Hello2,
//     Hello3 {},
// }

fn main() {
    // let test1 = Test1 {test: true};
    let schema = Test1::schema(Test1Default {
        test: false,
        float: 3.,
    });

    println!("{}", serde_json::to_string_pretty(&schema).unwrap())
}
