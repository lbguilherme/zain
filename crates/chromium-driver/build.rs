//! Codegen for the typed CDP bindings under `src/cdp/`.
//!
//! Reads the per-domain `*.json` (split from the DevTools protocol) and emits
//! the Rust bindings following `src/cdp/CONVENTIONS.md`. Writes the result back
//! into `src/cdp/` (build.rs → src, by design) but only:
//!   - for the domains listed in `MANIFEST` (opt-in, curated coverage), and
//!   - when the generated text actually differs (write-if-changed), so a clean
//!     tree stays clean and there is no rebuild loop.
//!
//! `rerun-if-changed` is emitted for build.rs and every input JSON, so this
//! only runs when an input changes.

use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

/// Domains to generate. Opt-in so coverage stays curated (the crate uses ~10
/// of 56 domains). Per-domain command subsetting can be added here later.
const MANIFEST: &[&str] = &["io", "input"];

const HEADER_WIDTH: usize = 80;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let cdp = Path::new(&manifest_dir).join("src/cdp");
    println!("cargo::rerun-if-changed=build.rs");

    for domain in MANIFEST {
        let json_path = cdp.join(format!("{domain}.json"));
        println!("cargo::rerun-if-changed=src/cdp/{domain}.json");
        let raw = std::fs::read_to_string(&json_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", json_path.display()));
        let json: Value = serde_json::from_str(&raw).expect("parse domain json");

        let code = Cx::new(&json).generate();

        let out = cdp.join(format!("{domain}.rs"));
        write_if_changed(&out, &code);
    }
}

fn write_if_changed(path: &Path, content: &str) {
    if std::fs::read_to_string(path).ok().as_deref() == Some(content) {
        return;
    }
    std::fs::write(path, content).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

// ── Generator ────────────────────────────────────────────────────────────────

/// How a struct's fields are (de)serialized, which drives the derives and the
/// per-field serde attribute for optional fields.
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// Command parameters: `Serialize`; optional → `skip_serializing_if`.
    Param,
    /// Command return: `Deserialize`; optional → `#[serde(default)]`.
    Return,
    /// Event payload: `Deserialize` (+ `Clone`); optional → `#[serde(default)]`.
    Event,
    /// A named protocol type (object): both directions.
    Object,
}

/// Generation state for one domain, accumulating imports, hoisted inline enums,
/// which serde traits are needed, and which local types derive `Default` (so a
/// struct embedding them can derive `Default` too).
struct Cx<'a> {
    json: &'a Value,
    domain: String,
    uses: BTreeSet<String>,
    inline_enums: Vec<String>,
    inline_seen: BTreeSet<String>,
    default_types: BTreeSet<String>,
    needs_ser: bool,
    needs_de: bool,
}

impl<'a> Cx<'a> {
    fn new(json: &'a Value) -> Self {
        let domain = json["domain"].as_str().expect("domain name").to_string();
        let mut uses = BTreeSet::new();
        uses.insert("crate::error::Result".to_string());
        uses.insert("crate::session::CdpSession".to_string());
        Self {
            json,
            domain,
            uses,
            inline_enums: Vec::new(),
            inline_seen: BTreeSet::new(),
            default_types: BTreeSet::new(),
            needs_ser: false,
            needs_de: false,
        }
    }

    fn generate(mut self) -> String {
        let domain = self.domain.clone();
        let pascal = pascal_domain(&domain);
        let prefix = domain.to_lowercase();
        let trait_name = format!("{pascal}Commands");
        let empty = Vec::new();

        // ── Types ── (processed first so later structs know their `Default`-ness)
        let type_items: Vec<String> = self
            .json
            .get("types")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
            .iter()
            .map(|t| self.gen_type(t))
            .collect();

        // ── Commands → params / returns / trait / impl ──
        let mut param_items = Vec::new();
        let mut return_items = Vec::new();
        let mut trait_methods = Vec::new();
        let mut internal_params = Vec::new();
        let mut impl_methods = Vec::new();

        let commands = self.json.get("commands").and_then(Value::as_array).cloned();
        for cmd in commands.unwrap_or_default() {
            if cmd.get("deprecated").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            let name = cmd["name"].as_str().unwrap();
            let cmd_pascal = pascal_case(name);
            let method = format!("{prefix}_{}", snake_case(name));
            let cdp_name = format!("{domain}.{name}");

            let params = cmd["parameters"].as_array().cloned().unwrap_or_default();
            let returns = cmd["returns"].as_array().cloned().unwrap_or_default();
            let required: Vec<&Value> = params
                .iter()
                .filter(|p| p.get("optional").and_then(Value::as_bool) != Some(true))
                .collect();
            let has_optional = params
                .iter()
                .any(|p| p.get("optional").and_then(Value::as_bool) == Some(true));
            let any_inline_enum = params.iter().any(is_inline_enum);

            let return_ty = if returns.is_empty() {
                None
            } else {
                let rname = format!("{cmd_pascal}Return");
                let item = self.gen_struct(
                    &rname,
                    &format!("/// Return type for [`{trait_name}::{method}`].\n"),
                    &returns,
                    Mode::Return,
                    &cmd_pascal,
                );
                return_items.push(item);
                Some(rname)
            };
            let ret_sig = match &return_ty {
                Some(t) => format!("Result<{t}>"),
                None => "Result<()>".into(),
            };

            let doc = doc_block(
                cmd["description"].as_str(),
                4,
                &format!("CDP: `{cdp_name}`"),
            );

            // Direct-args heuristic: 1-2 required params, none optional, no
            // inline enum (those need a named type, so a Params struct).
            let direct = !params.is_empty()
                && !has_optional
                && !any_inline_enum
                && (1..=2).contains(&required.len());

            if params.is_empty() {
                trait_methods.push(format!("{doc}    async fn {method}(&self) -> {ret_sig};"));
                let call_fn = call_fn(return_ty.is_some());
                impl_methods.push(format!(
                    "    async fn {method}(&self) -> {ret_sig} {{\n        self.{call_fn}(\"{cdp_name}\", &serde_json::json!({{}})).await\n    }}"
                ));
            } else if direct {
                self.needs_ser = true;
                let (sig_args, fields, ctor, lifetime) = self.direct_args(&required);
                trait_methods.push(format!(
                    "{doc}    async fn {method}(&self, {sig_args}) -> {ret_sig};"
                ));
                let struct_name = format!("{cmd_pascal}InternalParams");
                internal_params.push(format!(
                    "#[derive(Serialize)]\n#[serde(rename_all = \"camelCase\")]\nstruct {struct_name}{lifetime} {{\n{fields}\n}}"
                ));
                let call_fn = call_fn(return_ty.is_some());
                impl_methods.push(format!(
                    "    async fn {method}(&self, {sig_args}) -> {ret_sig} {{\n        let params = {struct_name} {{ {ctor} }};\n        self.{call_fn}(\"{cdp_name}\", &params).await\n    }}"
                ));
            } else {
                self.needs_ser = true;
                let pname = format!("{cmd_pascal}Params");
                let item = self.gen_struct(
                    &pname,
                    &format!("/// Parameters for [`{trait_name}::{method}`].\n"),
                    &params,
                    Mode::Param,
                    &cmd_pascal,
                );
                param_items.push(item);
                trait_methods.push(format!(
                    "{doc}    async fn {method}(&self, params: &{pname}) -> {ret_sig};"
                ));
                let call_fn = call_fn(return_ty.is_some());
                impl_methods.push(format!(
                    "    async fn {method}(&self, params: &{pname}) -> {ret_sig} {{\n        self.{call_fn}(\"{cdp_name}\", params).await\n    }}"
                ));
            }
        }

        // ── Events ──
        let mut event_items = Vec::new();
        let events = self.json.get("events").and_then(Value::as_array).cloned();
        for ev in events.unwrap_or_default() {
            if ev.get("deprecated").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            let ev_pascal = pascal_case(ev["name"].as_str().unwrap());
            let fields = ev["parameters"].as_array().cloned().unwrap_or_default();
            let item = self.gen_struct(
                &format!("{ev_pascal}Event"),
                &doc_lines(ev["description"].as_str(), 0),
                &fields,
                Mode::Event,
                &ev_pascal,
            );
            event_items.push(item);
        }

        // ── Imports ──
        let serde_use = match (self.needs_ser, self.needs_de) {
            (true, true) => "use serde::{Deserialize, Serialize};",
            (true, false) => "use serde::Serialize;",
            (false, true) => "use serde::Deserialize;",
            (false, false) => "",
        };
        let crate_block = self
            .uses
            .iter()
            .map(|u| format!("use {u};"))
            .collect::<Vec<_>>()
            .join("\n");
        let imports = if serde_use.is_empty() {
            crate_block
        } else {
            format!("{serde_use}\n\n{crate_block}")
        };

        // ── Trait doc ──
        let mut trait_doc = format!("/// `{domain}` domain CDP methods.\n");
        if let Some(d) = self.json["description"].as_str() {
            trait_doc.push_str("///\n");
            for line in normalize_desc(d).split('\n') {
                trait_doc.push_str(&format!("/// {line}\n"));
            }
        }
        trait_doc.push_str("///\n");
        trait_doc.push_str(&format!(
            "/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/{domain}/>\n"
        ));
        let trait_block = format!(
            "{trait_doc}pub trait {trait_name} {{\n{}\n}}",
            trait_methods.join("\n\n")
        );

        let impl_block = format!(
            "impl {trait_name} for CdpSession {{\n{}\n}}",
            impl_methods.join("\n\n")
        );
        let mut impl_items = internal_params;
        impl_items.push(impl_block);

        // ── Assemble ──
        let mut sections = vec![imports, section("Types", &type_items)];
        if !self.inline_enums.is_empty() {
            sections.push(section("Inline enums", &self.inline_enums));
        }
        if !param_items.is_empty() {
            sections.push(section("Param types", &param_items));
        }
        if !return_items.is_empty() {
            sections.push(section("Return types", &return_items));
        }
        if !event_items.is_empty() {
            sections.push(section("Events", &event_items));
        }
        sections.push(format!("{}\n\n{trait_block}", header("Domain trait")));
        sections.push(format!("{}\n\n{}", header("Impl"), impl_items.join("\n\n")));

        let mut out = sections.join("\n\n");
        out.push('\n');
        out
    }

    /// A top-level protocol type: newtype (string/integer/number), string enum,
    /// or object struct.
    fn gen_type(&mut self, t: &Value) -> String {
        let id = t["id"].as_str().unwrap();
        let doc = doc_lines(t["description"].as_str(), 0);
        match t["type"].as_str() {
            Some("string") if t.get("enum").is_some() => {
                self.needs_ser = true;
                self.needs_de = true;
                self.default_types.insert(id.to_string());
                render_enum(id, &doc, t["enum"].as_array().unwrap())
            }
            Some("string") => {
                self.needs_ser = true;
                self.needs_de = true;
                format!(
                    "{doc}#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]\npub struct {id}(pub String);"
                )
            }
            Some("integer") => {
                self.needs_ser = true;
                self.needs_de = true;
                format!(
                    "{doc}#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\npub struct {id}(pub i64);"
                )
            }
            Some("number") => {
                self.needs_ser = true;
                self.needs_de = true;
                format!(
                    "{doc}#[derive(Debug, Clone, Copy, Serialize, Deserialize)]\npub struct {id}(pub f64);"
                )
            }
            Some("object") => {
                let props = t["properties"].as_array().cloned().unwrap_or_default();
                self.gen_struct(id, &doc, &props, Mode::Object, id)
            }
            other => panic!("type kind {other:?} not yet supported (type {id})"),
        }
    }

    /// Renders a struct: `leading` (formatted doc lines), derives per `mode`
    /// (adding `Default` when every field can default), and the fields.
    fn gen_struct(
        &mut self,
        name: &str,
        leading: &str,
        fields: &[Value],
        mode: Mode,
        ctx: &str,
    ) -> String {
        if matches!(mode, Mode::Param | Mode::Object) {
            self.needs_ser = true;
        }
        if matches!(mode, Mode::Return | Mode::Event | Mode::Object) {
            self.needs_de = true;
        }

        let mut body = String::new();
        let mut all_default = !fields.is_empty();
        for f in fields {
            let (rendered, ty, optional) = self.render_field(f, mode, ctx);
            if !optional && !self.default_able(&ty) {
                all_default = false;
            }
            body.push_str(&rendered);
        }

        let want_default = all_default && matches!(mode, Mode::Param | Mode::Object);
        if want_default && mode == Mode::Object {
            self.default_types.insert(name.to_string());
        }
        let derive = match (mode, want_default) {
            (Mode::Param, true) => "#[derive(Debug, Clone, Default, Serialize)]",
            (Mode::Param, false) => "#[derive(Debug, Clone, Serialize)]",
            (Mode::Return, _) => "#[derive(Debug, Deserialize)]",
            (Mode::Event, _) => "#[derive(Debug, Clone, Deserialize)]",
            (Mode::Object, true) => "#[derive(Debug, Clone, Default, Serialize, Deserialize)]",
            (Mode::Object, false) => "#[derive(Debug, Clone, Serialize, Deserialize)]",
        };

        format!(
            "{leading}{derive}\n#[serde(rename_all = \"camelCase\")]\npub struct {name} {{\n{body}}}"
        )
    }

    /// One struct field. Returns (rendered, type-without-`Option`, optional).
    /// Inline enums (a `string` field with `enum`) are hoisted to `{Ctx}{Field}`.
    fn render_field(&mut self, f: &Value, mode: Mode, ctx: &str) -> (String, String, bool) {
        let wire = f["name"].as_str().unwrap();
        let ident = field_ident(wire);
        let optional = f.get("optional").and_then(Value::as_bool) == Some(true);

        let ty = if is_inline_enum(f) {
            let ename = format!("{ctx}{}", pascal_case(wire));
            if self.inline_seen.insert(ename.clone()) {
                self.needs_ser = true;
                self.needs_de = true;
                self.default_types.insert(ename.clone());
                let e = render_enum(
                    &ename,
                    &doc_lines(f["description"].as_str(), 0),
                    f["enum"].as_array().unwrap(),
                );
                self.inline_enums.push(e);
            }
            ename
        } else {
            self.map_type(f)
        };

        let mut out = doc_lines(f["description"].as_str(), 4);
        let logical = ident.strip_prefix("r#").unwrap_or(&ident);
        if lower_camel(logical) != wire {
            out.push_str(&format!("    #[serde(rename = \"{wire}\")]\n"));
        }
        let decl_ty = if optional {
            match mode {
                Mode::Param => {
                    out.push_str("    #[serde(skip_serializing_if = \"Option::is_none\")]\n")
                }
                _ => out.push_str("    #[serde(default)]\n"),
            }
            format!("Option<{ty}>")
        } else {
            ty.clone()
        };
        out.push_str(&format!("    pub {ident}: {decl_ty},\n"));
        (out, ty, optional)
    }

    /// Builds the pieces for a direct-args command: the signature args, the
    /// `InternalParams` fields, the constructor field list, and the `<'a>`
    /// lifetime (empty if every arg is passed by value).
    fn direct_args(&mut self, required: &[&Value]) -> (String, String, String, String) {
        let mut sig = Vec::new();
        let mut fields = Vec::new();
        let mut ctor = Vec::new();
        let mut needs_lifetime = false;
        for p in required {
            let ident = field_ident(p["name"].as_str().unwrap());
            let mapped = self.map_type(p);
            // Copy primitives pass by value; everything else (String→str,
            // newtypes, $refs) is borrowed.
            let (ty, by_ref) = match mapped.as_str() {
                "String" => ("str".to_string(), true),
                "bool" | "i64" | "f64" => (mapped, false),
                other => (other.to_string(), true),
            };
            if by_ref {
                needs_lifetime = true;
                sig.push(format!("{ident}: &{ty}"));
                fields.push(format!("    {ident}: &'a {ty},"));
            } else {
                sig.push(format!("{ident}: {ty}"));
                fields.push(format!("    {ident}: {ty},"));
            }
            ctor.push(ident);
        }
        let lifetime = if needs_lifetime { "<'a>" } else { "" };
        (
            sig.join(", "),
            fields.join("\n"),
            ctor.join(", "),
            lifetime.to_string(),
        )
    }

    /// Maps a JSON node to a Rust type, recording cross-domain imports.
    fn map_type(&mut self, node: &Value) -> String {
        if let Some(r) = node.get("$ref").and_then(Value::as_str) {
            if let Some((dom, ty)) = r.split_once('.') {
                // Cross-domain ref → the type's owning `cdp` module. Shared IDs
                // (TargetId, FrameId, …) are re-exported by `crate::types` from
                // their owning module, so there is no special case here.
                self.uses
                    .insert(format!("crate::cdp::{}::{ty}", dom.to_lowercase()));
                return ty.to_string();
            }
            return r.to_string();
        }
        match node["type"].as_str() {
            Some("string") => "String".into(),
            Some("integer") => "i64".into(),
            Some("number") => "f64".into(),
            Some("boolean") => "bool".into(),
            Some("array") => format!("Vec<{}>", self.map_type(&node["items"])),
            Some("object") | Some("any") | None => "serde_json::Value".into(),
            other => panic!("field type {other:?} not yet supported"),
        }
    }

    /// Whether a (non-`Option`) field type can derive `Default`.
    fn default_able(&self, ty: &str) -> bool {
        ty.starts_with("Vec<")
            || matches!(ty, "String" | "i64" | "f64" | "bool" | "serde_json::Value")
            || self.default_types.contains(ty)
    }
}

fn call_fn(has_return: bool) -> &'static str {
    if has_return {
        "call"
    } else {
        "call_no_response"
    }
}

fn is_inline_enum(f: &Value) -> bool {
    f.get("enum").is_some() && f["type"].as_str() == Some("string")
}

/// Renders a string enum: camelCase wire form, PascalCase variants, with the
/// first variant marked `#[default]` so the enum (and structs embedding it)
/// can derive `Default`.
fn render_enum(name: &str, leading_doc: &str, values: &[Value]) -> String {
    let variants = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let var = pascal_case(v.as_str().unwrap());
            if i == 0 {
                format!("    #[default]\n    {var},")
            } else {
                format!("    {var},")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{leading_doc}#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]\n#[serde(rename_all = \"camelCase\")]\npub enum {name} {{\n{variants}\n}}"
    )
}

// ── Formatting helpers ─────────────────────────────────────────────────────

fn section(title: &str, items: &[String]) -> String {
    format!("{}\n\n{}", header(title), items.join("\n\n"))
}

fn header(title: &str) -> String {
    let prefix = format!("// ── {title} ");
    let dashes = HEADER_WIDTH.saturating_sub(prefix.chars().count());
    format!("{prefix}{}", "─".repeat(dashes))
}

/// `///`-prefixed doc lines for a description (empty if none).
fn doc_lines(desc: Option<&str>, indent: usize) -> String {
    let pad = " ".repeat(indent);
    match desc {
        Some(d) => normalize_desc(d)
            .split('\n')
            .map(|l| format!("{pad}/// {l}\n"))
            .collect(),
        None => String::new(),
    }
}

/// A method doc block: description lines, a blank `///`, then `extra`.
fn doc_block(desc: Option<&str>, indent: usize, extra: &str) -> String {
    let pad = " ".repeat(indent);
    let mut s = doc_lines(desc, indent);
    s.push_str(&format!("{pad}///\n{pad}/// {extra}\n"));
    s
}

/// Trims a description and ensures it ends with sentence punctuation — humans
/// add a trailing period to JSON descriptions that omit one, so do the same.
fn normalize_desc(d: &str) -> String {
    let t = d.trim_end();
    match t.chars().last() {
        Some(c) if ".!?:)".contains(c) => t.to_string(),
        Some(_) => format!("{t}."),
        None => String::new(),
    }
}

/// Rust field identifier for a wire name: snake_case, escaping Rust keywords as
/// raw identifiers (`type` → `r#type`). Purely mechanical — no per-name
/// semantic aliases, so it stays correct as the protocol JSON evolves. The
/// handful of keywords that can't be raw identifiers get a trailing underscore.
fn field_ident(wire: &str) -> String {
    let s = snake_case(wire);
    if matches!(s.as_str(), "self" | "super" | "crate" | "Self") {
        format!("{s}_")
    } else if is_rust_keyword(&s) {
        format!("r#{s}")
    } else {
        s
    }
}

fn is_rust_keyword(s: &str) -> bool {
    matches!(
        s,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
            | "gen"
    )
}

fn pascal_domain(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
        None => String::new(),
    }
}

fn pascal_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn lower_camel(snake: &str) -> String {
    let mut out = String::new();
    for (i, part) in snake.split('_').enumerate() {
        if i == 0 {
            out.push_str(part);
        } else if let Some(f) = part.chars().next() {
            out.push(f.to_ascii_uppercase());
            out.push_str(&part[f.len_utf8()..]);
        }
    }
    out
}

fn snake_case(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                let prev = chars[i - 1];
                let next_lower = chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase());
                if prev.is_ascii_lowercase()
                    || prev.is_ascii_digit()
                    || (prev.is_ascii_uppercase() && next_lower)
                {
                    out.push('_');
                }
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}
