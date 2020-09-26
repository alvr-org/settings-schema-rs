use darling::*;
use proc_macro2::Ident;
use quote::quote;
use syn::{Expr, ExprField, ExprIndex, ExprMethodCall, LitStr, Member};

use crate::{error, TResult, TokenStream2};

#[derive(FromMeta)]
enum ChoiceControlType {
    Dropdown,
    ButtonGroup,
}

#[derive(FromMeta)]
enum HigherOrderType {
    Choice {
        default: String,

        #[darling(multiple)]
        #[darling(rename = "variant")]
        variants: Vec<String>,

        gui: ChoiceControlType,
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
struct ModifierDesc {
    target: String,
    update_op: UpdateType,
    expr: String,

    #[darling(multiple)]
    #[darling(rename = "var")]
    vars: Vec<String>,
}

#[derive(FromMeta)]
pub struct HigherOrderSetting {
    name: String,

    #[darling(rename = "data")]
    data_type: HigherOrderType,

    #[darling(multiple)]
    #[darling(rename = "modifier")]
    modifiers: Vec<ModifierDesc>,
}

fn parse_segments(expr: &Expr) -> TResult<Vec<TokenStream2>> {
    const ERR_MSG: &str = "Invalid expression inside string";

    match expr {
        Expr::Path(first_segment) => {
            let maybe_ident = syn::parse2::<Ident>(first_segment.to_token_stream()).ok();
            if let Some(ident) = maybe_ident {
                let ident_str = ident.to_string();
                return Ok(vec![
                    quote!(settings_schema::PathSegment::Identifier(#ident_str.into())),
                ]);
            }
        }
        Expr::Call(call) => {
            if call.to_token_stream().to_string() == "parent()" {
                return Ok(vec![quote!(settings_schema::PathSegment::Parent)]);
            }
        }
        Expr::MethodCall(ExprMethodCall {
            attrs,
            receiver,
            method,
            turbofish,
            args,
            ..
        }) => {
            if args.is_empty() && attrs.is_empty() && turbofish.is_none() && method == "parent" {
                let mut segments_ts = parse_segments(&receiver)?;
                segments_ts.push(quote!(settings_schema::PathSegment::Parent));

                return Ok(segments_ts);
            }
        }
        Expr::Field(ExprField {
            attrs,
            base,
            member,
            ..
        }) => {
            if attrs.is_empty() {
                if let Member::Named(ident) = &member {
                    let mut segments_ts = parse_segments(&base)?;
                    let ident_str = ident.to_string();
                    segments_ts
                        .push(quote!(settings_schema::PathSegment::Identifier(#ident_str.into())));

                    return Ok(segments_ts);
                }
            }
        }
        Expr::Index(ExprIndex {
            attrs, expr, index, ..
        }) => {
            if attrs.is_empty() {
                let index_lit = syn::parse2::<LitStr>(index.to_token_stream())
                    .map_err(|e| e.to_compile_error())?;

                let mut segments_ts = parse_segments(&expr)?;
                let lit_str = index_lit.value();
                segments_ts.push(quote!(settings_schema::PathSegment::Subscript(#lit_str.into())));

                return Ok(segments_ts);
            }
        }
        _ => (),
    }

    error(ERR_MSG, expr)
}

fn parse_path_string(path: &str) -> TResult {
    let path_expr = syn::parse_str::<Expr>(path).map_err(|e| e.to_compile_error())?;

    let segments_ts = parse_segments(&path_expr)?;

    Ok(quote!(vec![#(#segments_ts),*]))
}

fn parse_var_path_string(path: &str) -> TResult {
    if path == "input" {
        Ok(quote!(settings_schema::ModifierVariable::Input))
    } else {
        let segments_vec_ts = parse_path_string(path)?;
        Ok(quote!(settings_schema::ModifierVariable::Path(#segments_vec_ts)))
    }
}

pub fn schema(setting: &HigherOrderSetting) -> TResult {
    let key = &setting.name;

    let data_type_ts = match &setting.data_type {
        HigherOrderType::Choice {
            default,
            variants,
            gui,
        } => {
            let gui_ts = match gui {
                ChoiceControlType::Dropdown => quote!(settings_schema::ChoiceControlType::DropDown),
                ChoiceControlType::ButtonGroup => {
                    quote!(settings_schema::ChoiceControlType::ButtonGroup)
                }
            };

            quote!(settings_schema::HigherOrderType::Choice {
                default: #default.into()
                variants: vec![#(#variants.into()),*],
                gui: #gui_ts,
            })
        }
        HigherOrderType::Bool { default } => {
            quote!(settings_schema::HigherOrderType::Bool { default: #default })
        }
        HigherOrderType::Action => quote!(settings_schema::HigherOrderType::Action),
    };

    let mut modifiers_ts = vec![];
    for m in &setting.modifiers {
        let target_path_ts = parse_path_string(&m.target)?;

        let update_type_ts = match m.update_op {
            UpdateType::Assign => quote!(settings_schema::UpdateType::Assign),
            UpdateType::Remove => quote!(settings_schema::UpdateType::Remove),
        };

        let expr = &m.expr;

        let mut modifier_vars_ts = vec![];
        for var in &m.vars {
            modifier_vars_ts.push(parse_var_path_string(var)?);
        }

        modifiers_ts.push(quote!(settings_schema::ModifierDesc {
            target: #target_path_ts,
            update_operation: #update_type_ts,
            expression: #expr.into(),
            variables: vec![#(#modifier_vars_ts),*]
        }))
    }

    Ok(quote! {
        (
            #key.into(),
            settings_schema::EntryType::HigherOrder {
                data_type: #data_type_ts,
                modifiers: vec![#(#modifiers_ts),*],
            }
        )
    })
}
