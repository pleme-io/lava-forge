//! Typed Terraform provider schema — the wire shape
//! `terraform providers schema -json` emits.
//!
//! Mirrors tfplugin5/6 protobuf field-for-field so values round-trip
//! byte-equal between lava-forge, magma-types, and the providers
//! themselves. Same shape pangea-forge's Ruby code consumes — we pull
//! from the same upstream.

#![allow(clippy::module_name_repetitions)]

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Top-level wire shape `terraform providers schema -json` emits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSchemasFile {
    #[serde(default)]
    pub format_version: String,
    #[serde(default)]
    pub provider_schemas: IndexMap<String, ProviderSchema>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderSchema {
    #[serde(default)]
    pub provider: Block,
    #[serde(default)]
    pub resource_schemas: IndexMap<String, Block>,
    #[serde(default)]
    pub data_source_schemas: IndexMap<String, Block>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Block {
    #[serde(default)]
    pub version: u64,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub attributes: IndexMap<String, Attribute>,
    #[serde(default, rename = "block_types")]
    pub nested_blocks: IndexMap<String, NestedBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    /// The attribute's typed type. Terraform schema sometimes emits a
    /// bare string ("string") and sometimes a nested-list shape (e.g.
    /// `["list", "string"]`). [`AttributeType`] handles both.
    #[serde(default = "AttributeType::default_string")]
    pub r#type: AttributeType,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub computed: bool,
    #[serde(default)]
    pub sensitive: bool,
}

/// Terraform attribute type. Either a primitive name as a bare string
/// ("string", "number", "bool") OR a nested-list shape for collections
/// ("list" + element type, "map" + value type, "object" + body, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeType {
    Primitive(String),
    /// Complex type — first element is the constructor name ("list",
    /// "map", "object", "set", "tuple"); rest are arguments.
    Complex(Vec<serde_json::Value>),
}

impl AttributeType {
    pub(crate) fn default_string() -> Self {
        Self::Primitive("string".to_string())
    }

    /// Render the typed type to a typed [`crate::SExpr`] so the
    /// emitter can splice it into `(deflava-resource ...)` forms
    /// without format!()-ing lisp syntax.
    #[must_use]
    pub fn to_sexpr(&self) -> crate::SExpr {
        match self {
            // Bare primitive → bare symbol (`string`, `number`, `bool`).
            Self::Primitive(s) => crate::SExpr::sym(s.clone()),
            // Complex types render as a list. JSON [list, string]
            // becomes the lisp form `(list string)`.
            Self::Complex(items) => {
                let mut out = Vec::new();
                for v in items {
                    out.push(json_to_sexpr(v));
                }
                crate::SExpr::List(out)
            }
        }
    }
}

fn json_to_sexpr(v: &serde_json::Value) -> crate::SExpr {
    match v {
        serde_json::Value::String(s) => crate::SExpr::sym(s.clone()),
        serde_json::Value::Bool(b) => crate::SExpr::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                crate::SExpr::Int(i)
            } else {
                crate::SExpr::Str(n.to_string())
            }
        }
        serde_json::Value::Array(arr) => {
            crate::SExpr::List(arr.iter().map(json_to_sexpr).collect())
        }
        serde_json::Value::Object(obj) => {
            // Render object as alternating (:k v ...).
            let mut entries = Vec::new();
            for (k, val) in obj {
                entries.push(crate::SExpr::Kw(k.clone()));
                entries.push(json_to_sexpr(val));
            }
            crate::SExpr::List(entries)
        }
        serde_json::Value::Null => crate::SExpr::sym("nil".to_string()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestedBlock {
    pub nesting: NestingMode,
    #[serde(default)]
    pub block: Box<Block>,
    #[serde(default)]
    pub min_items: u32,
    #[serde(default)]
    pub max_items: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NestingMode {
    Single,
    Group,
    List,
    Set,
    Map,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_attribute_type_renders_as_bare_symbol() {
        let t = AttributeType::Primitive("string".to_string());
        let s = t.to_sexpr();
        assert_eq!(crate::render(&s), "string");
    }

    #[test]
    fn complex_attribute_type_renders_as_list_form() {
        let t = AttributeType::Complex(vec![
            serde_json::Value::String("list".to_string()),
            serde_json::Value::String("string".to_string()),
        ]);
        let s = t.to_sexpr();
        assert_eq!(crate::render(&s), "(list string)");
    }

    #[test]
    fn deserializes_provider_schemas_file_minimal() {
        let json = r#"{
            "format_version": "1.0",
            "provider_schemas": {
                "registry.terraform.io/hashicorp/aws": {
                    "provider": { "version": 0, "attributes": {}, "block_types": {} },
                    "resource_schemas": {
                        "aws_vpc": {
                            "version": 0,
                            "description": "Provides a VPC resource.",
                            "attributes": {
                                "cidr_block": {
                                    "type": "string",
                                    "required": true,
                                    "description": "CIDR block."
                                }
                            },
                            "block_types": {}
                        }
                    },
                    "data_source_schemas": {}
                }
            }
        }"#;
        let parsed: ProviderSchemasFile = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.format_version, "1.0");
        let aws = parsed.provider_schemas.get("registry.terraform.io/hashicorp/aws").unwrap();
        let vpc = aws.resource_schemas.get("aws_vpc").unwrap();
        assert_eq!(vpc.description.as_deref(), Some("Provides a VPC resource."));
        let cidr = vpc.attributes.get("cidr_block").unwrap();
        assert!(cidr.required);
    }
}
