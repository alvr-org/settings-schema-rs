mod higher_order;
mod ty;

use darling::{ast::Fields, util::Flag, FromDeriveInput, FromField, FromVariant};
use higher_order as ho;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use std::string::ToString;
use syn::{DeriveInput, Error, Ident, Lit, Type};

type TResult<T = TokenStream2> = Result<T, TokenStream>;

fn error<T, TT: ToTokens>(message: &str, tokens: TT) -> TResult<T> {
    Err(
        Error::new_spanned(tokens, format!("[SettingsSchema] {}", message))
            .to_compile_error()
            .into(),
    )
}

fn suffix_ident(ty_ident: &Ident, suffix: &str) -> Ident {
    Ident::new(
        &format!("{}{}", ty_ident.to_string(), suffix),
        ty_ident.span(),
    )
}

#[derive(FromVariant)]
#[darling(attributes(schema), supports(unit, newtype, named))]
struct SchemaVariant {
    ident: Ident,
    fields: darling::ast::Fields<SchemaField>,
}

#[derive(FromField)]
#[darling(attributes(schema))]
struct SchemaField {
    ident: Option<Ident>,

    ty: Type,

    #[darling(multiple)]
    #[darling(rename = "placeholder")]
    placeholders: Vec<String>,

    #[darling(multiple)]
    higher_order: Vec<ho::HigherOrderSetting>,

    #[darling(default)]
    advanced: Flag,

    #[darling(default)]
    switch_advanced: Flag,

    #[darling(default)]
    min: Option<Lit>,

    #[darling(default)]
    max: Option<Lit>,

    #[darling(default)]
    step: Option<Lit>,

    #[darling(default)]
    gui: Option<ty::NumericGuiType>,
}

// attributes(schema) is specified to deny any #[schema()] attributes at this level
#[derive(FromDeriveInput)]
#[darling(attributes(schema), supports(struct_named, enum_any))]
struct SchemaDeriveInput {
    data: darling::ast::Data<SchemaVariant, SchemaField>,
}

struct NamedFieldsData {
    idents: Vec<Ident>,
    tys_ts: Vec<TokenStream2>,
    schema_code_ts: TokenStream2,
}

fn named_fields_schema(fields: Vec<SchemaField>) -> TResult<NamedFieldsData> {
    let mut idents = vec![];
    let mut tys_ts = vec![];
    let mut schema_pairs_ts = vec![];
    for field in fields {
        for ph in &field.placeholders {
            schema_pairs_ts.push(quote!((#ph.into(), settings_schema::EntryType::Placeholder)))
        }

        for setting in &field.higher_order {
            schema_pairs_ts.push(ho::schema(setting)?);
        }

        let advanced = field.advanced.is_some();
        let ty::Schema {
            default_ty_ts,
            schema_code_ts,
        } = ty::schema(&field.ty, &field)?;

        let ident = field.ident.unwrap();
        let key = ident.to_string();
        schema_pairs_ts.push(quote! {
            (
                #key.into(),
                settings_schema::EntryType::Data(settings_schema::EntryData {
                    advanced: #advanced,
                    content: {
                        let default = default.#ident;
                        #schema_code_ts
                    }
                })
            )
        });

        idents.push(ident);
        tys_ts.push(default_ty_ts);
    }

    let schema_code_ts = quote! {{
        let mut entries = vec![];
        #(entries.push(#schema_pairs_ts);)*
        settings_schema::SchemaNode::Section(entries)
    }};

    Ok(NamedFieldsData {
        idents,
        tys_ts,
        schema_code_ts,
    })
}

struct SchemaVariants {
    variants: (Ident, Option<TokenStream2>),
    schema_code_ts: TokenStream2,
}

fn schema_variants(variants: Vec<SchemaVariant>) {}

fn schema(derive_input: DeriveInput) -> TResult {
    let schema_derive_input: SchemaDeriveInput =
        FromDeriveInput::from_derive_input(&derive_input).map_err(|e| e.write_errors())?;

    if !derive_input.generics.params.is_empty() {
        return error("Generics not supported", &derive_input.generics);
    }

    let schema_fn_content_ts;
    let field_idents;
    let field_tys_ts;
    match schema_derive_input.data {
        darling::ast::Data::Enum(variants) => {
            schema_variants(variants);
            todo!();
        }
        darling::ast::Data::Struct(Fields { fields, .. }) => {
            let fields_data = named_fields_schema(fields)?;
            schema_fn_content_ts = fields_data.schema_code_ts;
            field_idents = fields_data.idents;
            field_tys_ts = fields_data.tys_ts;
            // panic!("{:?}", named_fields_data.schema_code_ts.to_string())
        }
    }

    let vis = derive_input.vis;
    let derive_input_ident = derive_input.ident;
    let default_ty_ident = suffix_ident(&derive_input_ident, "Default");

    Ok(quote! {
        #[allow(non_snake_case)]
        #[derive(serde::Serialize, serde::Deserialize, Clone)]
        #vis struct #default_ty_ident {
            #(#vis #field_idents: #field_tys_ts,)*
        }

        impl #derive_input_ident {
            #vis fn schema(default: #default_ty_ident) -> settings_schema::SchemaNode {
                #schema_fn_content_ts
            }
        }
    })
}

#[proc_macro_derive(SettingsSchema, attributes(schema))]
pub fn create_settings_schema_fn_and_default_ty(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    match schema(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e,
    }
}
