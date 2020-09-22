use darling::{util::Flag, FromDeriveInput, FromField, FromMeta, FromVariant};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use std::string::ToString;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Error, Fields, FieldsNamed, GenericArgument,
    Generics, Ident, Lit, Meta, NestedMeta, Path, PathArguments, Type, Visibility,
};

type TResult<T = ()> = Result<T, TokenStream>;

fn error<T, TT: ToTokens>(message: &str, tokens: TT) -> TResult<T> {
    Err(
        Error::new_spanned(tokens, format!("[SettingsSchema] {}", message))
            .to_compile_error()
            .into(),
    )
}

fn to_tt<T>(res: darling::Result<T>) -> TResult<T> {
    res.map_err(|e| e.write_errors().into())
}

#[derive(FromMeta, Debug)]
enum NumericType {
    Float(f32),
    Int(i32),
}

#[derive(FromVariant)]
#[darling(attributes(schema), supports(unit, newtype, named))]
struct SchemaVariant {
    ident: Ident,
    fields: darling::ast::Fields<SchemaField>,
}

#[derive(FromMeta)]
enum NumericControlType {
    TextBox,
    UpDown,
    Slider,
}

#[derive(FromMeta)]
enum ChoiceControlType {
    Dropdown,
    ButtonGroup,
}

#[derive(FromMeta)]
enum HigherOrderDataDesc {
    Choice {
        default: String,

        #[darling(multiple)]
        variant: Vec<String>,

        gui: Option<ChoiceControlType>,
    },
    Bool {
        default: bool,
    },
    Action,
}

#[derive(FromMeta)]
enum UpdateType {
    Assign,
    Remove,
}

#[derive(FromMeta)]
enum ModifierVariable {
    Input,
    Path(Path)
}

#[derive(FromMeta)]
struct ModifierDesc {
    target: Path,
    update_op: UpdateType,
    expr: String,
    
    #[darling(multiple)]
    vars: Vec<ModifierVariable>
}

#[derive(FromMeta)]
struct HigherOrderSetting {
    name: String,

    data: HigherOrderDataDesc,

    #[darling(multiple)]
    modifier: Vec<ModifierDesc>,
}

#[derive(FromField)]
#[darling(attributes(schema))]
struct SchemaField {
    ident: Option<Ident>,

    ty: Type,

    #[darling(multiple)]
    placeholder: Vec<String>,

    #[darling(multiple)]
    higher_order: Vec<HigherOrderSetting>,

    #[darling(default)]
    advanced: Flag,

    #[darling(default)]
    min: Option<Lit>,

    #[darling(default)]
    max: Option<Lit>,

    #[darling(default)]
    step: Option<Lit>,

    #[darling(default)]
    gui: Option<Meta>,
}

// attributes(schema) is specified to deny any schema() attributes at this level
#[derive(FromDeriveInput)]
#[darling(attributes(schema), supports(struct_named, enum_any))]
struct SchemaDeriveInput {
    vis: Visibility,
    ident: Ident,
    generics: Generics,
    data: darling::ast::Data<SchemaVariant, SchemaField>,
}

fn schema(derive_input: DeriveInput) -> TResult<TokenStream2> {
    let schema_derive_input: SchemaDeriveInput =
        to_tt(FromDeriveInput::from_derive_input(&derive_input))?;

    if !schema_derive_input.generics.params.is_empty() {
        return error("Generics not supported", &derive_input.generics);
    }

    match schema_derive_input.data {
        darling::ast::Data::Enum(_) => (),
        darling::ast::Data::Struct(fields) => for field in fields {},
    }

    Ok(quote! {})
}

#[proc_macro_derive(SettingsSchema, attributes(schema))]
pub fn create_settings_schema_fn_and_default_ty(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match schema(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e,
    }
}
