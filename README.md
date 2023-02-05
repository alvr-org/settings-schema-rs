# settings-schema-rs

Derive macro for generating automatically a schema from Rust structures and enums. The schema can be serialized to JSON and/or used to generate a GUI.

[Example](https://github.com/zarik5/settings-schema-rs/blob/master/settings-schema/examples/example.rs)

## Node types

* Section (from `struct`). Contains fields that can be marked as advanced setting. Unnamed fields are not supported.
* Choice (from `enum`). Up to one unnamed field per variant is supported. Can be marked with the `gui` attribute with `"drop_down"` or `"button_group"`.
* Optional (from `Option`). Describes data that can be omitted.
* Switch. Can be `Enabled` (with data) or `Disabled`. The content can be set to advanced.
* Boolean (from `bool`).
* Integer (from `u/i 8/32/64/size`) and Float (from `f32/f64`). Can be marked with `min`, `max`, `step`, `gui` attributes. `gui` can be equal to `"text_box"`, `"up_down"` or `"slider"`.
* Text (from `String`).
* Array (from `[X; N]`).
* Vector (from `Vec<X>`).
* Dictionary (from `Vec<(String, X)>`).

Attributes like `min`, `gui` can be applied to fields with compound types like `Vec<Switch<u64>>`.

Custom types with generic type arguments are not supported.

New `*Default` structures are automatically created to store default values. This is done to allow specifying the default data for all variants in a given enum.
