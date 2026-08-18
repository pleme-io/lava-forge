//! Typed s-expression AST for lava-forge's emitted tatara-lisp source.
//!
//! Per ★★ TYPED EMISSION, the generator never `format!()`'s tatara-lisp
//! syntax. Every emitted form is built as a typed `SExpr` value, then
//! rendered via the canonical pretty-printer in this module.
//!
//! Bugs that would produce malformed s-exprs (mismatched parens, mis-
//! escaped strings, mis-typed values) become compile-time errors.

/// Typed s-expression — one node of a lava-forge-emitted tatara-lisp tree.
#[derive(Debug, Clone, PartialEq)]
pub enum SExpr {
    /// A bare symbol: `deflava-resource`, `defmagma-architecture`.
    Sym(String),
    /// A keyword: `:aws-vpc`, `:cidr-block`. Stored without the leading
    /// colon — renderer adds it.
    Kw(String),
    /// A string literal: `"Provides a VPC resource."`.
    Str(String),
    /// An integer literal.
    Int(i64),
    /// A boolean literal — renders as `#t` / `#f` per Scheme convention.
    Bool(bool),
    /// A parenthesized form: `(deflava-resource :aws-vpc :doc "..." ...)`.
    List(Vec<SExpr>),
}

impl SExpr {
    #[must_use]
    pub fn sym(s: impl Into<String>) -> Self {
        Self::Sym(s.into())
    }
}

/// Render a typed SExpr tree to canonical tatara-lisp source.
///
/// Lists wrap their children in `(...)`; child rendering is whitespace-
/// separated. The renderer chooses line breaks based on list size for
/// readability (single-line for short lists, indented multi-line for
/// long lists with sub-lists).
#[must_use]
pub fn render(expr: &SExpr) -> String {
    let mut out = String::new();
    write_expr(&mut out, expr, 0);
    out
}

fn write_expr(out: &mut String, expr: &SExpr, indent: usize) {
    match expr {
        SExpr::Sym(s) => out.push_str(s),
        SExpr::Kw(k) => {
            out.push(':');
            out.push_str(k);
        }
        SExpr::Str(s) => write_string(out, s),
        SExpr::Int(n) => out.push_str(&n.to_string()),
        SExpr::Bool(true) => out.push_str("#t"),
        SExpr::Bool(false) => out.push_str("#f"),
        SExpr::List(items) => write_list(out, items, indent),
    }
}

fn write_list(out: &mut String, items: &[SExpr], indent: usize) {
    // Empty list.
    if items.is_empty() {
        out.push_str("()");
        return;
    }
    // Short-form: one line if all leaf children + body fits ≤ 80 chars.
    let all_leaves = items
        .iter()
        .all(|e| !matches!(e, SExpr::List(inner) if !inner.is_empty()));
    if all_leaves {
        let mut probe = String::from("(");
        for (i, e) in items.iter().enumerate() {
            if i > 0 {
                probe.push(' ');
            }
            write_expr(&mut probe, e, indent + 2);
        }
        probe.push(')');
        if probe.len() <= 80 {
            out.push_str(&probe);
            return;
        }
    }
    // Long-form: each child on its own line, indented.
    out.push('(');
    for (i, e) in items.iter().enumerate() {
        if i == 0 {
            write_expr(out, e, indent + 2);
        } else {
            out.push('\n');
            for _ in 0..indent + 2 {
                out.push(' ');
            }
            write_expr(out, e, indent + 2);
        }
    }
    out.push(')');
}

/// Render a string with proper escaping. Backslash + quote + newline +
/// tab + control characters get escaped per Scheme string syntax.
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_symbol() {
        assert_eq!(render(&SExpr::sym("deflava-resource")), "deflava-resource");
    }

    #[test]
    fn renders_keyword_with_colon_prefix() {
        assert_eq!(render(&SExpr::Kw("aws-vpc".to_string())), ":aws-vpc");
    }

    #[test]
    fn renders_string_with_escaping() {
        assert_eq!(
            render(&SExpr::Str("Has \"quotes\" and \\backslash".to_string())),
            r#""Has \"quotes\" and \\backslash""#
        );
    }

    #[test]
    fn renders_short_list_inline() {
        let e = SExpr::List(vec![
            SExpr::sym("deflava-resource"),
            SExpr::Kw("aws-vpc".to_string()),
            SExpr::Bool(true),
        ]);
        assert_eq!(render(&e), "(deflava-resource :aws-vpc #t)");
    }

    #[test]
    fn renders_nested_list_multi_line() {
        let inner = SExpr::List(vec![
            SExpr::Kw("cidr-block".to_string()),
            SExpr::Kw("type".to_string()),
            SExpr::sym("string"),
            SExpr::Kw("required".to_string()),
            SExpr::Bool(true),
        ]);
        let outer = SExpr::List(vec![
            SExpr::sym("deflava-resource"),
            SExpr::Kw("aws-vpc".to_string()),
            SExpr::Kw("attributes".to_string()),
            SExpr::List(vec![inner]),
        ]);
        let rendered = render(&outer);
        // Multi-line form (parent has nested list children).
        assert!(rendered.contains("(deflava-resource"));
        assert!(rendered.contains(":cidr-block"));
        assert!(rendered.contains(":required #t"));
    }

    #[test]
    fn empty_list_renders_as_empty_parens() {
        assert_eq!(render(&SExpr::List(vec![])), "()");
    }

    #[test]
    fn bool_renders_scheme_style() {
        assert_eq!(render(&SExpr::Bool(true)), "#t");
        assert_eq!(render(&SExpr::Bool(false)), "#f");
    }
}
