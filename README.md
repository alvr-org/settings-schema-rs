# settings-schema-rs

Derive macro for generating automatically a schema from Rust structures and enums. The schema can be serialized to JSON and/or used to generate a GUI.

## Example

```rust
#[derive(SettingsSchema)]
enum ChoiceTest {
    A,

    #[schema(min = -10, max = 10, step = 2, gui = "Slider")]
    B(i32),

    C {
        #[schema(advanced)]
        text_c: String,
    },
}

fn choice_test_default() -> ChoiceTestDefault { ... }

println!(
    "{}",
    serde_json::to_string_pretty(
        &choice_test_schema(choice_test_default())
    )
    .unwrap()
);

```

Result:

```json
{
  "type": "Choice",
  "content": {
    "default": "B",
    "variants": [
      [
        "A",
        null
      ],
      [
        "B",
        {
          "advanced": false,
          "content": {
            "type": "Integer",
            "content": {
              "default": 10,
              "min": -10,
              "max": 10,
              "step": 2,
              "gui": "Slider"
            }
          }
        }
      ],
      [
        "C",
        {
          "advanced": false,
          "content": {
            "type": "Section",
            "content": {
              "entries": [
                [
                  "text_c",
                  {
                    "advanced": true,
                    "content": {
                      "type": "Text",
                      "content": {
                        "default": "Hello World"
                      }
                    }
                  }
                ]
              ]
            }
          }
        }
      ]
    ]
  }
}
```

In production [example](https://github.com/alvr-org/ALVR/blob/master/alvr/session/src/settings.rs).

## Node types

* Section (from `struct`). Contains fields that can be marked as advanced setting. Unnamed fields are not supported.
* Choice (from `enum`). Up to one unnamed field per variant is supported.
* Optional (from `Option`). Describes data that can be omitted.
* Switch. Can be `Enabled` (with data) or `Disabled`. The content can be set to advanced.
* Boolean (from `bool`).
* Integer (from `u/i 8/32/64`) and Float (from `f32/f64`). Can be marked with `min`, `max`, `step`, `gui` attributes. `gui` can be equal to `"TextBox"`, `"Updown"` or `"Slider"`.
* Text (from `String`).
* Array (from `[X; N]`).
* Vector (from `Vec<X>`).
* Dictionary (from `Vec<(String, X)>`).

A field with no data can be inserted with `#[schema(placeholder = "x")]`.

Attributes like `min`, `gui`, `switch_advanced` can be applied to fields with compound types like `Vec<Switch<u64>>`.

Custom types with generic type arguments are not supported.

New `*Default` structures are automatically created to store default values. This is done to allow specifying the default data for all variants in a given enum.

## Features

* (Optional) `"rename_camel_case"` or `"rename_snake_case"`: use `serde(rename_all)` on this crate structures. User structures and enums must still use `serde(rename_all)` for correct de/serialization.
