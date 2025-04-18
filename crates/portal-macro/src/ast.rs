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

use proc_macro2::Span;
use quote::format_ident;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use syn::{Attribute, Ident, Visibility};

#[cfg(any(feature = "ipc-client", feature = "ipc-server"))]
use proc_macro2::TokenStream;

#[cfg(any(feature = "ipc-client", feature = "ipc-server"))]
pub trait ClientServerTokens {
    fn client_tokens(&self) -> TokenStream;
    fn server_tokens(&self) -> TokenStream;
}

#[derive(Debug)]
#[allow(unused)]
pub struct PortalMacro {
    pub doc_attributes: Vec<Attribute>,
    pub args: Option<PortalMacroArgs>,
    pub vis: Visibility,
    pub trait_ident: Ident,
    pub endpoints: Vec<ProtocolEndpoint>,
}

#[derive(Debug)]
#[allow(unused)]
pub struct PortalMacroArgs {
    pub protocol_kind: ProtocolKind,
    pub is_global: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolKind {
    Syscall,
    Ipc,
    Invalid,
}

#[derive(Debug)]
#[allow(unused)]
pub struct ProtocolEndpoint {
    pub doc_attributes: Vec<Attribute>,
    pub portal_id: (usize, Span),
    pub kind: ProtocolEndpointKind,
    pub fn_ident: Ident,
    pub input_args: Vec<ProtocolInputArg>,
    pub output_arg: ProtocolOutputArg,
    pub is_unsafe: bool,
    pub is_async: bool,
    pub body: Vec<ProtocolDefine>,
}

#[derive(Debug)]
#[allow(unused)]
pub enum ProtocolVarType {
    ResultKind {
        span: Span,
        ok_ty: Box<ProtocolVarType>,
        err_ty: Box<ProtocolVarType>,
    },
    Never(Span),
    Unit(Span),
    Bool(Span),
    Signed8(Span),
    Signed16(Span),
    Signed32(Span),
    Signed64(Span),
    Unsigned8(Span),
    Unsigned16(Span),
    Unsigned32(Span),
    Unsigned64(Span),
    UnsignedSize(Span),
    Unknown(Ident),
    UserDefined {
        span: Span,
        to: ProtocolDefine,
    },
    IpcString(Span),
    IpcVec {
        span: Span,
        to: Box<ProtocolVarType>,
    },
    Str(Span),
    Array {
        span: Span,
        to: Box<ProtocolVarType>,
        len: Option<usize>,
    },
    RefTo {
        span: Span,
        is_mut: bool,
        to: Box<ProtocolVarType>,
    },
    PtrTo {
        span: Span,
        is_mut: bool,
        to: Box<ProtocolVarType>,
    },
}

#[derive(Debug)]
#[allow(unused)]
pub struct ProtocolInputArg {
    pub argument_ident: Ident,
    pub ty: ProtocolVarType,
}

#[derive(Debug)]
#[allow(unused)]
pub struct ProtocolOutputArg(pub ProtocolVarType);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProtocolEndpointKind {
    Event,
    Handle,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub enum ProtocolDefine {
    DefinedEnum(Rc<RefCell<ProtocolEnumDef>>),
    DefinedStruct(Rc<RefCell<ProtocolStructDef>>),
}

#[derive(Debug)]
#[allow(unused)]
pub struct ProtocolStructDef {
    pub docs: Vec<Attribute>,
    pub ident: Ident,
    pub items: Vec<ProtocolStructItem>,
}

#[derive(Debug)]
#[allow(unused)]
pub struct ProtocolStructItem {
    pub docs: Vec<Attribute>,
    pub name: Option<Ident>,
    pub ty: ProtocolVarType,
}

#[derive(Debug)]
#[allow(unused)]
pub struct ProtocolEnumDef {
    pub docs: Vec<Attribute>,
    pub requires_lifetime: bool,
    pub ident: Ident,
    pub varients: Vec<ProtocolEnumVarient>,
}

#[derive(Debug)]
#[allow(unused)]
pub struct ProtocolEnumVarient {
    pub docs: Vec<Attribute>,
    pub ident: Ident,
    pub fields: ProtocolEnumFields,
}

#[derive(Debug)]
#[allow(unused)]
pub enum ProtocolEnumFields {
    None,
    Unnamed(Vec<ProtocolVarType>),
    Named(HashMap<Ident, ProtocolVarType>),
}

impl ProtocolVarType {
    /// Runs `F` on the tree.
    ///
    /// Returns after the first `Some`
    pub fn search<F, R>(&self, f: &F) -> Option<R>
    where
        F: Fn(&Self) -> Option<R>,
    {
        if let Some(value) = f(self) {
            return Some(value);
        }

        if let Some(value) = match self {
            ProtocolVarType::Array {
                span: _,
                to,
                len: _,
            } => to.search(f),
            ProtocolVarType::ResultKind {
                span: _,
                ok_ty,
                err_ty,
            } => {
                if let Some(value) = ok_ty.search(f) {
                    return Some(value);
                }
                if let Some(value) = err_ty.search(f) {
                    return Some(value);
                }
                None
            }
            ProtocolVarType::RefTo {
                to,
                span: _,
                is_mut: _,
            } => to.search(f),
            ProtocolVarType::PtrTo {
                to,
                span: _,
                is_mut: _,
            } => to.search(f),
            _ => None,
        } {
            return Some(value);
        }

        None
    }

    /// Runs `F` on the tree.
    ///
    /// Returns after the first `Some`
    pub fn search_mut<F, R>(&mut self, f: &mut F) -> Option<R>
    where
        F: FnMut(&mut Self) -> Option<R>,
    {
        if let Some(value) = f(self) {
            return Some(value);
        }

        if let Some(value) = match self {
            ProtocolVarType::Array {
                span: _,
                to,
                len: _,
            } => to.search_mut(f),
            ProtocolVarType::ResultKind {
                span: _,
                ok_ty,
                err_ty,
            } => {
                if let Some(value) = ok_ty.search_mut(f) {
                    return Some(value);
                }
                if let Some(value) = err_ty.search_mut(f) {
                    return Some(value);
                }
                None
            }
            ProtocolVarType::RefTo {
                to,
                span: _,
                is_mut: _,
            } => to.search_mut(f),
            ProtocolVarType::PtrTo {
                to,
                span: _,
                is_mut: _,
            } => to.search_mut(f),
            _ => None,
        } {
            return Some(value);
        }

        None
    }

    /// Check if this type is allowed for this portal type
    pub fn check_allowed(&self, portal_type: ProtocolKind) -> Result<(), syn::Error> {
        match (portal_type, self) {
            (
                _,
                ProtocolVarType::ResultKind {
                    span: _,
                    ok_ty,
                    err_ty,
                },
            ) => {
                ok_ty.check_allowed(portal_type)?;
                err_ty.check_allowed(portal_type)?;
                Ok(())
            }
            (_, ProtocolVarType::Never(_)) => Ok(()),
            (_, ProtocolVarType::Unit(_)) => Ok(()),
            (_, ProtocolVarType::Bool(_)) => Ok(()),
            (_, ProtocolVarType::Signed8(_)) => Ok(()),
            (_, ProtocolVarType::Signed16(_)) => Ok(()),
            (_, ProtocolVarType::Signed32(_)) => Ok(()),
            (_, ProtocolVarType::Signed64(_)) => Ok(()),
            (_, ProtocolVarType::Unsigned8(_)) => Ok(()),
            (_, ProtocolVarType::Unsigned16(_)) => Ok(()),
            (_, ProtocolVarType::Unsigned32(_)) => Ok(()),
            (_, ProtocolVarType::Unsigned64(_)) => Ok(()),
            (_, ProtocolVarType::UnsignedSize(_)) => Ok(()),
            (_, ProtocolVarType::Unknown(_)) => Ok(()),

            (_, ProtocolVarType::UserDefined { span: _, to }) => match to {
                ProtocolDefine::DefinedEnum(ref_cell) => {
                    let enum_inner = ref_cell.borrow();
                    for enum_varients in &enum_inner.varients {
                        match &enum_varients.fields {
                            ProtocolEnumFields::None => (),
                            ProtocolEnumFields::Unnamed(protocol_var_types) => {
                                for var in protocol_var_types {
                                    var.check_allowed(portal_type)?;
                                }
                            }
                            ProtocolEnumFields::Named(hash_map) => {
                                for var in hash_map.values() {
                                    var.check_allowed(portal_type)?;
                                }
                            }
                        }
                    }

                    Ok(())
                }
                ProtocolDefine::DefinedStruct(ref_cell) => {
                    let struct_inner = ref_cell.borrow();
                    for struct_field in struct_inner.items.iter() {
                        struct_field.ty.check_allowed(portal_type)?;
                    }
                    Ok(())
                }
            },
            (ProtocolKind::Ipc, ProtocolVarType::IpcString(_)) => Ok(()),
            (ProtocolKind::Ipc, ProtocolVarType::IpcVec { span: _, to }) => {
                to.check_allowed(portal_type)
            }

            (ProtocolKind::Syscall, ProtocolVarType::Str(_)) => Ok(()),
            (
                ProtocolKind::Syscall,
                ProtocolVarType::Array {
                    span: _,
                    to,
                    len: _,
                },
            ) => to.check_allowed(portal_type),
            (
                ProtocolKind::Syscall,
                ProtocolVarType::RefTo {
                    span: _,
                    is_mut: _,
                    to,
                },
            ) => to.check_allowed(portal_type),
            (
                ProtocolKind::Syscall,
                ProtocolVarType::PtrTo {
                    span: _,
                    is_mut: _,
                    to,
                },
            ) => to.check_allowed(portal_type),

            (protocol, var) => Err(syn::Error::new(
                var.span(),
                format!(
                    "Cannot use '{}' with {:?} protocol!",
                    var.span().source_text().as_deref().unwrap_or("??"),
                    protocol
                ),
            )),
        }
    }

    /// Check if this type is of the same type as some other user type.
    pub fn is_unknown_of(&self, ident: &str) -> bool {
        match self {
            Self::Unknown(unknown) if unknown.to_string() == ident => true,
            _ => false,
        }
    }

    /// Get the span of this var type
    pub fn span(&self) -> Span {
        match self {
            ProtocolVarType::ResultKind { span, .. } => span.clone(),
            ProtocolVarType::Never(span) => span.clone(),
            ProtocolVarType::Unit(span) => span.clone(),
            ProtocolVarType::Signed8(span) => span.clone(),
            ProtocolVarType::Signed16(span) => span.clone(),
            ProtocolVarType::Signed32(span) => span.clone(),
            ProtocolVarType::Signed64(span) => span.clone(),
            ProtocolVarType::Unsigned8(span) => span.clone(),
            ProtocolVarType::Unsigned16(span) => span.clone(),
            ProtocolVarType::Unsigned32(span) => span.clone(),
            ProtocolVarType::Unsigned64(span) => span.clone(),
            ProtocolVarType::UnsignedSize(span) => span.clone(),
            ProtocolVarType::Unknown(ident) => ident.span().clone(),
            ProtocolVarType::UserDefined { span, .. } => span.clone(),
            ProtocolVarType::Str(span) => span.clone(),
            ProtocolVarType::RefTo { span, .. } => span.clone(),
            ProtocolVarType::PtrTo { span, .. } => span.clone(),
            ProtocolVarType::Array { span, .. } => span.clone(),
            ProtocolVarType::Bool(span) => span.clone(),
            ProtocolVarType::IpcString(span) => span.clone(),
            ProtocolVarType::IpcVec { span, to: _ } => span.clone(),
        }
    }
}

impl ProtocolDefine {
    /// Get the ident of the defined var
    pub fn var_ident(&self) -> Ident {
        match self {
            ProtocolDefine::DefinedEnum(ref_cell) => ref_cell.borrow().ident.clone(),
            ProtocolDefine::DefinedStruct(ref_cell) => ref_cell.borrow().ident.clone(),
        }
    }
}

impl ProtocolEndpoint {
    pub fn get_enum_ident(&self) -> Ident {
        let mut new_str = String::new();
        let mut next_char_should_be_upper = true;

        for old_char in self.fn_ident.to_string().chars() {
            if old_char == '_' {
                next_char_should_be_upper = true;
                continue;
            }

            new_str.push(if next_char_should_be_upper {
                next_char_should_be_upper = false;
                old_char.to_ascii_uppercase()
            } else {
                old_char.to_ascii_lowercase()
            });
        }

        Ident::new(&new_str, self.fn_ident.span())
    }
}

impl ProtocolEnumFields {
    pub fn requires_lifetime(&self) -> bool {
        match self {
            ProtocolEnumFields::None => false,
            ProtocolEnumFields::Unnamed(protocol_var_types) => {
                protocol_var_types.iter().any(|var| {
                    var.search(&|ty| match ty {
                        ProtocolVarType::RefTo { .. } => Some(true),
                        _ => None,
                    })
                    .unwrap_or(false)
                })
            }
            ProtocolEnumFields::Named(hash_map) => hash_map.values().any(|var| {
                var.search(&|ty| match ty {
                    ProtocolVarType::RefTo { .. } => Some(true),
                    _ => None,
                })
                .unwrap_or(false)
            }),
        }
    }
}

impl PortalMacro {
    pub fn is_syscall_kind(&self) -> bool {
        self.args
            .as_ref()
            .is_some_and(|args| args.protocol_kind == ProtocolKind::Syscall)
    }

    /// Get all the not unique portal ids
    pub fn all_non_unique_portal_ids(&self) -> impl Iterator<Item = (usize, Span, Span)> {
        // FIXME: Maybe there is a less slow way of doing this?
        self.endpoints
            .iter()
            .enumerate()
            .flat_map(|(our_index, endpoint)| {
                let (our_id, our_span) = endpoint.portal_id;

                self.endpoints
                    .iter()
                    .enumerate()
                    .skip(our_index + 1)
                    .find_map(|(_, other_endpoints)| {
                        let (other_id, other_span) = other_endpoints.portal_id;

                        if other_id == our_id {
                            Some((other_id, our_span, other_span))
                        } else {
                            None
                        }
                    })
            })
    }

    /// Get the highest protocol endpoint ID
    pub fn highest_id(&self) -> usize {
        self.endpoints
            .iter()
            .map(|endpoint| endpoint.portal_id.0)
            .max()
            .unwrap_or(0)
    }

    /// Check all types to ensure this protocol supports them.
    pub fn check_types(&self) -> Result<(), syn::Error> {
        let endpoint_type = self
            .args
            .as_ref()
            .map(|args| args.protocol_kind)
            .unwrap_or(ProtocolKind::Invalid);

        for endpoint in &self.endpoints {
            for input_arg in &endpoint.input_args {
                input_arg.ty.check_allowed(endpoint_type)?;
            }

            endpoint.output_arg.0.check_allowed(endpoint_type)?;
        }

        Ok(())
    }

    /// Let types gather info about where they are used to figure out the best
    /// way to generate them.
    pub fn type_explore(&mut self) {
        let protocol_defines: Vec<ProtocolDefine> = self
            .endpoints
            .iter()
            .flat_map(|endpoint| endpoint.body.iter().cloned())
            .collect();

        fn search_var_type(var_type: &mut ProtocolVarType, defines: &[ProtocolDefine]) {
            var_type.search_mut(&mut |inner| {
                let maybe_defined = match inner {
                    // We only want to look for types that are currently unknown
                    ProtocolVarType::Unknown(_) => defines
                        .iter()
                        .find(|proto| match proto {
                            ProtocolDefine::DefinedEnum(ref_cell) => {
                                ref_cell.try_borrow().is_ok_and(|ref_cell| {
                                    inner.is_unknown_of(&ref_cell.ident.to_string())
                                })
                            }
                            ProtocolDefine::DefinedStruct(ref_cell) => {
                                ref_cell.try_borrow().is_ok_and(|ref_cell| {
                                    inner.is_unknown_of(&ref_cell.ident.to_string())
                                })
                            }
                        })
                        .cloned(),
                    _ => None,
                };

                if let Some(user_defined) = maybe_defined {
                    *inner = ProtocolVarType::UserDefined {
                        span: inner.span(),
                        to: user_defined,
                    };
                }

                None::<()>
            });
        }

        self.endpoints
            .iter_mut()
            .flat_map(|endpoints| {
                endpoints
                    .input_args
                    .iter_mut()
                    .map(|input| &mut input.ty)
                    .chain([&mut endpoints.output_arg.0].into_iter())
            })
            .for_each(|var_type| search_var_type(var_type, &protocol_defines));

        // Also we need to make sure we include the user defined types too
        self.endpoints
            .iter_mut()
            .flat_map(|endpoint| endpoint.body.iter())
            .for_each(|user_defined| match user_defined {
                ProtocolDefine::DefinedEnum(ref_cell) => {
                    let mut cell = ref_cell.borrow_mut();
                    cell.varients
                        .iter_mut()
                        .for_each(|varient| match &mut varient.fields {
                            ProtocolEnumFields::Unnamed(protocol_var_types) => {
                                protocol_var_types.iter_mut().for_each(|var_type| {
                                    search_var_type(var_type, &protocol_defines)
                                });
                            }
                            ProtocolEnumFields::Named(hash_map) => hash_map
                                .iter_mut()
                                .map(|(_, value)| value)
                                .for_each(|var_type| search_var_type(var_type, &protocol_defines)),
                            _ => (),
                        });
                }
                ProtocolDefine::DefinedStruct(ref_cell) => {
                    let mut cell = ref_cell.borrow_mut();
                    cell.items
                        .iter_mut()
                        .for_each(|varient| search_var_type(&mut varient.ty, &protocol_defines));
                }
            });
    }

    /// Get the name of this portal for modules
    pub fn _get_mod_ident(&self) -> Ident {
        let mut new_str = String::new();
        for old_char in self.trait_ident.to_string().chars() {
            if old_char.is_uppercase() {
                if !new_str.is_empty() {
                    new_str.push('_');
                }
                new_str.push(old_char.to_ascii_lowercase());
            } else {
                new_str.push(old_char);
            }
        }

        Ident::new(&new_str, self.trait_ident.span())
    }

    #[cfg(any(feature = "ipc-client", feature = "ipc-server"))]
    pub fn get_service_name(&self) -> String {
        let mut new_str = String::new();
        for old_char in self.trait_ident.to_string().chars() {
            if old_char.is_uppercase() {
                if !new_str.is_empty() {
                    new_str.push('_');
                }
                new_str.push(old_char.to_ascii_lowercase());
            } else {
                new_str.push(old_char);
            }
        }

        new_str
    }

    pub fn get_input_enum_ident(&self) -> Ident {
        if matches!(self.args.as_ref().unwrap().protocol_kind, ProtocolKind::Ipc) {
            Ident::new(
                &format!("{}ClientRequest", self.trait_ident),
                self.trait_ident.span(),
            )
        } else {
            Ident::new(
                &format!("{}InputArgs", self.trait_ident),
                self.trait_ident.span(),
            )
        }
    }

    pub fn get_output_enum_ident(&self) -> Ident {
        if matches!(self.args.as_ref().unwrap().protocol_kind, ProtocolKind::Ipc) {
            Ident::new(
                &format!("{}ServerRequest", self.trait_ident),
                self.trait_ident.span(),
            )
        } else {
            Ident::new(
                &format!("{}OutputArgs", self.trait_ident),
                self.trait_ident.span(),
            )
        }
    }

    #[cfg(any(feature = "ipc-client", feature = "ipc-server"))]
    pub fn get_info_struct_ident(&self) -> Ident {
        Ident::new(
            &format!("{}Info", self.trait_ident),
            self.trait_ident.span(),
        )
    }

    pub fn trait_client_name(&self) -> Ident {
        format_ident!("{}Client", self.trait_ident)
    }

    #[cfg(any(feature = "ipc-server", feature = "syscall-server"))]
    pub fn trait_server_name(&self) -> Ident {
        format_ident!("{}Server", self.trait_ident)
    }
}
