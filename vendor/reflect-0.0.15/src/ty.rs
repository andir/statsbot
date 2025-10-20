use crate::{
    generics, Data, Function, GenericConstraint, GenericParam, Generics, Ident, Lifetime, Path,
    Print, Signature, TypeParamBound,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use ref_cast::RefCast;
use syn::TypePath;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Type(pub(crate) TypeNode);

#[derive(Debug, Clone)]
pub(crate) enum TypeNode {
    Infer,
    Tuple(Vec<Type>),
    PrimitiveStr,
    Reference {
        lifetime: Option<Lifetime>,
        inner: Box<TypeNode>,
    },
    ReferenceMut {
        lifetime: Option<Lifetime>,
        inner: Box<TypeNode>,
    },
    Dereference(Box<TypeNode>),
    TraitObject(Vec<TypeParamBound>),
    DataStructure {
        name: Ident,
        generics: Generics,
        data: Data<Type>,
    },
    Path(Path),
}

impl Type {
    pub fn unit() -> Self {
        Type(TypeNode::Tuple(Vec::new()))
    }

    pub fn tuple(types: &[Self]) -> Self {
        Type(TypeNode::Tuple(Vec::from(types)))
    }

    pub fn primitive_str() -> Self {
        Type(TypeNode::PrimitiveStr)
    }

    pub fn reference(&self) -> Self {
        Type(TypeNode::Reference {
            lifetime: None,
            inner: Box::new(self.0.clone()),
        })
    }

    pub fn reference_mut(&self) -> Self {
        Type(TypeNode::ReferenceMut {
            lifetime: None,
            inner: Box::new(self.0.clone()),
        })
    }

    pub fn dereference(&self) -> Self {
        match &self.0 {
            TypeNode::Reference { inner, .. } => Type((**inner).clone()),
            TypeNode::ReferenceMut { inner, .. } => Type((**inner).clone()),
            other => Type(TypeNode::Dereference(Box::new(other.clone()))),
        }
    }

    pub fn get_function(&self, name: &str, sig: Signature) -> Function {
        Function {
            parent: Some(self.clone()),
            name: name.to_owned(),
            sig,
        }
    }

    pub fn data(&self) -> Data<Self> {
        match &self.0 {
            TypeNode::DataStructure { data, .. } => data.clone().map(|field| field.element),
            TypeNode::Reference { lifetime, inner } => {
                Type((**inner).clone()).data().map(|field| {
                    Type(TypeNode::Reference {
                        lifetime: lifetime.clone(),
                        inner: Box::new(field.element.0),
                    })
                })
            }
            TypeNode::ReferenceMut { lifetime, inner } => {
                Type((**inner).clone()).data().map(|field| {
                    Type(TypeNode::ReferenceMut {
                        lifetime: lifetime.clone(),
                        inner: Box::new(field.element.0),
                    })
                })
            }
            _ => panic!("Type::data"),
        }
    }

    /// Returns a Type from a Tuple
    pub fn get_tuple_type(&self, index: usize) -> Self {
        match &self.0 {
            TypeNode::Tuple(types) => types[index].clone(),
            _ => panic!("Type::get_tuple_type: Not a Tuple"),
        }
    }

    pub(crate) fn syn_to_type(ty: syn::Type) -> Self {
        match ty {
            syn::Type::Path(TypePath {
                // FIXME: add qself to Path
                qself: None,
                path,
            }) => Type(TypeNode::Path(Path::syn_to_path(path))),

            syn::Type::Reference(reference) => {
                let inner = Box::new(Type::syn_to_type(*reference.elem).0);
                let lifetime = reference.lifetime.map(|lifetime| Lifetime {
                    ident: Ident::from(lifetime.ident),
                });
                if reference.mutability.is_some() {
                    Type(TypeNode::ReferenceMut { lifetime, inner })
                } else {
                    Type(TypeNode::Reference { lifetime, inner })
                }
            }
            // FIXME: TraitObject
            syn::Type::TraitObject(type_trait_object) => Type(TypeNode::TraitObject(
                generics::syn_to_type_param_bounds(type_trait_object.bounds),
            )),

            syn::Type::Tuple(type_tuple) => {
                if type_tuple.elems.is_empty() {
                    Type::unit()
                } else if type_tuple.elems.len() == 1 && !type_tuple.elems.trailing_punct() {
                    // It is not a tuple. The parentheses were just used to
                    // disambiguate the type.
                    Self::syn_to_type(type_tuple.elems.into_iter().next().unwrap())
                } else {
                    Type(TypeNode::Tuple(
                        type_tuple
                            .elems
                            .into_iter()
                            .map(Self::syn_to_type)
                            .collect(),
                    ))
                }
            }
            _ => unimplemented!("Type::syn_to_type"),
        }
    }

    pub(crate) fn name_and_generics(
        &self,
    ) -> (TokenStream, Vec<GenericParam>, Vec<GenericConstraint>) {
        self.0.name_and_generics()
    }
}

impl TypeNode {
    pub(crate) fn get_name(&self) -> String {
        match self {
            // FIXME: Add more TypeNode branches
            TypeNode::Tuple(types) => {
                let types = types.iter().map(Print::ref_cast);
                quote!((#(#types),*)).to_string()
            }
            TypeNode::PrimitiveStr => String::from("str"),
            TypeNode::DataStructure { name, .. } => name.to_string(),
            TypeNode::Reference { inner, .. } => (**inner).get_name(),
            TypeNode::ReferenceMut { inner, .. } => (**inner).get_name(),
            TypeNode::Path(path) => {
                let mut tokens = TokenStream::new();
                Print::ref_cast(path).to_tokens(&mut tokens);
                tokens.to_string()
            }
            _ => panic!("Type::get_name"),
        }
    }

    pub(crate) fn name_and_generics(
        &self,
    ) -> (TokenStream, Vec<GenericParam>, Vec<GenericConstraint>) {
        use super::TypeNode::*;
        match self {
            Infer => panic!("Type::name_and_generics: Infer"),

            Tuple(types) => {
                let types = types.iter().map(Print::ref_cast);
                (quote!((#(#types),*)), Vec::new(), Vec::new())
            }

            PrimitiveStr => (quote!(str), Vec::new(), Vec::new()),

            Reference { lifetime, inner } => {
                let lifetime = lifetime.as_ref().map(Print::ref_cast);
                let (name, params, constraints) = inner.name_and_generics();
                (quote!(& #lifetime #name), params, constraints)
            }

            ReferenceMut { lifetime, inner } => {
                let lifetime = lifetime.as_ref().map(Print::ref_cast);
                let (name, params, constraints) = inner.name_and_generics();
                (quote!(&mut #lifetime #name), params, constraints)
            }

            Dereference(_dereference) => panic!("Type::name_and_generics: Dereference"),

            TraitObject(type_param_bound) => {
                if type_param_bound.len() != 1 {
                    panic!("Type::name_and_generics: TraitObject has more than one bound");
                }
                let type_param_bound = Print::ref_cast(&type_param_bound[0]);
                (quote!(dyn #type_param_bound), Vec::new(), Vec::new())
            }

            DataStructure {
                name,
                generics:
                    Generics {
                        params,
                        constraints,
                    },
                ..
            } => (quote!(#name), params.clone(), constraints.clone()),

            Path(path) => {
                // FIXME: separate generics from path if possible
                let path = Print::ref_cast(path);
                (quote!(path), Vec::new(), Vec::new())
            }
        }
    }
}
