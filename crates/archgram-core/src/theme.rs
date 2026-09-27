//! A project's own design tokens as archgram's theme (docs/SPEC.md, Theme
//! file). A mapping file names the project's DTCG resolver, the inputs
//! that pick its light and its dark theme, and which of its colour tokens
//! fills each of archgram's roles; a role it leaves out keeps the mono
//! palette's colour. The result must keep DESIGN.md's contrast (Colors).
//!
//! Only what reading colours needs of the Resolver Module (Design Tokens
//! 2025.10) is supported: sets, modifiers and their inputs, the resolution
//! order, `$ref` to files and to `#/sets/…`, inline sources, curly-brace
//! aliases and JSON Pointer `$ref`s inside values, `$type` inherited from
//! groups. Remote references and group `$extends` are refused with a
//! message. The core reads no file: the caller hands over each file's text
//! by its path, relative to the mapping file.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Deserialize;
use serde_json::Value;

use crate::color::{Rgb, Shortfall, check, from_dtcg};
use crate::tokens::{Colors, Role, theme};

/// The light and the dark colours a drawing uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeColors {
    pub light: Colors,
    pub dark: Colors,
}

/// The only version of the mapping format.
pub const THEME_FORMAT: u32 = 1;

/// How deep aliases may chain before archgram calls it a loop.
const MAX_ALIASES: usize = 64;

/// A problem with the mapping or the tokens it reads: in which file, where
/// in it (a JSON pointer, or `/` for the whole file) and what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportError {
    pub file: String,
    pub pointer: String,
    pub message: String,
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pointer = if self.pointer.is_empty() {
            "/"
        } else {
            &self.pointer
        };
        write!(f, "{} {pointer}: {}", self.file, self.message)
    }
}

fn error(file: &str, pointer: impl Into<String>, message: impl Into<String>) -> ImportError {
    ImportError {
        file: file.to_owned(),
        pointer: pointer.into(),
        message: message.into(),
    }
}

/// The mapping file (`archgram.theme.json`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mapping {
    #[serde(rename = "$schema", default)]
    _schema: Option<String>,
    version: u32,
    /// The project's resolver document, relative to the mapping file.
    resolver: String,
    /// Each archgram role's token id, for both themes.
    #[serde(default)]
    roles: BTreeMap<String, String>,
    themes: Themes,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Themes {
    light: ThemeInputs,
    dark: ThemeInputs,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeInputs {
    /// The resolver's inputs: each modifier's context.
    #[serde(default)]
    inputs: BTreeMap<String, String>,
    /// Roles this theme maps differently, over `roles`.
    #[serde(default)]
    roles: BTreeMap<String, String>,
}

/// What an import found: both themes' colours, and where each role's came
/// from (a token id, or `None` for the mono palette's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    pub colors: ThemeColors,
    pub light: Vec<(Role, Option<String>, Rgb)>,
    pub dark: Vec<(Role, Option<String>, Rgb)>,
}

/// Reads the mapping file at `path` and the tokens it names, each file's
/// text from `read` by its path, and returns both themes' colours.
///
/// # Errors
///
/// Every problem found, each in its file: a mapping or resolver that breaks
/// the format, a missing file or token, a role no colour fills, a loop of
/// aliases, a colour that cannot be read, or a pair below its contrast.
///
/// # Panics
///
/// Only if archgram's own tokens lost the mono palette, which
/// `tests/tokens.rs` guards.
pub fn import(path: &str, read: &dyn Fn(&str) -> Option<String>) -> Result<Imported, Vec<ImportError>> {
    let text = read(path).ok_or_else(|| vec![error(path, "", "the file cannot be read")])?;
    let mapping: Mapping = serde_json::from_str(&text).map_err(|e| {
        vec![error(
            path,
            format!("line {} column {}", e.line(), e.column()),
            e.to_string()
                .split(" at line ")
                .next()
                .unwrap_or_default()
                .to_owned(),
        )]
    })?;
    let mut errors = Vec::new();
    if mapping.version != THEME_FORMAT {
        errors.push(error(
            path,
            "/version",
            format!(
                "unsupported format version {}; this archgram reads version {THEME_FORMAT}",
                mapping.version
            ),
        ));
    }
    // Every role named, in either list, is one of archgram's.
    let known: Vec<&str> = Role::ALL.iter().map(|r| r.name()).collect();
    let lists = [
        ("/roles".to_owned(), &mapping.roles),
        ("/themes/light/roles".to_owned(), &mapping.themes.light.roles),
        ("/themes/dark/roles".to_owned(), &mapping.themes.dark.roles),
    ];
    for (at, list) in lists {
        for name in list.keys().filter(|n| Role::from_name(n).is_none()) {
            errors.push(error(
                path,
                format!("{at}/{name}"),
                format!(
                    "`{name}` is not an archgram role; the roles are {}",
                    known.join(", ")
                ),
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let resolver_path = join(path, &mapping.resolver);
    let mut docs = Documents {
        read,
        cache: BTreeMap::new(),
    };
    let mut result = Vec::new();
    for (name, inputs) in [("light", &mapping.themes.light), ("dark", &mapping.themes.dark)] {
        let mut roles = mapping.roles.clone();
        roles.extend(inputs.roles.clone());
        let base = theme("mono", name)
            .expect("mono has a light and a dark theme")
            .colors;
        match one_theme(path, &resolver_path, (name, inputs), &roles, &mut docs, base) {
            Ok(found) => result.push(found),
            Err(e) => errors.extend(e),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let (dark, light) = (result.pop().expect("dark"), result.pop().expect("light"));
    Ok(Imported {
        colors: ThemeColors {
            light: light.0,
            dark: dark.0,
        },
        light: light.1,
        dark: dark.1,
    })
}

type Found = (Colors, Vec<(Role, Option<String>, Rgb)>);

/// One theme: the resolver's tokens for its inputs, each mapped role's colour
/// over the mono palette's, then the contrast check.
fn one_theme(
    path: &str,
    resolver_path: &str,
    (name, inputs): (&str, &ThemeInputs),
    roles: &BTreeMap<String, String>,
    docs: &mut Documents,
    base: Colors,
) -> Result<Found, Vec<ImportError>> {
    let tree = resolve_tree(
        resolver_path,
        &inputs.inputs,
        docs,
        &format!("/themes/{name}/inputs"),
        path,
    )?;
    let tokens =
        Tokens::flatten(&tree).map_err(|(pointer, message)| vec![error(resolver_path, pointer, message)])?;
    let mut colors = base;
    let mut sources = Vec::new();
    let mut errors = Vec::new();
    for (role_name, id) in roles {
        let pointer = if inputs.roles.contains_key(role_name) {
            format!("/themes/{name}/roles/{role_name}")
        } else {
            format!("/roles/{role_name}")
        };
        let role = Role::from_name(role_name).expect("roles checked above");
        let id = id.trim().trim_start_matches('{').trim_end_matches('}');
        match tokens.colour(id, &tree) {
            Ok(rgb) => colors.set(role, rgb),
            Err(message) => errors.push(error(path, pointer, format!("{name}: `{id}`: {message}"))),
        }
    }
    for role in Role::ALL {
        let id = roles
            .get(role.name())
            .map(|id| id.trim().trim_start_matches('{').trim_end_matches('}').to_owned());
        sources.push((role, id, colors.get(role)));
    }
    if errors.is_empty() {
        errors.extend(
            check(&colors)
                .iter()
                .map(|s: &Shortfall| error(path, format!("/themes/{name}"), format!("{name}: {s}"))),
        );
    }
    if errors.is_empty() {
        Ok((colors, sources))
    } else {
        Err(errors)
    }
}

/// The files read so far, parsed, by path.
struct Documents<'a> {
    read: &'a dyn Fn(&str) -> Option<String>,
    cache: BTreeMap<String, Value>,
}

impl Documents<'_> {
    fn get(&mut self, path: &str) -> Result<Value, String> {
        if let Some(v) = self.cache.get(path) {
            return Ok(v.clone());
        }
        let text = (self.read)(path).ok_or("the file cannot be read")?;
        let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        self.cache.insert(path.to_owned(), v.clone());
        Ok(v)
    }
}

/// The token tree the resolver gives for `inputs`: the resolution order's
/// sets and chosen contexts merged in turn, the later winning.
fn resolve_tree(
    resolver_path: &str,
    inputs: &BTreeMap<String, String>,
    docs: &mut Documents,
    inputs_pointer: &str,
    mapping_path: &str,
) -> Result<Value, Vec<ImportError>> {
    let doc = docs
        .get(resolver_path)
        .map_err(|m| vec![error(resolver_path, "", m)])?;
    let r = Resolving {
        resolver_path,
        mapping_path,
        inputs,
        inputs_pointer,
        doc: &doc,
    };
    if doc.get("version").and_then(Value::as_str) != Some("2025.10") {
        return Err(r.at("/version", "a resolver document's `version` is `2025.10`"));
    }
    let order = doc
        .get("resolutionOrder")
        .and_then(Value::as_array)
        .ok_or_else(|| r.at("/resolutionOrder", "the resolver needs a `resolutionOrder` list"))?;
    r.check_inputs(order)?;
    let mut tree = Value::Object(serde_json::Map::new());
    let mut names = BTreeSet::new();
    for (i, entry) in order.iter().enumerate() {
        let (kind, body, pointer) = r.entry(i, entry, &mut names)?;
        let (sources, at) = r.sources(&kind, &body, &pointer)?;
        merge_sources(
            &mut tree,
            &sources,
            &at,
            resolver_path,
            &doc,
            docs,
            &mut Vec::new(),
        )?;
    }
    Ok(tree)
}

/// What resolving one theme needs to hand: the files' paths for messages,
/// the inputs and the resolver document.
struct Resolving<'a> {
    resolver_path: &'a str,
    mapping_path: &'a str,
    inputs: &'a BTreeMap<String, String>,
    inputs_pointer: &'a str,
    doc: &'a Value,
}

impl Resolving<'_> {
    fn at(&self, pointer: &str, message: impl Into<String>) -> Vec<ImportError> {
        vec![error(self.resolver_path, pointer, message)]
    }

    fn in_mapping(&self, pointer: String, message: String) -> Vec<ImportError> {
        vec![error(self.mapping_path, pointer, message)]
    }

    /// Every input names a modifier the resolver has, and a context.
    fn check_inputs(&self, order: &[Value]) -> Result<(), Vec<ImportError>> {
        let mut modifiers: BTreeSet<&str> = self
            .doc
            .get("modifiers")
            .and_then(Value::as_object)
            .map(|m| m.keys().map(String::as_str).collect())
            .unwrap_or_default();
        modifiers.extend(
            order
                .iter()
                .filter(|e| e.get("type").and_then(Value::as_str) == Some("modifier"))
                .filter_map(|e| e.get("name").and_then(Value::as_str)),
        );
        let mut errors = Vec::new();
        for (name, context) in self.inputs {
            let at = format!("{}/{name}", self.inputs_pointer);
            if !modifiers.contains(name.as_str()) {
                errors.extend(self.in_mapping(
                    at,
                    format!(
                        "the resolver has no modifier `{name}`; it has {}",
                        listed(modifiers.iter().copied())
                    ),
                ));
            } else if context.is_empty() {
                errors.extend(self.in_mapping(at, "an input is a context's name".into()));
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }

    /// One entry of the resolution order: whether it is a set or a
    /// modifier, its body and where it is.
    fn entry(
        &self,
        i: usize,
        entry: &Value,
        names: &mut BTreeSet<String>,
    ) -> Result<(String, Value, String), Vec<ImportError>> {
        let here = format!("/resolutionOrder/{i}");
        match (
            entry.get("$ref").and_then(Value::as_str),
            entry.get("type").and_then(Value::as_str),
        ) {
            (Some(r), _) => {
                let Some(target) = r.strip_prefix("#/") else {
                    return Err(self.at(
                        &here,
                        format!(
                            "`{r}`: the resolution order refers to this document's sets and modifiers only"
                        ),
                    ));
                };
                let (kind, name) = target.split_once('/').unwrap_or((target, ""));
                let body = self
                    .doc
                    .get(kind)
                    .and_then(|k| k.get(name))
                    .cloned()
                    .ok_or_else(|| self.at(&here, format!("`{r}` points at nothing")))?;
                Ok((
                    kind.trim_end_matches('s').to_owned(),
                    body,
                    format!("/{kind}/{name}"),
                ))
            }
            (None, Some(kind @ ("set" | "modifier"))) => {
                let name = entry.get("name").and_then(Value::as_str).unwrap_or_default();
                if !names.insert(name.to_owned()) {
                    return Err(self.at(&here, format!("two inline entries are named `{name}`")));
                }
                Ok((kind.to_owned(), entry.clone(), here))
            }
            _ => Err(self.at(
                &here,
                "an entry is a `$ref` to a set or modifier, or an inline one with `type` and `name`",
            )),
        }
    }

    /// The sources an entry brings: a set's, or the chosen context's of a
    /// modifier (its input, else its default), with where they are.
    fn sources(&self, kind: &str, body: &Value, pointer: &str) -> Result<(Value, String), Vec<ImportError>> {
        if kind != "modifier" {
            let sources = body
                .get("sources")
                .cloned()
                .ok_or_else(|| self.at(pointer, "a set needs `sources`"))?;
            return Ok((sources, format!("{pointer}/sources")));
        }
        let name = pointer.rsplit('/').next().unwrap_or_default();
        let name = body.get("name").and_then(Value::as_str).unwrap_or(name);
        let contexts = body
            .get("contexts")
            .and_then(Value::as_object)
            .ok_or_else(|| self.at(pointer, "a modifier needs `contexts`"))?;
        // One context is allowed (the format only advises against it);
        // none is an error.
        if contexts.is_empty() {
            return Err(self.at(pointer, "a modifier needs at least one context"));
        }
        let choices = || listed(contexts.keys().map(String::as_str));
        let chosen = self
            .inputs
            .get(name)
            .cloned()
            .or_else(|| body.get("default").and_then(Value::as_str).map(str::to_owned))
            .ok_or_else(|| {
                self.in_mapping(
                    self.inputs_pointer.to_owned(),
                    format!(
                        "modifier `{name}` has no default; give it an input, one of {}",
                        choices()
                    ),
                )
            })?;
        let sources = contexts.get(&chosen).ok_or_else(|| {
            self.in_mapping(
                format!("{}/{name}", self.inputs_pointer),
                format!(
                    "modifier `{name}` has no context `{chosen}`; it has {}",
                    choices()
                ),
            )
        })?;
        Ok((sources.clone(), format!("{pointer}/contexts/{chosen}")))
    }
}

/// Merges each source into `tree` in turn: a file (with an optional pointer
/// into it), another set, or tokens written inline.
fn merge_sources(
    tree: &mut Value,
    sources: &Value,
    pointer: &str,
    resolver_path: &str,
    doc: &Value,
    docs: &mut Documents,
    seen: &mut Vec<String>,
) -> Result<(), Vec<ImportError>> {
    let at = |p: &str, message: String| vec![error(resolver_path, p, message)];
    let list = sources
        .as_array()
        .ok_or_else(|| at(pointer, "sources are a list".into()))?;
    for (k, source) in list.iter().enumerate() {
        let here = format!("{pointer}/{k}");
        let Some(r) = source.get("$ref").and_then(Value::as_str) else {
            merge(tree, source);
            continue;
        };
        if r.contains("://") {
            return Err(at(
                &here,
                format!("`{r}`: archgram reads files beside the resolver, not remote references"),
            ));
        }
        let (file, fragment) = r.split_once('#').unwrap_or((r, ""));
        let mut value = if file.is_empty() {
            if fragment.starts_with("/modifiers/") {
                return Err(at(&here, format!("`{r}`: a set cannot refer to a modifier")));
            }
            if seen.iter().any(|s| s == r) {
                return Err(at(&here, format!("`{r}` refers back to itself")));
            }
            let target =
                pointer_get(doc, fragment).ok_or_else(|| at(&here, format!("`{r}` points at nothing")))?;
            let inner = target
                .get("sources")
                .cloned()
                .ok_or_else(|| at(&here, format!("`{r}` is not a set")))?;
            seen.push(r.to_owned());
            merge_sources(tree, &inner, fragment, resolver_path, doc, docs, seen)?;
            seen.pop();
            continue;
        } else {
            let path = join(resolver_path, file);
            docs.get(&path).map_err(|m| vec![error(&path, "", m)])?
        };
        if !fragment.is_empty() {
            value = pointer_get(&value, fragment)
                .cloned()
                .ok_or_else(|| at(&here, format!("`{r}` points at nothing")))?;
        }
        // Keys written beside `$ref` override what it brings.
        if let Some(extra) = source.as_object().filter(|o| o.len() > 1) {
            let mut local = extra.clone();
            local.remove("$ref");
            merge(&mut value, &Value::Object(local));
        }
        merge(tree, &value);
    }
    Ok(())
}

/// Deep merge, the later winning: groups merge key by key; a token is
/// replaced whole.
fn merge(into: &mut Value, from: &Value) {
    match (into, from) {
        (Value::Object(a), Value::Object(b)) if !a.contains_key("$value") && !b.contains_key("$value") => {
            for (k, v) in b {
                match a.get_mut(k) {
                    Some(existing) => merge(existing, v),
                    None => {
                        a.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (slot, v) => *slot = v.clone(),
    }
}

/// The value at a JSON Pointer (RFC 6901) into `v`.
fn pointer_get<'a>(v: &'a Value, pointer: &str) -> Option<&'a Value> {
    if pointer.is_empty() {
        return Some(v);
    }
    v.pointer(pointer)
}

/// Every token in the merged tree by its dotted id, with its type as the
/// groups give it.
struct Tokens {
    by_id: BTreeMap<String, (Option<String>, Value)>,
}

impl Tokens {
    fn flatten(tree: &Value) -> Result<Tokens, (String, String)> {
        fn walk(
            node: &Value,
            path: &mut Vec<String>,
            ty: Option<&str>,
            out: &mut BTreeMap<String, (Option<String>, Value)>,
        ) -> Result<(), (String, String)> {
            let Some(o) = node.as_object() else { return Ok(()) };
            if o.contains_key("$extends") {
                return Err((
                    format!("/{}", path.join("/")),
                    "group `$extends` is not supported; write the tokens out, or alias them".into(),
                ));
            }
            let ty = o.get("$type").and_then(Value::as_str).or(ty);
            if let Some(v) = o.get("$value") {
                out.insert(path.join("."), (ty.map(str::to_owned), v.clone()));
                return Ok(());
            }
            for (k, child) in o {
                if k.starts_with('$') {
                    continue;
                }
                path.push(k.clone());
                walk(child, path, ty, out)?;
                path.pop();
            }
            Ok(())
        }
        let mut by_id = BTreeMap::new();
        walk(tree, &mut Vec::new(), None, &mut by_id)?;
        Ok(Tokens { by_id })
    }

    /// Token `id`'s colour, its aliases and pointers followed.
    fn colour(&self, id: &str, tree: &Value) -> Result<Rgb, String> {
        let Some((ty, _)) = self.by_id.get(id) else {
            return Err(self.missing(id));
        };
        let ty = match ty {
            Some(t) => t.clone(),
            None => self.alias_type(id, &mut Vec::new())?,
        };
        if ty != "color" {
            return Err(format!("the token is a `{ty}`, not a colour"));
        }
        let value = self.resolve(&self.by_id[id].1, tree, &mut vec![id.to_owned()])?;
        from_dtcg(&value)
    }

    /// The type of an untyped token that aliases another: the target's.
    fn alias_type(&self, id: &str, chain: &mut Vec<String>) -> Result<String, String> {
        if chain.len() > MAX_ALIASES || chain.iter().any(|c| c == id) {
            return Err(format!("aliases loop: {}", chain.join(" → ")));
        }
        chain.push(id.to_owned());
        let (ty, value) = self
            .by_id
            .get(id)
            .ok_or_else(|| match chain.iter().rev().nth(1) {
                Some(from) => format!("`{from}` aliases `{id}`: {}", self.missing(id)),
                None => self.missing(id),
            })?;
        if let Some(t) = ty {
            return Ok(t.clone());
        }
        match value.as_str().and_then(alias) {
            Some(target) => self.alias_type(target, chain),
            None => Err("the token has no `$type`, and no group above it gives one".into()),
        }
    }

    /// `v` with every alias and pointer replaced by what it points at.
    fn resolve(&self, v: &Value, tree: &Value, chain: &mut Vec<String>) -> Result<Value, String> {
        if chain.len() > MAX_ALIASES {
            return Err(format!("aliases chain past {MAX_ALIASES}: {}", chain.join(" → ")));
        }
        match v {
            Value::String(s) => match alias(s) {
                Some(target) => {
                    if chain.iter().any(|c| c == target) {
                        chain.push(target.to_owned());
                        return Err(format!("aliases loop: {}", chain.join(" → ")));
                    }
                    let (_, value) = self.by_id.get(target).ok_or_else(|| {
                        let from = chain.last().map_or("", String::as_str);
                        format!("`{from}` aliases `{target}`: {}", self.missing(target))
                    })?;
                    chain.push(target.to_owned());
                    let out = self.resolve(value, tree, chain);
                    chain.pop();
                    out
                }
                None => Ok(v.clone()),
            },
            Value::Object(o) if o.contains_key("$ref") => {
                let r = o["$ref"].as_str().ok_or("`$ref` is a string")?;
                let Some(pointer) = r.strip_prefix('#') else {
                    return Err(format!("`{r}`: a value's `$ref` points into the tokens (`#/…`)"));
                };
                if chain.iter().any(|c| c == r) {
                    chain.push(r.to_owned());
                    return Err(format!("references loop: {}", chain.join(" → ")));
                }
                let target = pointer_get(tree, pointer).ok_or_else(|| format!("`{r}` points at nothing"))?;
                chain.push(r.to_owned());
                let out = self.resolve(target, tree, chain);
                chain.pop();
                out
            }
            Value::Object(o) => o
                .iter()
                .map(|(k, x)| self.resolve(x, tree, chain).map(|r| (k.clone(), r)))
                .collect::<Result<serde_json::Map<_, _>, _>>()
                .map(Value::Object),
            Value::Array(a) => a
                .iter()
                .map(|x| self.resolve(x, tree, chain))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            _ => Ok(v.clone()),
        }
    }

    /// A missing token, with the tokens nearest its id suggested.
    fn missing(&self, id: &str) -> String {
        let near = crate::validate::nearest_few(id, self.by_id.keys().map(String::as_str), 3);
        if near.is_empty() {
            "no such token".into()
        } else {
            format!("no such token; did you mean {}?", listed(near.into_iter()))
        }
    }
}

/// The token a whole-value alias names: `{a.b}` gives `a.b`.
fn alias(s: &str) -> Option<&str> {
    s.strip_prefix('{').and_then(|r| r.strip_suffix('}'))
}

/// `a`, `b` and `c`, backticked, for a message.
fn listed<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let all: Vec<String> = names.map(|n| format!("`{n}`")).collect();
    if all.is_empty() {
        "none".into()
    } else {
        all.join(", ")
    }
}

/// `relative` against the directory of `base`, with `.` and `..` folded:
/// paths as the files name each other, whatever reads them.
#[must_use]
pub fn join(base: &str, relative: &str) -> String {
    let absolute = relative.starts_with('/') || (base.starts_with('/') && !relative.starts_with('/'));
    let mut parts: Vec<&str> = Vec::new();
    if !relative.starts_with('/') {
        parts.extend(base.split('/').filter(|p| !p.is_empty() && *p != "."));
        parts.pop();
    }
    for part in relative.split('/') {
        match part {
            "" | "." => {}
            ".." if parts.last().is_some_and(|p| *p != "..") => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    let joined = parts.join("/");
    if absolute { format!("/{joined}") } else { joined }
}
