mod higher_order;
mod ty;

use darling::{ast::Fields, util::Flag, FromDeriveInput, FromField, FromVariant};
use heck::SnakeCase;
use higher_order as ho;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use std::string::ToString;
use syn::{DeriveInput, Error, Ident, Lit, Type, Visibility};

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

fn snake_case_ident(ty_ident: &Ident) -> Ident {
    Ident::new(&ty_ident.to_string().to_snake_case(), ty_ident.span())
}

#[derive(FromField)]
#[darling(attributes(schema))]
struct FieldMeta {
    vis: Visibility,

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

#[derive(FromVariant)]
#[darling(attributes(schema), supports(unit, newtype, named))]
struct VariantMeta {
    ident: Ident,
    fields: darling::ast::Fields<FieldMeta>,
}

// attributes(schema) is specified to deny any #[schema()] attributes at this level
#[derive(FromDeriveInput)]
#[darling(attributes(schema), supports(struct_named, enum_any))]
struct DeriveInputMeta {
    data: darling::ast::Data<VariantMeta, FieldMeta>,
}

struct SchemaData {
    default_fields_ts: TokenStream2,
    schema_code_ts: TokenStream2,
    aux_objects_ts: Option<TokenStream2>,
}

fn named_fields_schema(meta: Vec<FieldMeta>) -> TResult<SchemaData> {
    let mut vis = vec![];
    let mut idents = vec![];
    let mut tys_ts = vec![];
    let mut keys = vec![];
    let mut entry_types_ts = vec![];

    for meta in meta {
        for ph in &meta.placeholders {
            keys.push(ph.clone());
            entry_types_ts.push(quote!(settings_schema::EntryType::Placeholder))
        }

        for setting in &meta.higher_order {
            let ho::Entry { key, entry_type_ts } = ho::schema(setting)?;

            keys.push(key);
            entry_types_ts.push(entry_type_ts);
        }

        let ident = meta.ident.as_ref().unwrap().clone();
        let advanced = meta.advanced.is_some();
        let ty::SchemaData {
            default_ty_ts,
            schema_code_ts,
        } = ty::schema(&meta.ty, &meta)?;

        vis.push(meta.vis);
        idents.push(ident.clone());
        tys_ts.push(default_ty_ts);
        keys.push(ident.to_string());
        entry_types_ts.push(quote!(
            settings_schema::EntryType::Data(settings_schema::EntryData {
                advanced: #advanced,
                content: {
                    let default = default.#ident;
                    #schema_code_ts
                }
            })
        ));
    }

    Ok(SchemaData {
        default_fields_ts: quote!(#(#vis #idents: #tys_ts,)*),
        schema_code_ts: quote!(settings_schema::SchemaNode::Section(
            vec![#((#keys.into(), #entry_types_ts)),*]
        )),
        aux_objects_ts: None,
    })
}

fn variants_schema(vis: &Visibility, ident: &Ident, meta: Vec<VariantMeta>) -> TResult<SchemaData> {
    let mut variants = vec![];
    let mut data_variants = vec![];
    let mut data_tys_ts = vec![];
    let mut keys = vec![];
    let mut entry_data_ts = vec![];
    let mut aux_objects_ts = vec![];

    for meta in meta {
        let variant_ident = meta.ident;
        let snake_case_variant_ident = snake_case_ident(&variant_ident);

        variants.push(variant_ident.clone());
        keys.push(variant_ident.to_string().to_snake_case());

        match meta.fields.style {
            darling::ast::Style::Tuple => {
                // darling macro attribute makes sure there is one and only one field
                let field_meta = &meta.fields.fields[0];

                if !field_meta.higher_order.is_empty() {
                    error(
                        "'higher_order' attributes not supported in this position",
                        &variant_ident,
                    )?;
                }

                if !field_meta.placeholders.is_empty() {
                    error(
                        "'placeholder' attributes not supported in this position",
                        &variant_ident,
                    )?;
                }

                let advanced = field_meta.advanced.is_some();
                let ty::SchemaData {
                    default_ty_ts,
                    schema_code_ts,
                } = ty::schema(&field_meta.ty, &field_meta)?;

                data_variants.push(snake_case_variant_ident.clone());
                data_tys_ts.push(default_ty_ts);
                entry_data_ts.push(quote!(Some(settings_schema::EntryData {
                    advanced: #advanced,
                    content: {
                        let default = default.#snake_case_variant_ident;
                        #schema_code_ts
                    }
                })));
            }
            darling::ast::Style::Struct => {
                let default_ty_ts =
                    suffix_ident(&suffix_ident(ident, &variant_ident.to_string()), "Default")
                        .to_token_stream();
                let SchemaData {
                    default_fields_ts,
                    schema_code_ts,
                    ..
                } = named_fields_schema(meta.fields.fields)?;

                data_variants.push(snake_case_variant_ident.clone());
                data_tys_ts.push(default_ty_ts.clone());
                entry_data_ts.push(quote!(Some(settings_schema::EntryData {
                    advanced: false,
                    content: {
                        let default = default.#snake_case_variant_ident;
                        #schema_code_ts
                    }
                })));
                aux_objects_ts.push(quote! {
                    #[derive(settings_schema::Serialize, settings_schema::Deserialize, Clone)]
                    #vis struct #default_ty_ts {
                        #default_fields_ts
                    }
                });
            }
            darling::ast::Style::Unit => {
                entry_data_ts.push(quote!(None));
            }
        }
    }

    let default_variant_ty = suffix_ident(&ident, "DefaultVariant");

    Ok(SchemaData {
        default_fields_ts: quote! {
            #(#vis #data_variants: #data_tys_ts,)*
            variant: #default_variant_ty,
        },
        schema_code_ts: quote!(settings_schema::SchemaNode::Choice {
            default: settings_schema::to_json_value(default.variant)
                .unwrap()
                .as_str()
                .unwrap()
                .into(),
            variants: vec![#((#keys.into(), #entry_data_ts)),*]
        }),
        aux_objects_ts: Some(quote! {
            #(#aux_objects_ts)*

            #[derive(settings_schema::Serialize, settings_schema::Deserialize, Clone)]
            #[serde(rename_all = "snake_case")]
            #vis enum #default_variant_ty {
                #(#variants,)*
            }
        }),
    })
}

fn schema(derive_input: DeriveInput) -> TResult {
    if !derive_input.generics.params.is_empty() {
        return error("Generics not supported", &derive_input.generics);
    }

    let meta: DeriveInputMeta =
        FromDeriveInput::from_derive_input(&derive_input).map_err(|e| e.write_errors())?;

    let vis = derive_input.vis;
    let derive_input_ident = derive_input.ident;
    let default_ty_ident = suffix_ident(&derive_input_ident, "Default");

    let SchemaData {
        default_fields_ts,
        schema_code_ts,
        aux_objects_ts,
    } = match meta.data {
        darling::ast::Data::Enum(variants) => variants_schema(&vis, &derive_input_ident, variants)?,
        darling::ast::Data::Struct(Fields { fields, .. }) => named_fields_schema(fields)?,
    };

    Ok(quote! {
        #aux_objects_ts

        #[allow(non_snake_case)]
        #[derive(serde::Serialize, serde::Deserialize, Clone)]
        #vis struct #default_ty_ident {
            #default_fields_ts
        }

        impl #derive_input_ident {
            #vis fn schema(default: #default_ty_ident) -> settings_schema::SchemaNode {
                #schema_code_ts
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
