use crate::{error, suffix_ident, SchemaField, TResult, TokenStream2};
use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{quote, ToTokens};
use syn::{GenericArgument, Lit, PathArguments, Type, TypeArray, TypePath};

#[derive(FromMeta)]
pub enum NumericGuiType {
    TextBox,
    UpDown,
    Slider,
}

pub struct Schema {
    pub default_ty_ts: TokenStream2,
    pub schema_code_ts: TokenStream2,
}

fn get_only_type_argument(arguments: &PathArguments) -> &Type {
    if let PathArguments::AngleBracketed(args_block) = &arguments {
        if let GenericArgument::Type(ty) = args_block.args.first().unwrap() {
            return ty;
        }
    }
    // Fail cases are already handled by the compiler
    unreachable!()
}

fn forbid_numeric_attrs(field: &SchemaField, type_str: &str) -> TResult<()> {
    let maybe_invalid_arg = field
        .min
        .as_ref()
        .or_else(|| field.max.as_ref())
        .or_else(|| field.step.as_ref());
    // .map_or_else(|| field.gui.as_ref());

    let tokens = if let Some(arg) = maybe_invalid_arg {
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

fn bool_type_schema(field: &SchemaField) -> TResult {
    forbid_numeric_attrs(field, "bool")?;

    Ok(quote!(settings_schema::SchemaNode::Boolean { default }))
}

fn maybe_integer_literal(literal: Option<&Lit>) -> TResult {
    if let Some(literal) = literal {
        if let Lit::Int(lit_int) = literal {
            Ok(quote!(Some(#lit_int)))
        } else {
            error("Expected integer literal", literal)
        }
    } else {
        Ok(quote!(None))
    }
}

fn maybe_float_literal(literal: Option<&Lit>) -> TResult {
    if let Some(literal) = literal {
        if let Lit::Float(lit_float) = literal {
            Ok(quote!(Some(#lit_float as _)))
        } else {
            error("Expected float literal", literal)
        }
    } else {
        Ok(quote!(None))
    }
}

fn maybe_numeric_gui(gui: Option<&NumericGuiType>) -> TResult {
    if let Some(gui) = gui {
        match gui {
            NumericGuiType::TextBox => Ok(quote!(Some(settings_schema::NumericGuiType::TextBox))),
            NumericGuiType::UpDown => Ok(quote!(Some(settings_schema::NumericGuiType::UpDown))),
            NumericGuiType::Slider => Ok(quote!(Some(settings_schema::NumericGuiType::Slider))),
        }
    } else {
        Ok(quote!(None))
    }
}

fn integer_type_schema(field: &SchemaField) -> TResult {
    let min_ts = maybe_integer_literal(field.min.as_ref())?;
    let max_ts = maybe_integer_literal(field.max.as_ref())?;
    let step_ts = maybe_integer_literal(field.step.as_ref())?;
    let gui_ts = maybe_numeric_gui(field.gui.as_ref())?;

    Ok(quote! {
        settings_schema::SchemaNode::Integer {
            default: default as _,
            min: #min_ts,
            max: #max_ts,
            step: #step_ts,
            gui: #gui_ts,
        }
    })
}

fn float_type_schema(field: &SchemaField) -> TResult {
    let min_ts = maybe_float_literal(field.min.as_ref())?;
    let max_ts = maybe_float_literal(field.max.as_ref())?;
    let step_ts = maybe_float_literal(field.step.as_ref())?;
    let gui_ts = maybe_numeric_gui(field.gui.as_ref())?;

    Ok(quote! {
        settings_schema::SchemaNode::Float {
            default: default as _,
            min: #min_ts,
            max: #max_ts,
            step: #step_ts,
            gui: #gui_ts,
        }
    })
}

fn string_type_schema(field: &SchemaField) -> TResult {
    forbid_numeric_attrs(field, "String")?;

    Ok(quote!(settings_schema::SchemaNode::Text { default }))
}

fn custom_leaf_type_schema(ty_ident: &Ident, field: &SchemaField) -> TResult {
    forbid_numeric_attrs(field, "custom")?;

    Ok(quote!(#ty_ident::schema(default)))
}

pub(crate) fn schema(ty: &Type, field: &SchemaField) -> Result<Schema, TokenStream> {
    match &ty {
        Type::Array(TypeArray { len, .. }) => {
            let Schema {
                default_ty_ts,
                schema_code_ts,
            } = schema(ty, field)?;
            Ok(Schema {
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
                let mut custom_default_ty_ts = None;
                let schema_code_ts = match ty_ident.to_string().as_str() {
                    "bool" => bool_type_schema(field)?,
                    "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" => {
                        integer_type_schema(field)?
                    }
                    "f32" | "f64" => float_type_schema(field)?,
                    "String" => string_type_schema(field)?,
                    _ => {
                        custom_default_ty_ts =
                            Some(suffix_ident(&ty_ident, "Default").to_token_stream());
                        custom_leaf_type_schema(ty_ident, field)?
                    }
                };
                Ok(Schema {
                    default_ty_ts: if let Some(tokens) = custom_default_ty_ts {
                        tokens
                    } else {
                        ty_ident.to_token_stream()
                    },
                    schema_code_ts,
                })
            } else if ty_ident == "Option" {
                let Schema {
                    default_ty_ts,
                    schema_code_ts,
                } = schema(get_only_type_argument(&ty_last.arguments), field)?;
                Ok(Schema {
                    default_ty_ts: quote!(settings_schema::OptionalDefault<#default_ty_ts>),
                    schema_code_ts: quote! {{
                        let default_set = default.set;
                        let default = default.content;
                        let content = Box::new(#schema_code_ts);
                        settings_schema::SchemaNode::Optional { default_set, content }
                    }},
                })
            } else if ty_ident == "Switch" {
                let content_advanced = field.switch_advanced.is_some();
                let Schema {
                    default_ty_ts,
                    schema_code_ts,
                } = schema(get_only_type_argument(&ty_last.arguments), field)?;
                Ok(Schema {
                    default_ty_ts: quote!(settings_schema::SwitchDefault<#default_ty_ts>),
                    schema_code_ts: quote! {{
                        let default_enabled = default.enabled;
                        let default = default.content;
                        let content = Box::new(#schema_code_ts);
                        settings_schema::SchemaNode::Switch {
                            default_enabled,
                            content_advanced: #content_advanced,
                            content
                        }
                    }},
                })
            } else if ty_ident == "Vec" {
                let ty_arg = get_only_type_argument(&ty_last.arguments);
                if let Type::Tuple(ty_tuple) = ty_arg {
                    if ty_tuple.elems.len() != 2 {
                        error("Expected two arguments", &ty_tuple.elems)
                    } else if ty_tuple.elems[0].to_token_stream().to_string() != "String" {
                        error("First argument must be a `String`", &ty_tuple.elems)
                    } else {
                        let ty_arg = &ty_tuple.elems[1];
                        let Schema {
                            default_ty_ts,
                            schema_code_ts,
                        } = schema(ty_arg, field)?;
                        Ok(Schema {
                            default_ty_ts: quote! {
                                settings_schema::DictionaryDefault<#default_ty_ts, #ty_arg>
                            },
                            schema_code_ts: quote! {{
                                let default_content =
                                    serde_json::to_value(default.default).unwrap();
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
                    let Schema {
                        default_ty_ts,
                        schema_code_ts,
                    } = schema(ty_arg, field)?;
                    Ok(Schema {
                        default_ty_ts: quote!(settings_schema::VectorDefault<#default_ty_ts, #ty_arg>),
                        schema_code_ts: quote! {{
                            let default_content =
                                serde_json::to_value(default.default).unwrap();
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
                    "Generics are supported only for Option, Switch, Vec",
                    &ty_last,
                )
            }
        }
        _ => error("Unsupported type", &ty),
    }
}
