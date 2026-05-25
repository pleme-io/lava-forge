//! lava-forge — tatara-lisp source generator for lava providers.
//!
//! Consumes the JSON output of `terraform providers schema -json`
//! (same upstream pangea-forge consumes) and emits typed
//! `(deflava-resource ...)` forms — one .tlisp file per resource,
//! one aggregator per provider.
//!
//! ## Pipeline
//!
//! ```text
//! terraform providers schema -json     (upstream)
//!         │
//!         ▼  serde_json::from_str → ProviderSchemasFile
//! typed schema (this crate)
//!         │
//!         ▼  Generator::run
//! <out>/<provider-name>/
//!         ├── aws_vpc.tlisp            one per resource
//!         ├── aws_subnet.tlisp
//!         ├── ...
//!         └── _index.tlisp             provider aggregator
//! ```
//!
//! ## TYPED EMISSION
//!
//! No format!() of tatara-lisp source. Every emitted form is built as
//! a typed [`SExpr`] and rendered via the canonical pretty-printer.

#![allow(clippy::module_name_repetitions)]

use std::path::{Path, PathBuf};
use thiserror::Error;

pub mod schema;
pub mod sexpr;

pub use schema::{
    Attribute, AttributeType, Block, NestedBlock, NestingMode, ProviderSchema,
    ProviderSchemasFile,
};
pub use sexpr::{render, SExpr};

#[derive(Debug, Error)]
pub enum ForgeError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json parse: {0}")]
    Json(#[from] serde_json::Error),
}

/// Generator config + driver. One instance per `forge` run.
pub struct Generator {
    out_dir: PathBuf,
}

impl Generator {
    #[must_use]
    pub fn new(out_dir: impl Into<PathBuf>) -> Self {
        Self {
            out_dir: out_dir.into(),
        }
    }

    /// Emit one .tlisp file per resource + an aggregator per provider.
    /// Returns the list of written paths in deterministic insertion
    /// order (`provider_schemas` is an `IndexMap`).
    pub fn run(&self, schemas: &ProviderSchemasFile) -> Result<Vec<PathBuf>, ForgeError> {
        let mut written = Vec::new();
        for (provider_source, provider) in &schemas.provider_schemas {
            let pname = provider_short_name(provider_source);
            let dir = self.out_dir.join(&pname);
            std::fs::create_dir_all(&dir)?;
            let mut resource_names: Vec<String> = Vec::new();
            for (type_id, block) in &provider.resource_schemas {
                let lisp = render_resource(type_id, block);
                let mut fname = file_safe(type_id);
                fname.push_str(".tlisp");
                let path = dir.join(&fname);
                std::fs::write(&path, lisp)?;
                written.push(path);
                resource_names.push(type_id.clone());
            }
            let aggregator = render_aggregator(&pname, &resource_names);
            let agg_path = dir.join("_index.tlisp");
            std::fs::write(&agg_path, aggregator)?;
            written.push(agg_path);
        }
        Ok(written)
    }
}

/// Drive lava-forge from a file path containing the schema JSON.
pub fn run_from_file(schema_json: &Path, out_dir: &Path) -> Result<Vec<PathBuf>, ForgeError> {
    let text = std::fs::read_to_string(schema_json)?;
    let parsed: ProviderSchemasFile = serde_json::from_str(&text)?;
    let generator = Generator::new(out_dir.to_path_buf());
    generator.run(&parsed)
}

/// "hashicorp/aws" → "aws"; "registry.terraform.io/hashicorp/aws" → "aws".
fn provider_short_name(source: &str) -> String {
    source.rsplit('/').next().unwrap_or(source).to_string()
}

fn file_safe(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Convert `aws_vpc` → `aws-vpc` (kw body without the leading colon).
fn type_id_to_kw_body(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '_' {
            out.push('-');
        } else {
            out.push(c.to_ascii_lowercase());
        }
    }
    out
}

fn render_resource(type_id: &str, block: &Block) -> String {
    let mut items = vec![
        SExpr::sym("deflava-resource"),
        SExpr::Kw(type_id_to_kw_body(type_id)),
    ];
    if let Some(desc) = &block.description {
        items.push(SExpr::Kw("doc".to_string()));
        items.push(SExpr::Str(desc.clone()));
    }
    // :attributes
    let mut attr_forms: Vec<SExpr> = Vec::new();
    let mut attrs: Vec<(&String, &Attribute)> = block.attributes.iter().collect();
    attrs.sort_by(|a, b| a.0.cmp(b.0));
    for (name, attr) in attrs {
        let mut entry = vec![SExpr::Kw(type_id_to_kw_body(name))];
        entry.push(SExpr::Kw("type".to_string()));
        entry.push(attr.r#type.to_sexpr());
        if attr.required {
            entry.push(SExpr::Kw("required".to_string()));
            entry.push(SExpr::Bool(true));
        }
        if attr.optional {
            entry.push(SExpr::Kw("optional".to_string()));
            entry.push(SExpr::Bool(true));
        }
        if attr.computed {
            entry.push(SExpr::Kw("computed".to_string()));
            entry.push(SExpr::Bool(true));
        }
        if attr.sensitive {
            entry.push(SExpr::Kw("sensitive".to_string()));
            entry.push(SExpr::Bool(true));
        }
        if let Some(d) = &attr.description {
            entry.push(SExpr::Kw("doc".to_string()));
            entry.push(SExpr::Str(d.clone()));
        }
        attr_forms.push(SExpr::List(entry));
    }
    if !attr_forms.is_empty() {
        items.push(SExpr::Kw("attributes".to_string()));
        items.push(SExpr::List(attr_forms));
    }
    // :nested-blocks
    let mut nested_forms: Vec<SExpr> = Vec::new();
    let mut nested: Vec<(&String, &NestedBlock)> = block.nested_blocks.iter().collect();
    nested.sort_by(|a, b| a.0.cmp(b.0));
    for (name, nb) in nested {
        let mut entry = vec![SExpr::Kw(type_id_to_kw_body(name))];
        entry.push(SExpr::Kw("mode".to_string()));
        entry.push(SExpr::sym(nesting_kw(nb.nesting)));
        if nb.min_items > 0 {
            entry.push(SExpr::Kw("min-items".to_string()));
            entry.push(SExpr::Int(i64::from(nb.min_items)));
        }
        if let Some(max) = nb.max_items {
            entry.push(SExpr::Kw("max-items".to_string()));
            entry.push(SExpr::Int(i64::from(max)));
        }
        nested_forms.push(SExpr::List(entry));
    }
    if !nested_forms.is_empty() {
        items.push(SExpr::Kw("nested-blocks".to_string()));
        items.push(SExpr::List(nested_forms));
    }

    let mut out = String::from(";; ");
    out.push_str(type_id);
    out.push_str(".tlisp — auto-generated by lava-forge.\n");
    out.push_str(";; Do NOT edit; re-run lava-forge against the upstream provider schema.\n\n");
    out.push_str(&render(&SExpr::List(items)));
    out.push('\n');
    out
}

fn nesting_kw(n: NestingMode) -> &'static str {
    match n {
        NestingMode::Single => "single",
        NestingMode::Group => "group",
        NestingMode::List => "list",
        NestingMode::Set => "set",
        NestingMode::Map => "map",
    }
}

fn render_aggregator(provider: &str, resources: &[String]) -> String {
    let mut sorted = resources.to_vec();
    sorted.sort();
    let items = vec![
        SExpr::sym("deflava-provider"),
        SExpr::Kw(provider.to_string()),
        SExpr::Kw("resources".to_string()),
        SExpr::List(
            sorted
                .iter()
                .map(|n| SExpr::Kw(type_id_to_kw_body(n)))
                .collect(),
        ),
    ];
    let mut out = String::from(";; _index.tlisp — auto-generated by lava-forge.\n");
    out.push_str(";; Catalog of every resource this provider exposes.\n\n");
    out.push_str(&render(&SExpr::List(items)));
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    fn tiny_aws_schema() -> ProviderSchemasFile {
        let mut attrs = IndexMap::new();
        attrs.insert(
            "cidr_block".to_string(),
            Attribute {
                r#type: AttributeType::Primitive("string".to_string()),
                description: Some("CIDR block of the VPC.".to_string()),
                required: true,
                optional: false,
                computed: false,
                sensitive: false,
            },
        );
        attrs.insert(
            "enable_dns_hostnames".to_string(),
            Attribute {
                r#type: AttributeType::Primitive("bool".to_string()),
                description: None,
                required: false,
                optional: true,
                computed: false,
                sensitive: false,
            },
        );
        let mut resource_schemas = IndexMap::new();
        resource_schemas.insert(
            "aws_vpc".to_string(),
            Block {
                version: 0,
                description: Some("Provides a VPC resource.".to_string()),
                attributes: attrs,
                nested_blocks: IndexMap::new(),
            },
        );
        let mut provider_schemas = IndexMap::new();
        provider_schemas.insert(
            "registry.terraform.io/hashicorp/aws".to_string(),
            ProviderSchema {
                provider: Block::default(),
                resource_schemas,
                data_source_schemas: IndexMap::new(),
            },
        );
        ProviderSchemasFile {
            format_version: "1.0".to_string(),
            provider_schemas,
        }
    }

    fn tempdir() -> Tmp {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "lava-forge-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }

    struct Tmp(PathBuf);
    impl Tmp {
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn generator_emits_one_tlisp_per_resource_plus_aggregator() {
        let tmp = tempdir();
        let generator = Generator::new(tmp.path().to_path_buf());
        let written = generator.run(&tiny_aws_schema()).unwrap();
        assert_eq!(written.len(), 2);
        let vpc_path = tmp.path().join("aws").join("aws_vpc.tlisp");
        assert!(
            vpc_path.exists(),
            "aws_vpc.tlisp not written: {:?}",
            written
        );
        let body = std::fs::read_to_string(&vpc_path).unwrap();
        // The pretty-printer wraps long forms across lines; check for
        // typed identifiers + values rather than exact whitespace.
        assert!(body.contains("deflava-resource"));
        assert!(body.contains(":aws-vpc"));
        assert!(body.contains(":cidr-block"));
        assert!(body.contains(":required #t"));
        let agg = tmp.path().join("aws").join("_index.tlisp");
        assert!(agg.exists());
        let agg_body = std::fs::read_to_string(&agg).unwrap();
        assert!(agg_body.contains("deflava-provider"));
        assert!(agg_body.contains(":aws"));
        assert!(agg_body.contains(":aws-vpc"));
    }

    #[test]
    fn type_id_to_kw_body_handles_underscores() {
        assert_eq!(type_id_to_kw_body("aws_vpc"), "aws-vpc");
        assert_eq!(type_id_to_kw_body("cidr_block"), "cidr-block");
    }

    #[test]
    fn provider_short_name_strips_registry_prefix() {
        assert_eq!(
            provider_short_name("registry.terraform.io/hashicorp/aws"),
            "aws"
        );
        assert_eq!(provider_short_name("hashicorp/aws"), "aws");
        assert_eq!(provider_short_name("aws"), "aws");
    }
}
