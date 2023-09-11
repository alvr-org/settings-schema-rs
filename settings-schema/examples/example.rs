#![allow(dead_code)]

use settings_schema::*;

#[derive(SettingsSchema)]
#[schema(collapsible)]
struct TestStruct {
    #[schema(gui(slider(min = 10, max = 100, step = 10, logarithmic)))]
    optional: Option<usize>,

    #[schema(strings(my_string = "Switch"))]
    switch: Switch<String>,

    #[schema(gui(slider(min = -0.5, max = 0.5, step = 0.1)), suffix = "m")]
    array: [f32; 2],

    vec: Vec<f32>,

    #[schema(flag = "advanced")]
    dictionary: Vec<(String, bool)>,
}

#[derive(SettingsSchema)]
#[schema(gui = "button_group")]
enum TestEnum {
    #[schema(strings(display_name = "First option"))]
    Variant,
    Value(i32),

    #[schema(collapsible)]
    Block {
        #[schema(strings(hint = "This is a test"))]
        test_struct: TestStruct,
    },
}

fn main() {
    let default = TestEnumDefault {
        variant: TestEnumDefaultVariant::Block,
        Value: 3,
        Block: TestEnumBlockDefault {
            gui_collapsed: true,
            test_struct: TestStructDefault {
                gui_collapsed: false,
                optional: OptionalDefault {
                    set: true,
                    content: 50,
                },
                switch: SwitchDefault {
                    enabled: false,
                    content: "test".into(),
                },
                array: [0.0, 0.2],
                vec: VectorDefault {
                    element: 0.0,
                    content: vec![],
                },
                dictionary: DictionaryDefault {
                    key: "key".into(),
                    value: false,
                    content: vec![("key 1".into(), false), ("key 2".into(), true)],
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
