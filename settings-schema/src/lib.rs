use serde::{Deserialize, Serialize};

pub use settings_schema_derive::SettingsSchema;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum Switch<T> {
    Enabled(T),
    Disabled,
}

impl<T> Switch<T> {
    pub fn into_option(self) -> Option<T> {
        match self {
            Self::Enabled(t) => Some(t),
            Self::Disabled => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OptionalDefault<C> {
    pub set: bool,
    pub content: C,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SwitchDefault<C> {
    pub enabled: bool,
    pub content: C,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VectorDefault<C, D> {
    pub element: C,
    pub default: Vec<D>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DictionaryDefault<V, D> {
    pub key: String,
    pub value: V,
    pub default: Vec<(String, D)>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum NumericGuiType {
    TextBox,
    UpDown,
    Slider,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EntryData {
    pub advanced: bool,
    pub content: SchemaNode,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type", content = "content")]
pub enum SchemaNode {

    Section {
        entries: Vec<(String, Option<EntryData>)>,
    },

    Choice {
        default: String,
        variants: Vec<(String, Option<EntryData>)>,
    },

    #[serde(rename_all = "camelCase")]
    Optional {
        default_set: bool,
        content: Box<SchemaNode>,
    },

    #[serde(rename_all = "camelCase")]
    Switch {
        default_enabled: bool,
        content_advanced: bool,
        content: Box<SchemaNode>,
    },

    Boolean {
        default: bool,
    },

    Integer {
        default: i128,
        min: i128,
        max: i128,
        step: i128,
        gui: Option<NumericGuiType>,
    },

    Float {
        default: f64,
        min: Option<f64>,
        max: Option<f64>,
        step: Option<f64>,
        gui: Option<NumericGuiType>,
    },

    Text {
        default: String,
    },

    Array(Vec<SchemaNode>),

    #[serde(rename_all = "camelCase")]
    Vector {
        default_element: Box<SchemaNode>,
        default: serde_json::Value,
    },

    #[serde(rename_all = "camelCase")]
    Dictionary {
        default_key: String,
        default_value: Box<SchemaNode>,
        default: serde_json::Value,
    },
}
