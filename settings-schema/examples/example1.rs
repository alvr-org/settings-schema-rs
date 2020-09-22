use settings_schema::*;

#[derive(SettingsSchema)]
struct Test1 {
    // #[schema(higher_order(name = "hello", modifier(op = "hello")))]
    #[schema(advanced)]
    fjdksljf: bool,
}

#[derive(SettingsSchema)]
enum Test2 {
    Hello(#[schema(advanced)] i32),
    Hello2,
    Hello3 {},
}

fn main() {}
