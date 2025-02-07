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

use crate::ast;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote::TokenStreamExt;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;
use syn::Lifetime;

/// A generator for the portal's trait
pub struct PortalTrait<'a> {
    portal: &'a ast::PortalMacro,
}

impl<'a> PortalTrait<'a> {
    pub fn new(portal: &'a ast::PortalMacro) -> Self {
        Self { portal }
    }
}

/// A generator for the portal's user defined types
pub struct PortalUserDefined<'a> {
    portal: &'a ast::PortalMacro,
}

impl<'a> PortalUserDefined<'a> {
    pub fn new(portal: &'a ast::PortalMacro) -> Self {
        Self { portal }
    }
}

/// A generator for the enum that all functions will put their arguments
pub struct PortalTranslationInputType<'a> {
    portal: &'a ast::PortalMacro,
}

impl<'a> PortalTranslationInputType<'a> {
    pub fn new(portal: &'a ast::PortalMacro) -> Self {
        Self { portal }
    }
}

/// A generator for the enum that all functions will output
pub struct PortalTranslationOutputType<'a> {
    portal: &'a ast::PortalMacro,
}

impl<'a> PortalTranslationOutputType<'a> {
    pub fn new(portal: &'a ast::PortalMacro) -> Self {
        Self { portal }
    }
}

/// A generator for a type that requires a lifetime
pub struct LifetimedProtocolVarType<'a> {
    lifetime_ident: &'a Lifetime,
    ty: &'a ast::ProtocolVarType,
}

impl<'a> LifetimedProtocolVarType<'a> {
    pub fn new(lifetime_ident: &'a Lifetime, ty: &'a ast::ProtocolVarType) -> Self {
        Self { lifetime_ident, ty }
    }
}

/// A generator for QuantumOS's into syscall
/// (aka. The default type that will impl the portal's trait)
pub struct IntoPortalImpl<'a> {
    portal: &'a ast::PortalMacro,
}

impl<'a> IntoPortalImpl<'a> {
    pub fn new(portal: &'a ast::PortalMacro) -> Self {
        Self { portal }
    }
}

pub fn generate_rust_portal(portal: &ast::PortalMacro) -> TokenStream2 {
    portal.to_token_stream()
}

impl ToTokens for ast::PortalMacro {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let portal_trait = PortalTrait::new(self);
        let user_defined = PortalUserDefined::new(self);
        let input = PortalTranslationInputType::new(self);
        let output = PortalTranslationOutputType::new(self);
        let into_portal_impl = IntoPortalImpl::new(self);

        let doc_attr = self.doc_attributes.iter();
        let portal_ident = self.get_mod_ident();
        let vis = &self.vis;

        tokens.append_all(quote! {
            #(#doc_attr)*
            #vis mod #portal_ident {
                #user_defined
                #input
                #output
                #portal_trait
                #into_portal_impl
            }
        });
    }
}

impl<'a> ToTokens for PortalTrait<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let trait_ident = &self.portal.trait_ident;
        let endpoints = &self.portal.endpoints;

        tokens.append_all(quote! {
            pub trait #trait_ident {
                #(#endpoints)*
            }
        });
    }
}

impl ToTokens for ast::ProtocolEndpoint {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let fn_ident = &self.fn_ident;
        let docs = &self.doc_attributes;
        let arguments = &self.input_args;
        let return_type = &self.output_arg;

        tokens.append_all(quote! {
            #(#docs)*
            fn #fn_ident(#(#arguments),*) #return_type;

        });
    }
}

impl ToTokens for ast::ProtocolInputArg {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let argument_name = &self.argument_ident;
        let ty = &self.ty;

        tokens.append_all(quote! {
            #argument_name : #ty
        });
    }
}

impl ToTokens for ast::ProtocolOutputArg {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let inner = &self.0;
        if !matches!(inner, ast::ProtocolVarType::Unit(_)) {
            tokens.append_all(quote! { -> #inner});
        }
    }
}

impl<'a> ToTokens for PortalUserDefined<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let user_defined_types = self
            .portal
            .endpoints
            .iter()
            .flat_map(|endpoint| endpoint.body.iter());
        tokens.append_all(quote! {
            #(#user_defined_types)*
        });
    }
}

impl ToTokens for ast::ProtocolDefine {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ast::ProtocolDefine::DefinedEnum(ref_cell) => {
                let enum_def = ref_cell.borrow();

                let docs = &enum_def.docs;
                let ident = &enum_def.ident;
                let varients = &enum_def.varients;

                let lifetime = if enum_def.requires_lifetime {
                    quote! {<'defined>}
                } else {
                    quote! {}
                };

                tokens.append_all(quote! {
                    #(#docs)*
                    pub enum #ident #lifetime {
                        #(#varients),*
                    }
                });
            }
        }
    }
}

impl ToTokens for ast::ProtocolEnumVarient {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let fields = &self.fields;

        tokens.append_all(quote! {
            #ident #fields
        });
    }
}

impl ToTokens for ast::ProtocolEnumFields {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ast::ProtocolEnumFields::None => {
                tokens.append_all(quote! {});
            }
            ast::ProtocolEnumFields::Unnamed(protocol_var_types) => {
                tokens.append_all(quote! {(#(#protocol_var_types),*)});
            }
            ast::ProtocolEnumFields::Named(hash_map) => {
                let var_defs = hash_map.iter().map(|(name, ty)| quote! { #name : #ty });

                tokens.append_all(quote! {
                    { #(#var_defs)* }
                });
            }
        }
    }
}

impl<'a> ToTokens for PortalTranslationInputType<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let lifetime = Lifetime::new("'a", Span::call_site());

        let translation_ident = self.portal.get_input_enum_ident();
        let varients = self.portal.endpoints.iter().map(|endpoint| {
            let named_var = endpoint.input_args.iter().map(|input_arg| {
                let ty = LifetimedProtocolVarType::new(&lifetime, &input_arg.ty);
                let ident = &input_arg.argument_ident;

                quote! {
                    #ident : #ty
                }
            });
            let endpoint_enum_name = format_ident!("{}Endpoint", endpoint.get_enum_ident());

            let fields = if !endpoint.input_args.is_empty() {
                quote! { { #(#named_var),* } }
            } else {
                quote! { }
            };

            quote! {
                #endpoint_enum_name #fields,
            }
        });

        // TODO: We should try and not emit this field in the future, and look to see if we
        // actually need to use the lifetime.
        tokens.append_all(quote! {
            pub enum #translation_ident<#lifetime> {
                #(#varients)*
                UnusedPhantomData(core::marker::PhantomData<&#lifetime ()>)
            }
        });
    }
}


impl<'a> ToTokens for PortalTranslationOutputType<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let translation_ident = self.portal.get_output_enum_ident();
        let varients = self.portal.endpoints.iter().map(|endpoint| {
            let var_output = &endpoint.output_arg.0;
            let endpoint_enum_name = format_ident!("{}Endpoint", endpoint.get_enum_ident());

            let fields = if !matches!(var_output, ast::ProtocolVarType::Unit(_)) && !matches!(var_output, ast::ProtocolVarType::Never(_)) {
                quote! { ( #var_output ) }
            } else {
                quote! {}
            };

            quote! {
                #endpoint_enum_name #fields,
            }
        });

        tokens.append_all(quote! {
            pub enum #translation_ident {
                #(#varients)*
            }
        });
    }
}

impl ToTokens for ast::ProtocolVarType {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ast::ProtocolVarType::ResultKind {
                span,
                ok_ty,
                err_ty,
            } => {
                tokens.append_all(quote_spanned! {span.clone()=>::core::result::Result});
                tokens.append_all(quote! {<#ok_ty, #err_ty>});
            }
            ast::ProtocolVarType::Never(span) => {
                tokens.append_all(quote_spanned! {span.clone()=>!})
            }
            ast::ProtocolVarType::Unit(span) => {
                tokens.append_all(quote_spanned! {span.clone()=>()})
            }
            ast::ProtocolVarType::Signed8(span) => {
                tokens.append_all(quote_spanned! {span.clone()=>i8})
            }
            ast::ProtocolVarType::Signed16(span) => {
                tokens.append_all(quote_spanned! {span.clone()=>i16})
            }
            ast::ProtocolVarType::Signed32(span) => {
                tokens.append_all(quote_spanned! {span.clone()=>i32})
            }
            ast::ProtocolVarType::Signed64(span) => {
                tokens.append_all(quote_spanned! {span.clone()=>i64})
            }
            ast::ProtocolVarType::Unsigned8(span) => {
                tokens.append_all(quote_spanned! {span.clone()=>u8})
            }
            ast::ProtocolVarType::Unsigned16(span) => {
                tokens.append_all(quote_spanned! {span.clone()=>u16})
            }
            ast::ProtocolVarType::Unsigned32(span) => {
                tokens.append_all(quote_spanned! {span.clone()=>u32})
            }
            ast::ProtocolVarType::Unsigned64(span) => {
                tokens.append_all(quote_spanned! {span.clone()=>u64})
            }
            ast::ProtocolVarType::UnsignedSize(span) => {
                tokens.append_all(quote_spanned! {span.clone()=>usize})
            }
            ast::ProtocolVarType::Unknown(ident) => {
                let error_msg = format!("Unknown Type '{}'", ident.to_string());
                tokens.append_all(quote_spanned! {ident.span()=>compile_error!(#error_msg)})
            }
            ast::ProtocolVarType::UserDefined { span, to } => {
                let type_ident = to.var_ident();
                tokens.append_all(quote_spanned! {span.clone()=>#type_ident})
            }
            ast::ProtocolVarType::Str(span) => {
                tokens.append_all(quote_spanned! {span.clone()=>str})
            }
            ast::ProtocolVarType::RefTo { span, is_mut, to } if *is_mut => {
                tokens.append_all(quote_spanned! {span.clone()=>&mut #to})
            }
            ast::ProtocolVarType::RefTo { span, to, .. } => {
                tokens.append_all(quote_spanned! {span.clone()=>&#to})
            }
            ast::ProtocolVarType::PtrTo { span, is_mut, to } if *is_mut => {
                tokens.append_all(quote_spanned! {span.clone()=>*mut #to})
            }
            ast::ProtocolVarType::PtrTo { span, to, .. } => {
                tokens.append_all(quote_spanned! {span.clone()=>*const #to})
            }
        }
    }
}

impl<'a> ToTokens for LifetimedProtocolVarType<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self.ty {
            ast::ProtocolVarType::ResultKind {
                span,
                ok_ty,
                err_ty,
            } => {
                let ok_ty = Self::new(&self.lifetime_ident, &ok_ty);
                let err_ty = Self::new(&self.lifetime_ident, &err_ty);

                tokens.append_all(quote_spanned! {span.clone()=>::core::result::Result});
                tokens.append_all(quote! {<#ok_ty, #err_ty>});
            }
            ast::ProtocolVarType::RefTo { span, is_mut, to } => {
                let lifetime = self.lifetime_ident;
                tokens.append_all(quote_spanned! {span.clone()=>&});
                tokens.append_all(quote! {#lifetime});

                if *is_mut {
                    tokens.append_all(quote! {mut});
                }

                let to = Self::new(&self.lifetime_ident, &to);
                tokens.append_all(quote! { #to });
            }
            ast::ProtocolVarType::PtrTo { span, is_mut, to } => {
                tokens.append_all(quote_spanned! {span.clone()=>*});
                if *is_mut {
                    tokens.append_all(quote! {mut});
                } else {
                    tokens.append_all(quote! {const});
                }

                let to = Self::new(&self.lifetime_ident, &to);
                tokens.append_all(quote! { #to });
            }
            ty => ty.to_tokens(tokens),
        }
    }
}

impl<'a> ToTokens for IntoPortalImpl<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = self.portal.get_quantum_os_impl_ident();
        let trait_ident = &self.portal.trait_ident;
        let input_enum = self.portal.get_input_enum_ident();
        let output_enum = self.portal.get_output_enum_ident();

        let endpoints = self.portal.endpoints.iter().map(|endpoint| {
            let fn_ident = &endpoint.fn_ident;
            let docs = &endpoint.doc_attributes;
            let arguments = &endpoint.input_args;
            let return_type = &endpoint.output_arg;
            let enum_part = format_ident!("{}Endpoint", endpoint.get_enum_ident());

            let syscall_part = if arguments.len() > 0 {
                let argument_in_body = endpoint.input_args.iter().map(|input_arg| {
                    let name = &input_arg.argument_ident;
                   quote! { #name } 
                });

                quote! { match (unsafe { Self::call_syscall(#input_enum::#enum_part {#(#argument_in_body),*}) }) }
            } else {
                quote! { match (unsafe { Self::call_syscall(#input_enum::#enum_part) }) }
            };

            let fn_closing = if matches!(endpoint.output_arg.0, ast::ProtocolVarType::Never(_)) {
                let fmt_string = format!("Portal Endpoint '{}' promised to never return, but yet returned!", fn_ident);
                quote! {
                    {
                        #output_enum::#enum_part => { unreachable!(#fmt_string); }
                        _ => todo!(),
                    };
                }
            } else {
                quote! {
                    {
                        _ => todo!(),
                    };
                }
            };

            // TODO: In the future we should try and reduce the need to put values into
            //       the input argument enum, and instead try and serialize the values
            //       into the ~6 CPU registers we have for syscalls. It would improve
            //       performance, and be easier for C to call.
            quote! {
                #(#docs)*
                fn #fn_ident(#(#arguments),*) #return_type {
                    #syscall_part
                    #fn_closing
                }
            }
        });

        tokens.append_all(quote! {
            pub struct #ident {}
        });
        tokens.append_all(quote! {
            impl #trait_ident for #ident {
                #(#endpoints)*
            }
        });
        tokens.append_all(quote! {
            impl #ident {
                pub unsafe fn call_syscall<'syscall>(arguments: #input_enum<'syscall>) -> #output_enum {
                    todo!()
                }
            }
        });
    }
}
