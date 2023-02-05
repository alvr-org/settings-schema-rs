#![allow(dead_code)]

use settings_schema::*;

#[derive(SettingsSchema)]
struct TestStruct {
    #[schema(min = 10, max = 100, step = 10, gui = "up_down")]
    optional: Option<usize>,

    #[schema(strings(my_string = "Switch"))]
    switch: Switch<String>,

    #[schema(min = -0.5, max = 0.5, step = 0.1, gui = "slider")]
    array: [f32; 2],

    dictionary: Vec<(String, bool)>,
}

#[derive(SettingsSchema)]
#[schema(gui = "button_group")]
enum TestEnum {
    #[schema(strings(display_name = "First option"))]
    Variant,
    Value(i32),
    Block {
        #[schema(strings(hint = "This is a test"))]
        test_struct: TestStruct,
    },
}

fn main() {
    let default = TestEnumDefault {
        variant: TestEnumDefaultVariant::Block,
        value: 3,
        block: TestEnumBlockDefault {
            test_struct: TestStructDefault {
                optional: OptionalDefault {
                    set: true,
                    content: 50,
                },
                switch: SwitchDefault {
                    enabled: false,
                    content: "test".into(),
                },
                array: [0.0, 0.2],
                dictionary: DictionaryDefault {
                    key: "flag".into(),
                    value: false,
                    content: vec![("flag 1".into(), false), ("flag 2".into(), true)],
                },
            },
        },
    };

    println!(
        "default:\n{}\n",
        serde_json::to_string_pretty(&default).unwrap()
    );

    let schema = TestEnum::schema(default);

    println!(
        "schema:\n{}\n",
        serde_json::to_string_pretty(&schema).unwrap()
    );
}
