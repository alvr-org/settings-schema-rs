mod ty;

use darling::{ast, FromDeriveInput, FromField, FromMeta, FromVariant};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use std::string::ToString;
use syn::{DeriveInput, Error, Ident, Lit, Type, Visibility};
use ty::{NumericGuiType, TypeSchemaData};

type TResult<T = TokenStream2> = Result<T, TokenStream>;

fn error<T, TT: ToTokens>(message: &str, tokens: TT) -> TResult<T> {
    Err(
        Error::new_spanned(tokens, format!("[SettingsSchema] {}", message))
            .to_compile_error()
            .into(),
    )
}

fn suffix_ident(ty_ident: &Ident, suffix: &str) -> Ident {
    Ident::new(&format!("{}{}", ty_ident, suffix), ty_ident.span())
}

#[derive(Default)]
struct StringMap(Vec<(String, String)>);

impl FromMeta for StringMap {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        if let syn::Meta::List(value) = item {
            let mut strings = vec![];
            for item in &value.nested {
                if let syn::NestedMeta::Meta(syn::Meta::NameValue(key_value)) = item {
                    let key_ident = key_value.path.get_ident().ok_or_else(|| {
                        darling::Error::custom("Key must be an identifier")
                            .with_span(&key_value.path)
                    })?;

                    let value = if let Lit::Str(string) = &key_value.lit {
                        string.value()
                    } else {
                        return Err(darling::Error::custom("Value must be a string")
                            .with_span(&key_value.lit));
                    };

                    strings.push((key_ident.to_string(), value));
                } else {
                    return Err(
                        darling::Error::custom("Unexpected syntax. Use `key = \"value\"`")
                            .with_span(item),
                    );
                }
            }

            Ok(StringMap(strings))
        } else {
            Err(
                darling::Error::custom("Invalid format for \"strings\". Use `strings(...)`")
                    .with_span(item),
            )
        }
    }
}

#[derive(FromField)]
#[darling(attributes(schema))]
struct FieldMeta {
    vis: Visibility,

    ident: Option<Ident>,

    ty: Type,

    #[darling(default)]
    strings: StringMap,

    #[darling(default)]
    min: Option<Lit>,

    #[darling(default)]
    max: Option<Lit>,

    #[darling(default)]
    step: Option<Lit>,

    #[darling(default)]
    gui: Option<NumericGuiType>,
}

#[derive(FromMeta)]
enum ChoiceControlType {
    Dropdown,
    ButtonGroup,
}

#[derive(FromVariant)]
#[darling(attributes(schema), supports(unit, newtype, named))]
struct VariantMeta {
    ident: Ident,

    #[darling(default)]
    strings: StringMap,

    fields: ast::Fields<FieldMeta>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(schema), supports(struct_named, enum_any))]
struct DeriveInputMeta {
    data: ast::Data<VariantMeta, FieldMeta>,

    #[darling(default)]
    gui: Option<ChoiceControlType>,
}

struct SchemaData {
    // Fields for the schema representation struct. In case of struct, the fields have the same name
    // of the original fields. Incase of enum, adds the field `variant` plus the name of the
    // variants of the original enum
    default_fields_ts: TokenStream2,

    // Schema instatiation code, to be inserted into the schema() method
    schema_code_ts: TokenStream2,

    // Auxiliary objects for enums (default variant and default variants data)
    aux_objects_ts: Option<TokenStream2>,
}

fn named_fields_schema(
    meta: Vec<FieldMeta>,
    vis_override: Option<Visibility>,
) -> TResult<SchemaData> {
    let mut default_entries_ts = vec![];
    let mut schema_entries_ts = vec![];

    for meta in meta {
        let vis = if let Some(vis) = &vis_override {
            vis
        } else {
            &meta.vis
        };
        let field_ident = meta.ident.as_ref().unwrap().clone();
        let TypeSchemaData {
            default_ty_ts,
            schema_code_ts,
        } = ty::schema(&meta.ty, &meta)?;
        let field_string = field_ident.to_string();

        let string_key_values_ts = meta
            .strings
            .0
            .into_iter()
            .map(|(key, value)| quote!((#key.into(), #value.into())));

        default_entries_ts.push(quote!(#vis #field_ident: #default_ty_ts));
        schema_entries_ts.push(quote!(settings_schema::NamedEntry {
            name: #field_string.into(),
            strings: [#(#string_key_values_ts),*].into(),
            content: {
                let default = default.#field_ident;
                #schema_code_ts
            }
        }));
    }

    Ok(SchemaData {
        default_fields_ts: quote!(#(#default_entries_ts,)*),
        schema_code_ts: quote!(settings_schema::SchemaNode::Section(
            vec![#(#schema_entries_ts),*]
        )),
        aux_objects_ts: None,
    })
}

fn variants_schema(
    gui_type: Option<ChoiceControlType>,
    vis: &Visibility,
    ident: &Ident,
    meta: Vec<VariantMeta>,
) -> TResult<SchemaData> {
    let mut default_variants_ts = vec![];
    let mut variant_entries_ts = vec![];
    let mut variants = vec![];
    let mut aux_variants_structs_ts = vec![];

    let gui_ts = match gui_type {
        None => quote!(None),
        Some(ChoiceControlType::Dropdown) => {
            quote!(Some(settings_schema::ChoiceControlType::Dropdown))
        }
        Some(ChoiceControlType::ButtonGroup) => {
            quote!(Some(settings_schema::ChoiceControlType::ButtonGroup))
        }
    };

    for meta in meta {
        let variant_ident = meta.ident;
        let variant_string = variant_ident.to_string();

        variants.push(variant_ident.clone());

        let entry_content_ts = match meta.fields.style {
            ast::Style::Tuple => {
                // darling macro attribute makes sure there is one and only one field
                let field_meta = &meta.fields.fields[0];
                let TypeSchemaData {
                    default_ty_ts,
                    schema_code_ts,
                } = ty::schema(&field_meta.ty, field_meta)?;

                if !field_meta.strings.0.is_empty() {
                    return error(
                        "Can't use `strings` list in variant tuple field.",
                        field_meta.ty.to_token_stream(),
                    );
                }

                default_variants_ts.push(quote!(#vis #variant_ident: #default_ty_ts));

                quote!(Some({
                    let default = default.#variant_ident;
                    #schema_code_ts
                }))
            }
            ast::Style::Struct => {
                let default_ty_ts =
                    suffix_ident(&suffix_ident(ident, &variant_ident.to_string()), "Default")
                        .to_token_stream();
                let SchemaData {
                    default_fields_ts,
                    schema_code_ts,
                    ..
                } = named_fields_schema(meta.fields.fields, Some(vis.clone()))?;

                default_variants_ts.push(quote!(#vis #variant_ident: #default_ty_ts));
                aux_variants_structs_ts.push(quote! {
                    #[derive(settings_schema::Serialize, settings_schema::Deserialize, Clone, Debug)]
                    #vis struct #default_ty_ts {
                        #default_fields_ts
                    }
                });

                quote!(Some({
                    let default = default.#variant_ident;
                    #schema_code_ts
                }))
            }
            ast::Style::Unit => quote!(None),
        };

        let string_key_values_ts = meta
            .strings
            .0
            .into_iter()
            .map(|(key, value)| quote!((#key.into(), #value.into())));

        variant_entries_ts.push(quote!(settings_schema::NamedEntry {
            name: #variant_string.into(),
            strings: [#(#string_key_values_ts),*].into(),
            content: #entry_content_ts,
        }));
    }

    let default_variant_ty = suffix_ident(ident, "DefaultVariant");

    Ok(SchemaData {
        default_fields_ts: quote! {
            #(#default_variants_ts,)*
            #vis variant: #default_variant_ty,
        },
        schema_code_ts: quote!(settings_schema::SchemaNode::Choice {
            default: settings_schema::to_json_value(default.variant)
                .unwrap()
                .as_str()
                .unwrap()
                .into(),
            variants: vec![#(#variant_entries_ts),*],
            gui: #gui_ts
        }),
        aux_objects_ts: Some(quote! {
            #(#aux_variants_structs_ts)*

            #[derive(settings_schema::Serialize, settings_schema::Deserialize, Clone, Debug)]
            #vis enum #default_variant_ty {
                #(#variants,)*
            }
        }),
    })
}

// Generate new code from the given struct or enum.
//
// In case of a struct two things are created:
// * a default settings representation (struct <StructName>Default)
// * a impl with a schema() method, that returns the schema associated to the current struct
// The default representation is a struct that contains each of the original fields, where the types
// are substituted with the matching default representation type.
//
// Like for structs, for enums the default settings representation and a schema method are generated.
// Some auxiliary objects are also generated: the default variant (enum <EnumName>DefaultVariant)
// and default variants stuctures (struct <EnumName><VariantName>Default). The default variant is a
// plain old enum with the same variants as the original enum but no variant data. The default
// variants stuctures contains the default representation of the variants content, both in case of
// newtype and struct style content.
// The default representation struct contains the `variant` field of type default variant; the rest
// of the fields are the name of the original variants, without casing transformations. Only
// variants which contains data are inserted as fields in the default representation struct.
fn schema(derive_input: DeriveInput) -> TResult {
    if !derive_input.generics.params.is_empty() {
        return error("Generics not supported", &derive_input.generics);
    }

    let meta: DeriveInputMeta =
        FromDeriveInput::from_derive_input(&derive_input).map_err(|e| e.write_errors())?;

    let gui_type = meta.gui;
    let vis = derive_input.vis;
    let derive_input_ident = derive_input.ident;
    let default_ty_ident = suffix_ident(&derive_input_ident, "Default");

    let SchemaData {
        default_fields_ts,
        schema_code_ts,
        aux_objects_ts,
    } = match meta.data {
        ast::Data::Enum(variants) => {
            variants_schema(gui_type, &vis, &derive_input_ident, variants)?
        }
        ast::Data::Struct(ast::Fields { fields, .. }) => named_fields_schema(fields, None)?,
    };

    Ok(quote! {
        #aux_objects_ts

        #[allow(non_snake_case)]
        #[derive(settings_schema::Serialize, settings_schema::Deserialize, Clone, Debug)]
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

// This is the entry point of the macro, that is `derive(SettingsSchema)`
#[proc_macro_derive(SettingsSchema, attributes(schema))]
pub fn create_settings_schema_fn_and_default_ty(input: TokenStream) -> TokenStream {
    match schema(syn::parse_macro_input!(input as DeriveInput)) {
        Ok(tokens) => tokens.into(),
        Err(e) => e,
    }
}
