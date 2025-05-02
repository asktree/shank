use heck::MixedCase;
use std::convert::{TryFrom, TryInto};

use serde::{Deserialize, Serialize};
use shank_macro_impl::parsed_struct::StructField;
use shank_macro_impl::types::{Composite, TypeKind};

use crate::idl_type::IdlType;
use anyhow::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdlField {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<Vec<String>>,
}

impl TryFrom<StructField> for IdlField {
    type Error = Error;

    fn try_from(field: StructField) -> Result<Self> {
        let docs = auto_docs(&field.rust_type);

        let ty: IdlType = if let Some(override_type) = field.type_override() {
            override_type.clone().try_into()?
        } else {
            field.rust_type.clone().try_into()?
        };

        let attrs = field
            .attrs
            .iter()
            .map(Into::<String>::into)
            .collect::<Vec<String>>();
        let attrs = if attrs.is_empty() { None } else { Some(attrs) };

        Ok(Self {
            name: field.ident.to_string().to_mixed_case(),
            ty,
            attrs,
            docs,
        })
    }
}

pub fn auto_docs(
    rust_ty: &shank_macro_impl::types::RustType,
) -> Option<Vec<String>> {
    match &rust_ty.kind {
        TypeKind::Composite(Composite::Decimal(p), _) => {
            Some(vec![format!("@amount decimals={}", p)])
        }
        _ => None,
    }
}
