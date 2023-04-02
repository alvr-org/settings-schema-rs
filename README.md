# settings-schema-rs

Derive macro for generating automatically a schema from Rust structures and enums. The schema can be serialized to JSON and/or used to generate a GUI.

[Example](https://github.com/zarik5/settings-schema-rs/blob/master/settings-schema/examples/example.rs)

## Node types

* Section (from `struct`). Fields can be marked with custom strings or flags. Unnamed fields are not supported.
* Choice (from `enum`). Up to one unnamed field per variant is supported. Can be marked with the `gui` attribute with `"drop_down"` or `"button_group"`.
* Optional (from `Option`). `None` is used when the content is "default" or calculated.
* Switch. Can be `Enabled` (with data) or `Disabled`.
* Boolean (from `bool`).
* Number (from `u/i 8/32/64/size` and `f32/f64`). Attribute `gui` can be `textbox` or `slider` (with sub attribtes `min`, `max`, `step` and `logarithmic`).
* Text (from `String`).
* Array (from `[X; N]`).
* Vector (from `Vec<X>`).
* Dictionary (from `Vec<(String, X)>`).

Attributes like `gui` can be applied to fields with compound types like `Vec<Switch<u64>>`.

Custom types with generic type arguments are not supported.

New `*Default` structures are automatically created to store default values. This is done to allow specifying the default data for all variants in a given enum.
