//! Codegen for the typed CDP bindings under `src/cdp/`.
//!
//! Downloads the pinned DevTools protocol (`browser_protocol.json` +
//! `js_protocol.json` at [`PROTOCOL_TAG`]) into a `target/` cache, then emits
//! the typed Rust bindings. Writes into `src/cdp/`
//! (build.rs → src, by design) but only:
//!   - for the domains listed in `MANIFEST` (opt-in, curated coverage),
//!   - plus a generated `common.rs` holding types pulled in from domains we
//!     don't generate (e.g. `Network`, `Debugger`), and
//!   - only when the generated text actually differs (write-if-changed), so a
//!     clean tree stays clean and there is no rebuild loop.
//!
//! Cross-domain `$ref`s are routed automatically by reachability — no hardcoded
//! type list: a ref to a manifest domain points at that module; a ref to a
//! domain we don't generate points at `common` (transitively closed).
//!
//! The download is cached under `target/cdp-protocol/<tag>/`; bumping
//! [`PROTOCOL_TAG`] re-downloads. If the protocol can't be fetched (offline, no
//! cache) codegen is skipped and the committed bindings are used as-is.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Pinned DevTools protocol release.
/// <https://github.com/ChromeDevTools/devtools-protocol/tags>
const PROTOCOL_TAG: &str = "v0.0.1643702";

/// Domains to generate (full: types + commands + events).
const MANIFEST: &[&str] = &[
    "io",
    "input",
    "runtime",
    "target",
    "browser",
    "page",
    "dom",
    "emulation",
    "storage",
];

const HEADER_WIDTH: usize = 80;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let cdp = Path::new(&manifest_dir).join("src/cdp");
    println!("cargo::rerun-if-changed=build.rs");

    // All protocol domains, keyed by lowercase name. Skip codegen (keep the
    // committed bindings) if the protocol can't be fetched.
    let Some(domains) = fetch_protocol() else {
        println!(
            "cargo::warning=CDP protocol {PROTOCOL_TAG} unavailable (offline, no cache); using committed bindings"
        );
        return;
    };

    let manifest: BTreeSet<String> = MANIFEST.iter().map(|s| s.to_string()).collect();

    // Pre-pass: discover foreign types (owned by domains we don't generate)
    // reachable from the manifest, closed transitively, named in `common`.
    let common = CommonSet::discover(&manifest, &domains);
    let reg = Registry { common: &common };

    if !common.order.is_empty() {
        let code = rustfmt(emit_common(&reg, &common));
        write_if_changed(&cdp.join("common.rs"), &code);
    }

    for d in MANIFEST {
        let dom = domains
            .get(*d)
            .unwrap_or_else(|| panic!("domain {d} not found in protocol {PROTOCOL_TAG}"));
        let code = rustfmt(Cx::new(&reg, d, dom).generate());
        write_if_changed(&cdp.join(format!("{d}.rs")), &code);
    }
}

/// Downloads (and caches under `target/`) the pinned protocol files, returning
/// every domain keyed by lowercase name. `None` if the files are neither cached
/// nor downloadable (offline) — the caller then keeps the committed bindings.
fn fetch_protocol() -> Option<BTreeMap<String, Value>> {
    const FILES: [&str; 2] = ["browser_protocol.json", "js_protocol.json"];

    let out = PathBuf::from(std::env::var("OUT_DIR").ok()?);
    // OUT_DIR = <target>/<profile>/build/<pkg-hash>/out → climb to <target>.
    let target = out.ancestors().nth(4).unwrap_or(&out);
    let cache = target.join("cdp-protocol").join(PROTOCOL_TAG);
    std::fs::create_dir_all(&cache).ok()?;

    let mut domains = BTreeMap::new();
    for file in FILES {
        let path = cache.join(file);
        if !path.exists() {
            let url = format!(
                "https://github.com/ChromeDevTools/devtools-protocol/raw/refs/tags/{PROTOCOL_TAG}/json/{file}"
            );
            let status = std::process::Command::new("curl")
                .args(["-sSfL", "--retry", "2", "-o"])
                .arg(&path)
                .arg(&url)
                .status();
            if !matches!(status, Ok(s) if s.success()) {
                let _ = std::fs::remove_file(&path);
                return None;
            }
        }
        let proto: Value = serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
        for dom in proto["domains"].as_array()? {
            if let Some(name) = dom["domain"].as_str() {
                domains.insert(name.to_lowercase(), dom.clone());
            }
        }
    }
    Some(domains)
}

/// Formats generated code with `rustfmt` so the output is stable under
/// `cargo fmt` (no tug-of-war with the formatter). Falls back to the
/// unformatted source if `rustfmt` isn't available.
fn rustfmt(src: String) -> String {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = match Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return src,
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(src.as_bytes());
    }
    match child.wait_with_output() {
        Ok(o) if o.status.success() => String::from_utf8(o.stdout).unwrap_or(src),
        _ => src,
    }
}

fn write_if_changed(path: &Path, content: &str) {
    if std::fs::read_to_string(path).ok().as_deref() == Some(content) {
        return;
    }
    std::fs::write(path, content).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

// ── Reference resolution ─────────────────────────────────────────────────────

/// Resolves `$ref`s to their owning module. Foreign types live in `common`;
/// everything else (manifest or hand-written) lives in its `cdp::<domain>`.
struct Registry<'a> {
    common: &'a CommonSet,
}

impl Registry<'_> {
    /// Resolves a `$ref` (qualified `Domain.Type` or local `Type`) to its Rust
    /// name and owning module (`"common"` or a domain).
    fn resolve(&self, current: &str, r: &str) -> (String, String) {
        let (owner, id) = match r.split_once('.') {
            Some((d, t)) => (d.to_lowercase(), t.to_string()),
            None => (current.to_string(), r.to_string()),
        };
        if let Some(name) = self.common.name_of(&owner, &id) {
            (name, "common".to_string())
        } else {
            // Manifest or hand-written domain (or current module).
            (type_name(&id), owner)
        }
    }
}

/// The set of foreign types (owner has no generated/hand-written module) pulled
/// into `common`, discovered transitively from the manifest.
struct CommonSet {
    /// (owner_lc, id) → def JSON, in discovery order.
    order: Vec<(String, String)>,
    defs: BTreeMap<(String, String), Value>,
    /// (owner_lc, id) → emitted name in `common` (disambiguated on collision).
    names: BTreeMap<(String, String), String>,
}

impl CommonSet {
    fn name_of(&self, owner_lc: &str, id: &str) -> Option<String> {
        self.names
            .get(&(owner_lc.to_string(), id.to_string()))
            .cloned()
    }

    fn discover(manifest: &BTreeSet<String>, domains: &BTreeMap<String, Value>) -> Self {
        let is_local = |d: &str| manifest.contains(d);

        // Seed queue with foreign refs from the manifest domains.
        let mut queue: Vec<(String, String)> = Vec::new();
        for d in manifest {
            if let Some(j) = domains.get(d) {
                for (owner, id) in refs_of_domain(j) {
                    if !is_local(&owner) {
                        queue.push((owner, id));
                    }
                }
            }
        }

        let mut defs: BTreeMap<(String, String), Value> = BTreeMap::new();
        let mut order: Vec<(String, String)> = Vec::new();
        let mut seen: BTreeSet<(String, String)> = BTreeSet::new();

        while let Some((owner, id)) = queue.pop() {
            if !seen.insert((owner.clone(), id.clone())) {
                continue;
            }
            let Some(def) = domains.get(&owner).and_then(|dj| type_def(dj, &id)) else {
                continue; // ref to something we can't resolve; skip (compile will flag)
            };
            // Follow this type's own refs; foreign ones extend the closure.
            for (o2, i2) in refs_in_type(&def, &owner) {
                if !is_local(&o2) && !seen.contains(&(o2.clone(), i2.clone())) {
                    queue.push((o2, i2));
                }
            }
            order.push((owner.clone(), id.clone()));
            defs.insert((owner, id), def.clone());
        }

        order.sort();
        // Assign names (normalized), prefixing the owner on collisions.
        let mut by_norm: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for (o, i) in &order {
            by_norm
                .entry(type_name(i))
                .or_default()
                .push((o.clone(), i.clone()));
        }
        let mut names = BTreeMap::new();
        for (norm, owners) in by_norm {
            if owners.len() == 1 {
                let (o, i) = &owners[0];
                names.insert((o.clone(), i.clone()), norm.clone());
            } else {
                for (o, i) in owners {
                    names.insert(
                        (o.clone(), i.clone()),
                        format!("{}{norm}", pascal_domain(&o)),
                    );
                }
            }
        }

        CommonSet { order, defs, names }
    }
}

/// All (owner_lc, type_id) referenced by a domain's commands, events and types.
fn refs_of_domain(json: &Value) -> Vec<(String, String)> {
    let domain = json["domain"].as_str().unwrap_or("").to_lowercase();
    let mut out = Vec::new();
    for sect in ["types", "commands", "events"] {
        if let Some(arr) = json.get(sect).and_then(Value::as_array) {
            for item in arr {
                collect_refs(item, &domain, &mut out);
            }
        }
    }
    out
}

fn refs_in_type(def: &Value, owner_lc: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    collect_refs(def, owner_lc, &mut out);
    out
}

/// Recursively collects `$ref`s under `node`, qualifying bare refs with `owner`.
fn collect_refs(node: &Value, owner: &str, out: &mut Vec<(String, String)>) {
    match node {
        Value::Object(map) => {
            if let Some(r) = map.get("$ref").and_then(Value::as_str) {
                let (o, i) = match r.split_once('.') {
                    Some((d, t)) => (d.to_lowercase(), t.to_string()),
                    None => (owner.to_string(), r.to_string()),
                };
                out.push((o, i));
            }
            for v in map.values() {
                collect_refs(v, owner, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_refs(v, owner, out);
            }
        }
        _ => {}
    }
}

fn type_def(domain_json: &Value, id: &str) -> Option<Value> {
    domain_json
        .get("types")
        .and_then(Value::as_array)?
        .iter()
        .find(|t| t["id"].as_str() == Some(id))
        .cloned()
}

// ── Per-module generation ────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Param,
    Return,
    Event,
    Object,
}

/// Emits the `common` module (foreign types only, no commands/events).
fn emit_common(reg: &Registry, common: &CommonSet) -> String {
    let mut cx = Cx::module(reg, "common");
    // Pre-register enum names so structs can derive `Default` regardless of order.
    for key in &common.order {
        let def = &common.defs[key];
        if def.get("enum").is_some() && def["type"].as_str() == Some("string") {
            cx.default_types.insert(common.names[key].clone());
        }
    }
    let items: Vec<String> = common
        .order
        .iter()
        .map(|key| {
            let def = &common.defs[key];
            let name = common.names[key].clone();
            cx.gen_type_named(def, &name)
        })
        .collect();

    let imports = cx.imports();
    let mut sections = vec![imports, section("Types", &items)];
    if !cx.inline_enums.is_empty() {
        sections.insert(1, section("Inline enums", &cx.inline_enums.clone()));
    }
    let mut out = sections.join("\n\n");
    out.push('\n');
    out
}

/// Generation state for one emitted module (a domain or `common`).
struct Cx<'a> {
    reg: &'a Registry<'a>,
    module: String,
    json: Option<&'a Value>,
    uses: BTreeSet<String>,
    inline_enums: Vec<String>,
    inline_seen: BTreeSet<String>,
    default_types: BTreeSet<String>,
    current_struct: Option<String>,
    needs_ser: bool,
    needs_de: bool,
}

impl<'a> Cx<'a> {
    fn new(reg: &'a Registry<'a>, domain: &str, json: &'a Value) -> Self {
        let mut cx = Self::module(reg, &domain.to_lowercase());
        cx.json = Some(json);
        cx
    }

    fn module(reg: &'a Registry<'a>, module: &str) -> Self {
        Self {
            reg,
            module: module.to_string(),
            json: None,
            uses: BTreeSet::new(),
            inline_enums: Vec::new(),
            inline_seen: BTreeSet::new(),
            default_types: BTreeSet::new(),
            current_struct: None,
            needs_ser: false,
            needs_de: false,
        }
    }

    fn imports(&self) -> String {
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
        if serde_use.is_empty() {
            crate_block
        } else if crate_block.is_empty() {
            serde_use.to_string()
        } else {
            format!("{serde_use}\n\n{crate_block}")
        }
    }

    fn generate(mut self) -> String {
        // Commands' trait + impl always use these.
        self.uses.insert("crate::error::Result".to_string());
        self.uses.insert("crate::session::CdpSession".to_string());
        let json = self.json.unwrap();
        let domain = json["domain"].as_str().unwrap().to_string();
        let pascal = pascal_domain(&domain);
        let prefix = domain.to_lowercase();
        let trait_name = format!("{pascal}Commands");
        let empty = Vec::new();

        // Enums always derive `Default`; pre-register them so a struct can derive
        // `Default` regardless of whether the enum is declared before or after it.
        for t in json
            .get("types")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
        {
            if t.get("enum").is_some()
                && t["type"].as_str() == Some("string")
                && let Some(id) = t["id"].as_str()
            {
                self.default_types.insert(type_name(id));
            }
        }

        let type_items: Vec<String> = json
            .get("types")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
            .iter()
            .map(|t| self.gen_type(t))
            .collect();

        let mut param_items = Vec::new();
        let mut return_items = Vec::new();
        let mut trait_methods = Vec::new();
        let mut internal_params = Vec::new();
        let mut impl_methods = Vec::new();

        for cmd in json
            .get("commands")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
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
            let direct = !params.is_empty()
                && !has_optional
                && !any_inline_enum
                && (1..=2).contains(&required.len());

            if params.is_empty() {
                trait_methods.push(format!("{doc}    async fn {method}(&self) -> {ret_sig};"));
                let cf = call_fn(return_ty.is_some());
                impl_methods.push(format!(
                    "    async fn {method}(&self) -> {ret_sig} {{\n        self.{cf}(\"{cdp_name}\", &serde_json::json!({{}})).await\n    }}"
                ));
            } else if direct {
                self.needs_ser = true;
                let (sig, fields, ctor, lt) = self.direct_args(&required);
                trait_methods.push(format!(
                    "{doc}    async fn {method}(&self, {sig}) -> {ret_sig};"
                ));
                let sname = format!("{cmd_pascal}InternalParams");
                internal_params.push(format!(
                    "#[derive(Serialize)]\n#[serde(rename_all = \"camelCase\")]\nstruct {sname}{lt} {{\n{fields}\n}}"
                ));
                let cf = call_fn(return_ty.is_some());
                impl_methods.push(format!(
                    "    async fn {method}(&self, {sig}) -> {ret_sig} {{\n        let params = {sname} {{ {ctor} }};\n        self.{cf}(\"{cdp_name}\", &params).await\n    }}"
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
                let cf = call_fn(return_ty.is_some());
                impl_methods.push(format!(
                    "    async fn {method}(&self, params: &{pname}) -> {ret_sig} {{\n        self.{cf}(\"{cdp_name}\", params).await\n    }}"
                ));
            }
        }

        let mut event_items = Vec::new();
        for ev in json
            .get("events")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
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

        let imports = self.imports();

        let mut trait_doc = format!("/// `{domain}` domain CDP methods.\n");
        if let Some(d) = json["description"].as_str() {
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

        let mut sections = vec![imports, section("Types", &type_items)];
        if !self.inline_enums.is_empty() {
            sections.push(section("Inline enums", &self.inline_enums.clone()));
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

    fn gen_type(&mut self, t: &Value) -> String {
        let id = t["id"].as_str().unwrap();
        self.gen_type_named(t, &type_name(id))
    }

    fn gen_type_named(&mut self, t: &Value, name: &str) -> String {
        let doc = doc_lines(t["description"].as_str(), 0);
        match t["type"].as_str() {
            Some("string") if t.get("enum").is_some() => {
                self.needs_ser = true;
                self.needs_de = true;
                self.default_types.insert(name.to_string());
                render_enum(name, &doc, t["enum"].as_array().unwrap())
            }
            Some("string") => {
                self.needs_ser = true;
                self.needs_de = true;
                format!(
                    "{doc}#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]\npub struct {name}(pub String);"
                )
            }
            Some("integer") => {
                self.needs_ser = true;
                self.needs_de = true;
                format!(
                    "{doc}#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\npub struct {name}(pub i64);"
                )
            }
            Some("number") => {
                self.needs_ser = true;
                self.needs_de = true;
                format!(
                    "{doc}#[derive(Debug, Clone, Copy, Serialize, Deserialize)]\npub struct {name}(pub f64);"
                )
            }
            Some("array") => {
                // Array-typed protocol types (e.g. DOM.Quad) are transparent
                // aliases, so callers can index/iterate them directly.
                let inner = self.map_type(t, name);
                format!("{doc}pub type {name} = {inner};")
            }
            Some("object") => {
                let props = t["properties"].as_array().cloned().unwrap_or_default();
                self.gen_struct(name, &doc, &props, Mode::Object, name)
            }
            None => {
                // Untyped type alias (rare): treat as opaque JSON.
                self.needs_ser = true;
                self.needs_de = true;
                format!(
                    "{doc}#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {name}(pub serde_json::Value);"
                )
            }
            other => panic!("type kind {other:?} not supported (type {name})"),
        }
    }

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
        let prev = self.current_struct.replace(name.to_string());

        let mut body = String::new();
        let mut all_default = !fields.is_empty();
        for f in fields {
            let (rendered, ty, optional) = self.render_field(f, mode, ctx);
            if !optional && !self.default_able(&ty) {
                all_default = false;
            }
            body.push_str(&rendered);
        }
        self.current_struct = prev;

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

    /// Returns (rendered, type-without-`Option`, optional).
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
            self.map_type(f, ctx)
        };

        let mut out = doc_lines(f["description"].as_str(), 4);
        let logical = ident.strip_prefix("r#").unwrap_or(&ident);
        if lower_camel(logical) != wire {
            out.push_str(&format!("    #[serde(rename = \"{wire}\")]\n"));
        }
        let decl_ty = if optional {
            // Skip `None` on serialize (CDP rejects explicit `null` for optional
            // params); `default` so deserialize tolerates an absent field.
            match mode {
                Mode::Param => {
                    out.push_str("    #[serde(skip_serializing_if = \"Option::is_none\")]\n")
                }
                Mode::Object => out
                    .push_str("    #[serde(default, skip_serializing_if = \"Option::is_none\")]\n"),
                Mode::Return | Mode::Event => out.push_str("    #[serde(default)]\n"),
            }
            format!("Option<{ty}>")
        } else {
            ty.clone()
        };
        out.push_str(&format!("    pub {ident}: {decl_ty},\n"));
        (out, ty, optional)
    }

    fn direct_args(&mut self, required: &[&Value]) -> (String, String, String, String) {
        let mut sig = Vec::new();
        let mut fields = Vec::new();
        let mut ctor = Vec::new();
        let mut lt = false;
        for p in required {
            let ident = field_ident(p["name"].as_str().unwrap());
            let mapped = self.map_type(p, "");
            let (ty, by_ref) = if mapped == "String" {
                ("str".to_string(), true)
            } else if matches!(mapped.as_str(), "bool" | "i64" | "f64") {
                (mapped, false)
            } else if let Some(inner) = mapped
                .strip_prefix("Vec<")
                .and_then(|s| s.strip_suffix('>'))
            {
                // Borrow arrays as slices (`&[T]`), per clippy.
                (format!("[{inner}]"), true)
            } else {
                (mapped, true)
            };
            if by_ref {
                lt = true;
                sig.push(format!("{ident}: &{ty}"));
                fields.push(format!("    {ident}: &'a {ty},"));
            } else {
                sig.push(format!("{ident}: {ty}"));
                fields.push(format!("    {ident}: {ty},"));
            }
            ctor.push(ident);
        }
        let lifetime = if lt { "<'a>" } else { "" };
        (
            sig.join(", "),
            fields.join("\n"),
            ctor.join(", "),
            lifetime.to_string(),
        )
    }

    /// Maps a JSON node to a Rust type, recording cross-module imports and
    /// boxing direct self-references (recursive types).
    fn map_type(&mut self, node: &Value, ctx: &str) -> String {
        let _ = ctx;
        if let Some(r) = node.get("$ref").and_then(Value::as_str) {
            let (name, module) = self.reg.resolve(&self.module, r);
            if module != self.module {
                self.uses.insert(format!("crate::cdp::{module}::{name}"));
            }
            // Box a direct self-reference so the type has a finite size.
            if Some(&name) == self.current_struct.as_ref() {
                return format!("Box<{name}>");
            }
            return name;
        }
        match node["type"].as_str() {
            Some("string") => "String".into(),
            Some("integer") => "i64".into(),
            Some("number") => "f64".into(),
            Some("boolean") => "bool".into(),
            Some("array") => format!("Vec<{}>", self.map_type(&node["items"], ctx)),
            Some("object") | Some("any") | None => "serde_json::Value".into(),
            other => panic!("field type {other:?} not supported"),
        }
    }

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

/// Normalizes a protocol type id to Rust convention: CDP spells ID types as
/// `TargetID`/`WindowID`; Rust uses `TargetId`/`WindowId`.
fn type_name(id: &str) -> String {
    match id.strip_suffix("ID") {
        Some(stem) if !stem.is_empty() => format!("{stem}Id"),
        _ => id.to_string(),
    }
}

fn is_inline_enum(f: &Value) -> bool {
    f.get("enum").is_some() && f["type"].as_str() == Some("string")
}

fn render_enum(name: &str, leading_doc: &str, values: &[Value]) -> String {
    // Per-variant `rename` to the exact wire value (no `rename_all`), so any
    // casing CDP uses — camelCase (`timeTicks`) or PascalCase (`QuirksMode`) —
    // round-trips correctly.
    let variants = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let val = v.as_str().unwrap();
            let var = enum_variant(val);
            let mut attrs = String::new();
            if i == 0 {
                attrs.push_str("    #[default]\n");
            }
            if var != val {
                attrs.push_str(&format!("    #[serde(rename = \"{val}\")]\n"));
            }
            format!("{attrs}    {var},")
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{leading_doc}#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]\npub enum {name} {{\n{variants}\n}}"
    )
}

/// PascalCase variant name for an enum wire value (handles values that aren't
/// already identifiers, e.g. `"text/html"` → `TextHtml`).
fn enum_variant(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect();
    let mut out = String::new();
    for word in cleaned.split_whitespace() {
        out.push_str(&pascal_case(word));
    }
    if out.is_empty() || out.chars().next().unwrap().is_ascii_digit() {
        format!("V{out}")
    } else {
        out
    }
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

fn doc_block(desc: Option<&str>, indent: usize, extra: &str) -> String {
    let pad = " ".repeat(indent);
    let mut s = doc_lines(desc, indent);
    s.push_str(&format!("{pad}///\n{pad}/// {extra}\n"));
    s
}

fn normalize_desc(d: &str) -> String {
    let t = d.trim_end();
    match t.chars().last() {
        Some(c) if ".!?:)".contains(c) => t.to_string(),
        Some(_) => format!("{t}."),
        None => String::new(),
    }
}

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
