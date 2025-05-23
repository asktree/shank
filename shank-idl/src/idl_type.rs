use std::convert::{TryFrom, TryInto};

use anyhow::{Error, Result};

use serde::{Deserialize, Serialize};
use shank_macro_impl::types::{
    Composite, Primitive, RustType, TypeKind, Value,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum IdlType {
    Array(Box<IdlType>, usize),
    Bool,
    Bytes,
    Defined(String),
    I128,
    I16,
    I32,
    I64,
    I8,
    Option(Box<IdlType>),
    Tuple(Vec<IdlType>),
    PublicKey,
    String,
    U128,
    U16,
    U32,
    U64,
    U8,
    Vec(Box<IdlType>),
    HashMap(Box<IdlType>, Box<IdlType>),
    BTreeMap(Box<IdlType>, Box<IdlType>),
    HashSet(Box<IdlType>),
    BTreeSet(Box<IdlType>),
}

impl TryFrom<RustType> for IdlType {
    type Error = Error;

    fn try_from(rust_ty: RustType) -> Result<Self> {
        let idl_ty = match rust_ty.kind {
            TypeKind::Primitive(prim) => match prim {
                Primitive::U8 => IdlType::U8,
                Primitive::I8 => IdlType::I8,
                Primitive::I16 => IdlType::I16,
                Primitive::U16 => IdlType::U16,
                Primitive::I32 => IdlType::I32,
                Primitive::U32 => IdlType::U32,
                Primitive::I64 => IdlType::I64,
                Primitive::U64 => IdlType::U64,
                Primitive::U128 => IdlType::U128,
                Primitive::I128 => IdlType::I128,
                // ebpf is 64-bit architecture
                Primitive::USize => IdlType::U64,
                Primitive::Bool => IdlType::Bool,
            },
            TypeKind::Value(val) => match val {
                Value::CString | Value::String | Value::Str => IdlType::String,
                Value::Custom(name) => {
                    if name == "Pubkey" {
                        IdlType::PublicKey
                    } else {
                        IdlType::Defined(name)
                    }
                }
            },
            TypeKind::Composite(kind, inners) => match kind {
                Composite::Vec => match inners.first().cloned() {
                    Some(inner) => {
                        let inner_idl: IdlType = inner.try_into()?;
                        if inner_idl == IdlType::U8 {
                            // Vec<u8>
                            IdlType::Bytes
                        } else {
                            IdlType::Vec(Box::new(inner_idl))
                        }
                    }
                    None => {
                        anyhow::bail!("Rust Vec Composite needs inner type")
                    }
                },
                Composite::Array(size) => match inners.first().cloned() {
                    Some(inner) => {
                        let inner_idl: IdlType = inner.try_into()?;
                        IdlType::Array(Box::new(inner_idl), size)
                    }
                    None => {
                        anyhow::bail!("Rust Array Composite needs inner type")
                    }
                },

                Composite::Option => match inners.first().cloned() {
                    Some(inner) => {
                        let inner_idl: IdlType = inner.try_into()?;
                        IdlType::Option(Box::new(inner_idl))
                    }
                    None => {
                        anyhow::bail!("Rust Option Composite needs inner type")
                    }
                },
                Composite::Tuple => {
                    if inners.len() < 2 {
                        anyhow::bail!("Rust Tuple Composite needs at least two inner types");
                    } else {
                        let idl_types: Result<Vec<IdlType>> =
                            inners.into_iter().map(IdlType::try_from).collect();
                        IdlType::Tuple(idl_types?)
                    }
                }
                Composite::HashMap => {
                    match (inners.first().cloned(), inners.get(1).cloned()) {
                        (Some(inner1), Some(inner2)) => {
                            let inner1_idl: IdlType = inner1.try_into()?;
                            let inner2_idl: IdlType = inner2.try_into()?;
                            IdlType::HashMap(
                                Box::new(inner1_idl),
                                Box::new(inner2_idl),
                            )
                        }
                        _ => {
                            anyhow::bail!(
                                "Rust HashMap Composite needs two inner types"
                            )
                        }
                    }
                }
                Composite::BTreeMap => {
                    match (inners.first().cloned(), inners.get(1).cloned()) {
                        (Some(inner1), Some(inner2)) => {
                            let inner1_idl: IdlType = inner1.try_into()?;
                            let inner2_idl: IdlType = inner2.try_into()?;
                            IdlType::BTreeMap(
                                Box::new(inner1_idl),
                                Box::new(inner2_idl),
                            )
                        }
                        _ => {
                            anyhow::bail!(
                                "Rust BTreeMap Composite needs two inner types"
                            )
                        }
                    }
                }
                Composite::HashSet => match inners.first().cloned() {
                    Some(inner) => {
                        let inner_idl: IdlType = inner.try_into()?;
                        IdlType::HashSet(Box::new(inner_idl))
                    }
                    _ => {
                        anyhow::bail!(
                            "Rust HashSet Composite needs one inner type"
                        )
                    }
                },
                Composite::BTreeSet => match inners.first().cloned() {
                    Some(inner) => {
                        let inner_idl: IdlType = inner.try_into()?;
                        IdlType::BTreeSet(Box::new(inner_idl))
                    }
                    _ => {
                        anyhow::bail!(
                            "Rust BTreeSet Composite needs one inner type"
                        )
                    }
                },
                Composite::Decimal(_precision) => {
                    // Decimal<const P: u8, T> where T: Copy + PartialEq + Eq + Debug
                    // We only care about the inner type (second generic parameter)
                    if inners.len() == 1 {
                        let inner_type = inners[0].clone();
                        inner_type.try_into()?
                    } else {
                        anyhow::bail!(
                            "Decimal composite needs one type parameter, got {}",
                            inners.len()
                        )
                    }
                }
                Composite::Custom(_) => {
                    anyhow::bail!(
                        "Rust Custom Composite IDL type not yet supported"
                    )
                }
            },
            TypeKind::Unit => anyhow::bail!("IDL types cannot be Unit ()"),
            TypeKind::Unknown => {
                anyhow::bail!("Can only convert known types to IDL type. You are using forked Shank. Type: {:?}", rust_ty)
            }
        };
        Ok(idl_ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idl_from_rust_type_primivives() {
        for (rust_prim, idl_expected) in [
            (Primitive::U8, IdlType::U8),
            (Primitive::U16, IdlType::U16),
            (Primitive::I128, IdlType::I128),
            (Primitive::Bool, IdlType::Bool),
            (Primitive::USize, IdlType::U64),
        ] {
            let rust_ty = RustType::owned_primitive("prim", rust_prim);
            let idl_ty: IdlType =
                rust_ty.try_into().expect("Failed to convert");
            assert_eq!(idl_ty, idl_expected);
        }
    }
    #[test]
    fn idl_from_rust_type_string() {
        let rust_ty = RustType::owned_string("s");
        let idl_ty: IdlType = rust_ty.try_into().expect("Failed to convert");
        assert_eq!(idl_ty, IdlType::String);
    }

    #[test]
    fn idl_from_rust_type_publickey() {
        let rust_ty = RustType::owned_custom_value("pk", "Pubkey");
        let idl_ty: IdlType = rust_ty.try_into().expect("Failed to convert");
        assert_eq!(idl_ty, IdlType::PublicKey);
    }

    #[test]
    fn idl_from_rust_type_custom() {
        let rust_ty = RustType::owned_custom_value("custom", "SomeUserStruct");
        let idl_ty: IdlType = rust_ty.try_into().expect("Failed to convert");
        assert_eq!(idl_ty, IdlType::Defined("SomeUserStruct".to_string()));
    }

    #[test]
    fn idl_from_rust_type_vec() {
        let rust_ty = RustType::owned_vec_primitive("vec_u16", Primitive::U16);
        let idl_ty: IdlType = rust_ty.try_into().expect("Failed to convert");
        assert_eq!(idl_ty, IdlType::Vec(Box::new(IdlType::U16)));
    }

    #[test]
    fn idl_from_rust_type_vec_u8() {
        let rust_ty = RustType::owned_vec_primitive("bytes", Primitive::U8);
        let idl_ty: IdlType = rust_ty.try_into().expect("Failed to convert");
        assert_eq!(idl_ty, IdlType::Bytes);
    }

    #[test]
    fn idl_from_rust_type_array_u8() {
        let rust_ty =
            RustType::owned_array_primitive("bytes", Primitive::U8, 5);
        let idl_ty: IdlType = rust_ty.try_into().expect("Failed to convert");
        assert_eq!(idl_ty, IdlType::Array(Box::new(IdlType::U8), 5));
    }

    #[test]
    fn idl_from_rust_type_option_i64() {
        let rust_ty = RustType::owned_option_primitive("bytes", Primitive::I64);
        let idl_ty: IdlType = rust_ty.try_into().expect("Failed to convert");
        assert_eq!(idl_ty, IdlType::Option(Box::new(IdlType::I64)));
    }
}
