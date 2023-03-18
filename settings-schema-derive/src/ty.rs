use crate::{error, suffix_ident, FieldMeta, TResult, TokenStream2};
use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{GenericArgument, Lit, PathArguments, Type, TypeArray, TypePath};

#[derive(FromMeta)]
pub enum NumericGuiType {
    Slider {
        min: Lit,
        max: Lit,
        step: Option<Lit>,

        #[darling(default)]
        logarithmic: bool,
    },
    TextBox,
}

pub struct TypeSchemaData {
    // Schema representation type, assigned to a specific field in the schema representation struct
    pub default_ty_ts: TokenStream2,

    // Schema instatiation code for a specific field
    pub schema_code_ts: TokenStream2,
}

fn get_first_and_only_type_argument(arguments: &PathArguments) -> &Type {
    if let PathArguments::AngleBracketed(args_block) = &arguments {
        if let GenericArgument::Type(ty) = args_block.args.first().unwrap() {
            return ty;
        }
    }
    // Fail cases are already handled by the compiler
    unreachable!()
}

fn forbid_numeric_attrs(field: &FieldMeta, type_str: &str) -> TResult<()> {
    let tokens = if let Some(arg) = &field.suffix {
        arg.to_token_stream()
    } else if field.gui.is_some() {
        quote!()
    } else {
        return Ok(());
    };

    error(
        &format!("Unexpected argument for {} type", type_str),
        tokens,
    )
}

fn bool_type_schema(field: &FieldMeta) -> TResult {
    forbid_numeric_attrs(field, "bool")?;

    Ok(quote!(settings_schema::SchemaNode::Boolean { default }))
}

enum NumberType {
    UnsignedInteger,
    SignedInteger,
    Float,
}

fn number_type_schema(field: &FieldMeta, ty_ident: &Ident, ty: NumberType) -> TResult {
    let gui_ts = match &field.gui {
        Some(NumericGuiType::Slider {
            min,
            max,
            step,
            logarithmic,
        }) => {
            let step_ts = if let Some(step) = step {
                quote!({
                    let step: #ty_ident = #step;
                    Some(step as f64)
                })
            } else {
                quote!(None)
            };

            quote!({
                let min: #ty_ident = #min;
                let max: #ty_ident = #max;
                debug_assert!(min <= max);

                settings_schema::NumericGuiType::Slider {
                    range: min as f64..=max as f64,
                    step: #step_ts,
                    logarithmic: #logarithmic
                }
            })
        }
        _ => quote!(settings_schema::NumericGuiType::TextBox),
    };

    let suffix_ts = if let Some(suffix) = &field.suffix {
        quote!(Some(#suffix.into()))
    } else {
        quote!(None)
    };

    let num_ty_string = match ty {
        NumberType::UnsignedInteger => "UnsignedInteger",
        NumberType::SignedInteger => "SignedInteger",
        NumberType::Float => "Float",
    };
    let num_ty_ident = Ident::new(num_ty_string, Span::call_site());

    Ok(quote! {
        settings_schema::SchemaNode::Number {
            default: default as _,
            ty: settings_schema::NumberType::#num_ty_ident,
            gui: #gui_ts,
            suffix: #suffix_ts
        }
    })
}

fn string_type_schema(field: &FieldMeta) -> TResult {
    forbid_numeric_attrs(field, "String")?;

    Ok(quote!(settings_schema::SchemaNode::Text { default }))
}

fn custom_leaf_type_schema(ty_ident: &Ident, field: &FieldMeta) -> TResult {
    forbid_numeric_attrs(field, "custom")?;

    Ok(quote!(#ty_ident::schema(default)))
}

// Generate a default representation type and corresponding schema instantiation code.
// This function calls itself recursively to parse the whole compound type. The recursion degree is
// 1: only types that have only one type argument can be parsed. Still custom types cannot have type
// arguments, so they are always the leaf type.
// The meta parameter contains the attributes associated to the curent field: they are forwarded
// as-is in every recursion step. Most of the attributes are used for numerical leaf types, but
// there is also the `switch_default` flag that is used by each Switch type inside the type chain.
pub(crate) fn schema(ty: &Type, meta: &FieldMeta) -> Result<TypeSchemaData, TokenStream> {
    match &ty {
        Type::Array(TypeArray { len, elem, .. }) => {
            let TypeSchemaData {
                default_ty_ts,
                schema_code_ts,
            } = schema(elem, meta)?;
            Ok(TypeSchemaData {
                default_ty_ts: quote!([#default_ty_ts; #len]),
                schema_code_ts: quote! {{
                    let length = #len;
                    // Note: for arrays, into_iter() behaves like iter(), because of a
                    // implementation complication in the std library. Blocked by const generics.
                    // For now clone() is necessary.
                    let content = default.iter().map(|default| {
                        let default = default.clone();
                        #schema_code_ts
                    }).collect::<Vec<_>>();

                    settings_schema::SchemaNode::Array(content)
                }},
            })
        }
        Type::Path(TypePath { path, .. }) => {
            let ty_last = path.segments.last().unwrap();
            let ty_ident = &ty_last.ident;
            if matches!(ty_last.arguments, PathArguments::None) {
                let mut default_ty_ts = None;
                let schema_code_ts = match ty_ident.to_string().as_str() {
                    "bool" => bool_type_schema(meta)?,
                    "u8" | "u16" | "u32" | "u64" | "usize" => {
                        number_type_schema(meta, ty_ident, NumberType::UnsignedInteger)?
                    }
                    "i8" | "i16" | "i32" | "i64" | "isize" => {
                        number_type_schema(meta, ty_ident, NumberType::SignedInteger)?
                    }
                    "f32" | "f64" => number_type_schema(meta, ty_ident, NumberType::Float)?,
                    "String" => string_type_schema(meta)?,
                    "u128" | "i128" => error("Unsupported integer size", ty_ident)?,
                    _ => {
                        default_ty_ts = Some(suffix_ident(ty_ident, "Default").to_token_stream());
                        custom_leaf_type_schema(ty_ident, meta)?
                    }
                };
                Ok(TypeSchemaData {
                    default_ty_ts: default_ty_ts.unwrap_or_else(|| ty_ident.to_token_stream()),
                    schema_code_ts,
                })
            } else if ty_ident == "Option" {
                let TypeSchemaData {
                    default_ty_ts,
                    schema_code_ts,
                } = schema(get_first_and_only_type_argument(&ty_last.arguments), meta)?;
                Ok(TypeSchemaData {
                    default_ty_ts: quote!(settings_schema::OptionalDefault<#default_ty_ts>),
                    schema_code_ts: quote! {{
                        let default_set = default.set;
                        let default = default.content;
                        let content = Box::new(#schema_code_ts);
                        settings_schema::SchemaNode::Optional { default_set, content }
                    }},
                })
            } else if ty_ident == "Switch" {
                let TypeSchemaData {
                    default_ty_ts,
                    schema_code_ts,
                } = schema(get_first_and_only_type_argument(&ty_last.arguments), meta)?;
                Ok(TypeSchemaData {
                    default_ty_ts: quote!(settings_schema::SwitchDefault<#default_ty_ts>),
                    schema_code_ts: quote! {{
                        let default_enabled = default.enabled;
                        let default = default.content;
                        let content = Box::new(#schema_code_ts);
                        settings_schema::SchemaNode::Switch {
                            default_enabled,
                            content
                        }
                    }},
                })
            } else if ty_ident == "Vec" {
                let ty_arg = get_first_and_only_type_argument(&ty_last.arguments);
                if let Type::Tuple(ty_tuple) = ty_arg {
                    if ty_tuple.elems.len() != 2 {
                        error("Expected two arguments", &ty_tuple.elems)
                    } else if ty_tuple.elems[0].to_token_stream().to_string() != "String" {
                        error("First argument must be a `String`", &ty_tuple.elems)
                    } else {
                        let ty_arg = &ty_tuple.elems[1];
                        let TypeSchemaData {
                            default_ty_ts,
                            schema_code_ts,
                        } = schema(ty_arg, meta)?;
                        Ok(TypeSchemaData {
                            default_ty_ts: quote! {
                                settings_schema::DictionaryDefault<#default_ty_ts>
                            },
                            schema_code_ts: quote! {{
                                let default_content =
                                    serde_json::to_value(default.content).unwrap();
                                let default_key = default.key;
                                let default = default.value;
                                let default_value = Box::new(#schema_code_ts);
                                settings_schema::SchemaNode::Dictionary {
                                    default_key,
                                    default_value,
                                    default: default_content
                                }
                            }},
                        })
                    }
                } else {
                    let TypeSchemaData {
                        default_ty_ts,
                        schema_code_ts,
                    } = schema(ty_arg, meta)?;
                    Ok(TypeSchemaData {
                        default_ty_ts: quote!(settings_schema::VectorDefault<#default_ty_ts>),
                        schema_code_ts: quote! {{
                            let default_content =
                                serde_json::to_value(default.content).unwrap();
                            let default = default.element;
                            let default_element = Box::new(#schema_code_ts);
                            settings_schema::SchemaNode::Vector {
                                default_element,
                                default: default_content
                            }
                        }},
                    })
                }
            } else {
                error(
                    "Type arguments are supported only for Option, Switch, Vec",
                    ty_last,
                )
            }
        }
        _ => error("Unsupported type", ty),
    }
}
