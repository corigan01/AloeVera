/*
  ____                 __               __   _ __
 / __ \__ _____ ____  / /___ ____ _    / /  (_) /
/ /_/ / // / _ `/ _ \/ __/ // /  ' \  / /__/ / _ \
\___\_\_,_/\_,_/_//_/\__/\_,_/_/_/_/ /____/_/_.__/
    Part of the Quantum OS Project

Copyright 2025 Gavin Kellam

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute,
sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial
portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT
OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::ast;
use proc_macro2::Span;
use syn::{
    Attribute, Field, Fields, FnArg, Ident, ItemEnum, ItemStruct, LitBool, LitStr, ReturnType,
    Token, TraitItemFn, parse::Parse, spanned::Spanned,
};

impl Parse for ast::ProtocolKind {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let string: LitStr = input.parse()?;
        let str_value = string.value();

        if &str_value == "syscall" {
            Ok(Self::Syscall)
        } else if &str_value == "ipc" {
            Ok(Self::Ipc)
        } else {
            Ok(Self::Invalid)
        }
    }
}

mod portal_keywords {
    syn::custom_keyword!(global);
    syn::custom_keyword!(protocol);
    syn::custom_keyword!(event);
    syn::custom_keyword!(handle);
}

impl Parse for ast::PortalMacroArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut global: Option<LitBool> = None;
        let mut protocol = None;

        loop {
            if input.is_empty() {
                break;
            }

            let lookahead = input.lookahead1();
            if lookahead.peek(portal_keywords::global) {
                input.parse::<portal_keywords::global>()?;
                input.parse::<Token![=]>()?;
                global = Some(input.parse()?);
            } else if lookahead.peek(portal_keywords::protocol) {
                input.parse::<portal_keywords::protocol>()?;
                input.parse::<Token![=]>()?;
                protocol = Some(input.parse()?);
            } else {
                return Err(lookahead.error());
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            protocol_kind: protocol.unwrap_or(ast::ProtocolKind::Ipc),
            is_global: global.map(|gl| gl.value).unwrap_or(false),
        })
    }
}

impl Parse for ast::PortalMacro {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attr = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let _trait_token: Token![trait] = input.parse()?;
        let portal_name = input.parse()?;

        let inner;
        let _brace_token = syn::braced!(inner in input);
        let mut endpoints = Vec::new();

        loop {
            if inner.is_empty() {
                break;
            }

            while inner.peek(Token![;]) {
                inner.parse::<Token![;]>()?;
            }

            let item_fn: ast::ProtocolEndpoint = match inner.parse() {
                Ok(v) => v,
                Err(err) => {
                    return Err(syn::Error::new(
                        err.span(),
                        format!("Cannot parse endpoint function: {}", err),
                    ));
                }
            };

            endpoints.push(item_fn);
        }

        let portal_macro = Self {
            doc_attributes: attr,
            args: None,
            vis,
            trait_ident: portal_name,
            endpoints,
        };

        // Check for duplicate IDs
        for (duplicate_id, _duplicate_source, duplicate_use) in
            portal_macro.all_non_unique_portal_ids()
        {
            let _new_id = portal_macro.highest_id() + 1;

            return Err(syn::Error::new(
                duplicate_use,
                format!(
                    "Cannot have two endpoint functions with the same ID {}",
                    duplicate_id
                ),
            ));
        }

        Ok(portal_macro)
    }
}

fn convert_attribute_to_id_kind(
    attribute: Attribute,
) -> syn::Result<(usize, Span, ast::ProtocolEndpointKind)> {
    if attribute.path().is_ident("event") {
        let name_value = attribute.meta.require_name_value()?;
        match &name_value.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(expr_lit),
                ..
            }) => {
                let id = expr_lit.base10_parse()?;
                Ok((id, expr_lit.span(), ast::ProtocolEndpointKind::Event))
            }
            _ => Err(syn::Error::new(
                attribute.span(),
                "Only integer literals are supported 'event' IDs",
            )),
        }
    } else if attribute.path().is_ident("handle") {
        let name_value = attribute.meta.require_name_value()?;
        match &name_value.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(expr_lit),
                ..
            }) => {
                let id = expr_lit.base10_parse()?;
                Ok((id, expr_lit.span(), ast::ProtocolEndpointKind::Handle))
            }
            _ => Err(syn::Error::new(
                attribute.span(),
                "Only integer literals are supported 'handle' IDs",
            )),
        }
    } else {
        Err(syn::Error::new(
            attribute.span(),
            format!(
                "Attribute '{}' not supported.",
                attribute.span().source_text().as_deref().unwrap_or("??")
            ),
        ))
    }
}

impl Parse for ast::ProtocolEndpoint {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let TraitItemFn {
            attrs,
            sig,
            default,
            semi_token: _,
        } = input.parse()?;

        let (doc_attributes, mut remaining): (Vec<_>, Vec<_>) = attrs
            .into_iter()
            .partition(|attr| attr.path().is_ident("doc"));

        let (id, span, kind) = remaining
            .pop()
            .map(convert_attribute_to_id_kind)
            .ok_or(syn::Error::new(input.span(), "Must define endpoint kind"))??;

        if !remaining.is_empty() {
            return Err(syn::Error::new(
                span,
                "Only one endpoint specifier is allowed",
            ));
        }

        let input_args = sig
            .inputs
            .into_iter()
            .map(|arg| arg.try_into())
            .collect::<syn::Result<_>>()?;
        let output_arg = sig.output.try_into()?;

        let mut body = Vec::new();
        if let Some(block) = default {
            for statement in block.stmts.iter() {
                match statement {
                    syn::Stmt::Item(syn::Item::Enum(enum_statement)) => {
                        body.push(ast::ProtocolDefine::DefinedEnum(Rc::new(RefCell::new(
                            enum_statement.try_into()?,
                        ))))
                    }
                    syn::Stmt::Item(syn::Item::Struct(struct_statement)) => {
                        body.push(ast::ProtocolDefine::DefinedStruct(Rc::new(RefCell::new(
                            struct_statement.try_into()?,
                        ))))
                    }
                    stmt => {
                        return Err(syn::Error::new(
                            stmt.span(),
                            "Only `enum` definitions are currently supported",
                        ));
                    }
                }
            }
        }

        Ok(Self {
            doc_attributes,
            portal_id: (id, span),
            kind,
            fn_ident: sig.ident,
            input_args,
            output_arg,
            body,
            is_unsafe: sig.unsafety.is_some(),
            is_async: sig.asyncness.is_some(),
        })
    }
}

impl TryFrom<FnArg> for ast::ProtocolInputArg {
    type Error = syn::Error;
    fn try_from(value: FnArg) -> Result<Self, Self::Error> {
        match value {
            FnArg::Receiver(receiver) => Err(syn::Error::new(
                receiver.span(),
                "Self in endpoint is not supported, please remove all `self`",
            )),
            FnArg::Typed(pat_type) => {
                let argument_ident = match pat_type.pat.as_ref() {
                    syn::Pat::Ident(pat_ident) => Ok(pat_ident.ident.clone()),
                    _ => Err(syn::Error::new(
                        pat_type.span(),
                        "Only direct identifiers are supported in function arguments",
                    )),
                }?;

                Ok(Self {
                    argument_ident,
                    ty: pat_type.ty.as_ref().try_into()?,
                })
            }
        }
    }
}

impl TryFrom<ReturnType> for ast::ProtocolOutputArg {
    type Error = syn::Error;
    fn try_from(value: ReturnType) -> Result<Self, Self::Error> {
        match value {
            ReturnType::Default => Ok(Self(ast::ProtocolVarType::Unit(value.span()))),
            ReturnType::Type(_, ty) => Ok(Self(ty.as_ref().try_into()?)),
        }
    }
}

impl TryFrom<&syn::Type> for ast::ProtocolVarType {
    type Error = syn::Error;
    fn try_from(value: &syn::Type) -> Result<Self, Self::Error> {
        match value {
            syn::Type::Never(type_never) => Ok(Self::Never(type_never.span())),
            syn::Type::Path(type_path) => {
                let path = type_path.path.segments.last().ok_or(syn::Error::new(
                    type_path.span(),
                    format!(
                        "Type '{}' is not currently supported by portal",
                        type_path.span().source_text().as_deref().unwrap_or("??")
                    ),
                ))?;

                match path.ident.to_string().as_str() {
                    "Result" => match &path.arguments {
                        syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                            let mut gen_iter = angle_bracketed_generic_arguments.args.iter();
                            match (gen_iter.next(), gen_iter.next(), gen_iter.next_back()) {
                                (
                                    Some(syn::GenericArgument::Type(ok_ty)),
                                    Some(syn::GenericArgument::Type(err_ty)),
                                    None,
                                ) => Ok(Self::ResultKind {
                                    span: path.span(),
                                    ok_ty: Box::new(Self::try_from(ok_ty)?),
                                    err_ty: Box::new(Self::try_from(err_ty)?),
                                }),
                                _ => Err(syn::Error::new(
                                    type_path.span(),
                                    format!(
                                        "Result '{}' only supports 2 generic arguments",
                                        type_path.span().source_text().as_deref().unwrap_or("??")
                                    ),
                                )),
                            }
                        }
                        _ => Err(syn::Error::new(
                            type_path.span(),
                            format!(
                                "Type '{}' has invalid syntax",
                                type_path.span().source_text().as_deref().unwrap_or("??")
                            ),
                        )),
                    },
                    "Vec" => match &path.arguments {
                        syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                            let mut gen_iter = angle_bracketed_generic_arguments.args.iter();
                            match (gen_iter.next(), gen_iter.next()) {
                                (Some(syn::GenericArgument::Type(inner_ty)), None) => {
                                    Ok(Self::IpcVec {
                                        span: path.span(),
                                        to: Box::new(Self::try_from(inner_ty)?),
                                    })
                                }
                                _ => Err(syn::Error::new(
                                    type_path.span(),
                                    format!(
                                        "Vec '{}' only supports 1 generic argument",
                                        type_path.span().source_text().as_deref().unwrap_or("??")
                                    ),
                                )),
                            }
                        }
                        _ => Err(syn::Error::new(
                            type_path.span(),
                            format!(
                                "Type '{}' has invalid syntax",
                                type_path.span().source_text().as_deref().unwrap_or("??")
                            ),
                        )),
                    },
                    "bool" => Ok(Self::Bool(path.span())),
                    "i8" => Ok(Self::Signed8(path.span())),
                    "i16" => Ok(Self::Signed16(path.span())),
                    "i32" => Ok(Self::Signed32(path.span())),
                    "i64" => Ok(Self::Signed64(path.span())),
                    "u8" => Ok(Self::Unsigned8(path.span())),
                    "u16" => Ok(Self::Unsigned16(path.span())),
                    "u32" => Ok(Self::Unsigned32(path.span())),
                    "u64" => Ok(Self::Unsigned64(path.span())),
                    "usize" => Ok(Self::UnsignedSize(path.span())),
                    "str" => Ok(Self::Str(path.span())),
                    "String" => Ok(Self::IpcString(path.span())),
                    user_defined => Ok(Self::Unknown(Ident::new(user_defined, type_path.span()))),
                }
            }
            syn::Type::Ptr(type_ptr) => Ok(Self::PtrTo {
                span: type_ptr.span(),
                is_mut: type_ptr.mutability.is_some(),
                to: Box::new(Self::try_from(type_ptr.elem.as_ref())?),
            }),
            syn::Type::Slice(type_slice) => Ok(Self::Array {
                span: type_slice.span(),
                to: Box::new(Self::try_from(type_slice.elem.as_ref())?),
                len: None,
            }),
            syn::Type::Array(type_array) => Ok(Self::Array {
                span: type_array.span(),
                to: Box::new(Self::try_from(type_array.elem.as_ref())?),
                len: Some(match &type_array.len {
                    syn::Expr::Lit(syn::PatLit {
                        attrs: _,
                        lit: syn::Lit::Int(lit_int),
                    }) => lit_int.base10_parse()?,
                    _ => {
                        return Err(syn::Error::new(
                            type_array.span(),
                            format!(
                                "Expected Array's length must be a literal, found '{}'",
                                type_array
                                    .len
                                    .span()
                                    .source_text()
                                    .as_deref()
                                    .unwrap_or("??")
                            ),
                        ));
                    }
                }),
            }),
            syn::Type::Reference(type_reference) => Ok(Self::RefTo {
                span: type_reference.span(),
                is_mut: type_reference.mutability.is_some(),
                to: Box::new(Self::try_from(type_reference.elem.as_ref())?),
            }),
            syn::Type::Tuple(type_tuple) => {
                if type_tuple.elems.is_empty() {
                    Ok(Self::Unit(type_tuple.span()))
                } else {
                    Err(syn::Error::new(
                        type_tuple.span(),
                        format!(
                            "Type '{}' is not currently supported by portal",
                            type_tuple.span().source_text().as_deref().unwrap_or("??")
                        ),
                    ))
                }
            }
            _ => Err(syn::Error::new(
                value.span(),
                format!(
                    "Type '{}' is not currently supported by portal",
                    value.span().source_text().as_deref().unwrap_or("??")
                ),
            )),
        }
    }
}

impl TryFrom<&ItemStruct> for ast::ProtocolStructDef {
    type Error = syn::Error;

    fn try_from(value: &ItemStruct) -> Result<Self, Self::Error> {
        let ItemStruct {
            attrs,
            vis: _,
            struct_token: _,
            ident,
            generics,
            fields,
            semi_token: _,
        } = value;

        let mut docs = Vec::new();
        for attr in attrs {
            if attr.path().is_ident("doc") {
                docs.push(attr.clone());
            } else {
                return Err(syn::Error::new(
                    attr.span(),
                    "Attribute not supported for portal defined enum",
                ));
            }
        }

        if !generics.params.is_empty() {
            return Err(syn::Error::new(
                generics.span(),
                "Portal defined enum cannot have any generics",
            ));
        }

        let mut items = Vec::new();
        for field in fields.iter() {
            items.push(field.try_into()?);
        }

        Ok(Self {
            docs,
            ident: ident.clone(),
            items,
        })
    }
}

impl TryFrom<&Field> for ast::ProtocolStructItem {
    type Error = syn::Error;

    fn try_from(value: &Field) -> Result<Self, Self::Error> {
        let Field {
            attrs,
            vis: _,
            mutability: _,
            ident,
            colon_token: _,
            ty,
        } = value;

        let mut docs = Vec::new();
        for attr in attrs {
            if attr.path().is_ident("doc") {
                docs.push(attr.clone());
            } else {
                return Err(syn::Error::new(
                    attr.span(),
                    "Attribute not supported for portal defined enum",
                ));
            }
        }

        Ok(Self {
            docs,
            name: ident.clone(),
            ty: ty.try_into()?,
        })
    }
}

impl TryFrom<&ItemEnum> for ast::ProtocolEnumDef {
    type Error = syn::Error;

    fn try_from(value: &ItemEnum) -> Result<Self, Self::Error> {
        let ItemEnum {
            attrs,
            vis: _,
            enum_token: _,
            ident,
            generics,
            brace_token: _,
            variants,
        } = value;

        let mut docs = Vec::new();
        for attr in attrs {
            if attr.path().is_ident("doc") {
                docs.push(attr.clone());
            } else {
                return Err(syn::Error::new(
                    attr.span(),
                    "Attribute not supported for portal defined enum",
                ));
            }
        }

        if !generics.params.is_empty() {
            return Err(syn::Error::new(
                generics.span(),
                "Portal defined enum cannot have any generics",
            ));
        }

        let mut requires_lifetime = false;
        let mut parsed_varients = Vec::new();
        for variant in variants.iter() {
            let parsed: ast::ProtocolEnumVarient = variant.try_into()?;
            if parsed.fields.requires_lifetime() {
                requires_lifetime = true;
            }

            parsed_varients.push(parsed);
        }

        Ok(Self {
            docs,
            requires_lifetime,
            ident: ident.clone(),
            varients: parsed_varients,
        })
    }
}

impl TryFrom<&syn::Variant> for ast::ProtocolEnumVarient {
    type Error = syn::Error;
    fn try_from(value: &syn::Variant) -> Result<Self, Self::Error> {
        let ident = value.ident.clone();
        let docs = value.attrs.clone();
        Ok(Self {
            ident,
            fields: (&value.fields).try_into()?,
            docs,
        })
    }
}

impl TryFrom<&Fields> for ast::ProtocolEnumFields {
    type Error = syn::Error;
    fn try_from(value: &syn::Fields) -> Result<Self, Self::Error> {
        match value {
            Fields::Named(fields_named) => {
                let mut map = HashMap::new();
                for field in fields_named.named.iter() {
                    map.insert(
                        field.ident.clone().ok_or(syn::Error::new(
                            fields_named.span(),
                            "Expected named field to have an ident",
                        ))?,
                        (&field.ty).try_into()?,
                    );
                }

                Ok(Self::Named(map))
            }
            Fields::Unnamed(fields_unnamed) => {
                let mut vec = Vec::new();
                for field in fields_unnamed.unnamed.iter() {
                    vec.push((&field.ty).try_into()?);
                }

                Ok(Self::Unnamed(vec))
            }
            Fields::Unit => Ok(Self::None),
        }
    }
}
