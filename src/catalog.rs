// Copyright (c) 2026, Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

use std::{
    collections::BTreeMap,
    fs,
    path::Path,
};

use serde::Deserialize;

use crate::error::{
    Error,
    Result,
};

#[derive(Debug, Deserialize)]
struct CatalogFile {
    kit: Kit,
    defaults: Defaults,
    #[serde(default)]
    presets: BTreeMap<String, Preset>,
    #[serde(default)]
    shapes: BTreeMap<String, Layer>,
    #[serde(default)]
    runtimes: BTreeMap<String, Layer>,
    #[serde(default)]
    orgs: BTreeMap<String, Layer>,
    paper: Layer,
}

#[derive(Debug, Deserialize)]
struct Kit {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Defaults {
    shape: String,
    runtime: String,
    org: String,
}

#[derive(Debug, Deserialize, Default)]
struct Preset {
    #[serde(default)]
    description: String,
    #[serde(default)]
    shape: String,
    #[serde(default)]
    runtime: String,
    #[serde(default)]
    org: String,
}

#[derive(Debug, Deserialize, Default)]
struct Layer {
    #[serde(default)]
    description: String,
    #[serde(default)]
    dirs: Vec<String>,
    #[serde(default)]
    keep: Vec<String>,
    #[serde(default)]
    files: Vec<FileSpec>,
    #[serde(default)]
    copies: Vec<CopySpec>,
    #[serde(default)]
    gitignore_append: Option<String>,
    #[serde(default)]
    vars: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Deserialize)]
struct FileSpec {
    src: String,
    dest: String,
}

#[derive(Debug, Deserialize)]
struct CopySpec {
    src: String,
    dest: String,
    #[serde(default)]
    mode: String,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub kind: ItemKind,
    pub src: String,
    pub dest: String,
    pub mode: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    File,
    Copy,
    Append,
    Keep,
}

impl Item {
    fn key(&self) -> String {
        match self.kind {
            ItemKind::Append => format!("__append__:{}", self.src),
            _ => self.dest.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Resolve<'a> {
    pub shape: &'a str,
    pub runtime: &'a str,
    pub org: &'a str,
    pub github_org: Option<&'a str>,
    pub project_name: &'a str,
}

#[derive(Debug)]
pub struct Resolved {
    pub shape: String,
    pub runtime: String,
    pub org: String,
    pub dirs: Vec<String>,
    pub items: Vec<Item>,
    pub vars: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct Catalog {
    file: CatalogFile,
}

impl Catalog {
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                Error::CatalogMissing(path.to_path_buf())
            } else {
                Error::Io(err)
            }
        })?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self> {
        let file: CatalogFile = serde_yaml::from_str(text)?;
        Ok(Self { file })
    }

    pub fn format_list(&self) -> String {
        let c = &self.file;
        let mut lines = vec![
            format!("{} catalog", c.kit.name),
            String::new(),
            "Defaults:".to_string(),
            format!("  shape    {}", c.defaults.shape),
            format!("  runtime  {}", c.defaults.runtime),
            format!("  org      {}", c.defaults.org),
            String::new(),
            "Shapes:".to_string(),
        ];
        for (name, spec) in &c.shapes {
            lines.push(format!("  {name:<12} {}", spec.description));
        }
        lines.push(String::new());
        lines.push("Runtimes:".to_string());
        for (name, spec) in &c.runtimes {
            lines.push(format!("  {name:<12} {}", spec.description));
        }
        lines.push(String::new());
        lines.push("Orgs:".to_string());
        for (name, spec) in &c.orgs {
            lines.push(format!("  {name:<12} {}", spec.description));
        }
        if !c.presets.is_empty() {
            lines.push(String::new());
            lines.push("Presets:".to_string());
            for (name, spec) in &c.presets {
                lines.push(format!(
                    "  {name:<12} shape={} runtime={} org={}  {}",
                    spec.shape, spec.runtime, spec.org, spec.description
                ));
            }
        }
        lines.push(String::new());
        lines.push("Paper:".to_string());
        lines.push(format!("  {}", c.paper.description));
        lines.push(String::new());
        lines.join("\n")
    }

    /// Empty strings count as unset.
    pub fn apply_preset(
        &self,
        preset: Option<&str>,
        mut shape: Option<&str>,
        mut runtime: Option<&str>,
        mut org: Option<&str>,
    ) -> Result<(String, String, String)> {
        let preset = blank_to_none(preset);
        shape = blank_to_none(shape);
        runtime = blank_to_none(runtime);
        org = blank_to_none(org);

        let mut shape = shape.unwrap_or("").to_string();
        let mut runtime = runtime.unwrap_or("").to_string();
        let mut org = org.unwrap_or("").to_string();

        if let Some(preset) = preset {
            let spec = self.file.presets.get(preset).ok_or_else(|| Error::UnknownId {
                kind: "preset",
                ident: preset.to_string(),
                known: known_keys(&self.file.presets),
            })?;
            if shape.is_empty() {
                shape = spec.shape.clone();
            }
            if runtime.is_empty() {
                runtime = spec.runtime.clone();
            }
            if org.is_empty() {
                org = spec.org.clone();
            }
        }

        if shape.is_empty() {
            shape = self.file.defaults.shape.clone();
        }
        if runtime.is_empty() {
            runtime = self.file.defaults.runtime.clone();
        }
        if org.is_empty() {
            org = self.file.defaults.org.clone();
        }
        Ok((shape, runtime, org))
    }

    pub fn resolve(&self, req: Resolve<'_>) -> Result<Resolved> {
        let shape_spec = require(&self.file.shapes, "shape", req.shape)?;
        let runtime_spec = require(&self.file.runtimes, "runtime", req.runtime)?;
        let org_spec = require(&self.file.orgs, "org", req.org)?;
        let layers = [shape_spec, runtime_spec, org_spec];
        let (dirs, items) = merge_files(&layers);

        let mut vars: BTreeMap<String, String> = BTreeMap::new();
        for layer in layers {
            for (key, value) in &layer.vars {
                vars.insert(key.clone(), value.clone().unwrap_or_default());
            }
        }

        let github_org = blank_to_none(req.github_org).unwrap_or("");
        if !github_org.is_empty() {
            vars.insert("GITHUB_ORG".to_string(), github_org.to_string());
        }
        fill_repo_urls(&mut vars, req.project_name);

        Ok(Resolved {
            shape: req.shape.to_string(),
            runtime: req.runtime.to_string(),
            org: req.org.to_string(),
            dirs,
            items,
            vars,
        })
    }

    pub fn paper(&self) -> (Vec<String>, Vec<Item>) {
        merge_files(&[&self.file.paper])
    }
}

pub fn fill_repo_urls(vars: &mut BTreeMap<String, String>, project_name: &str) {
    let ghorg = vars.get("GITHUB_ORG").cloned().unwrap_or_default();
    if ghorg.is_empty() {
        vars.insert("CLONE_SNIPPET".to_string(), "git clone <repository-url>".to_string());
        vars.insert("REPO_WEB".to_string(), "the project repository".to_string());
        return;
    }
    vars.insert(
        "CLONE_SNIPPET".to_string(),
        format!("git clone https://github.com/{ghorg}/{project_name}.git"),
    );
    vars.insert(
        "REPO_WEB".to_string(),
        format!("https://github.com/{ghorg}/{project_name}"),
    );
}

fn blank_to_none(value: Option<&str>) -> Option<&str> {
    value.filter(|s| !s.is_empty())
}

fn known_keys<T>(map: &BTreeMap<String, T>) -> String {
    if map.is_empty() {
        "(none)".to_string()
    } else {
        map.keys().cloned().collect::<Vec<_>>().join(", ")
    }
}

fn require<'a>(section: &'a BTreeMap<String, Layer>, kind: &'static str, ident: &str) -> Result<&'a Layer> {
    section.get(ident).ok_or_else(|| Error::UnknownId {
        kind,
        ident: ident.to_string(),
        known: known_keys(section),
    })
}

fn merge_files(layers: &[&Layer]) -> (Vec<String>, Vec<Item>) {
    let mut dirs = Vec::new();
    let mut items: Vec<Item> = Vec::new();
    for layer in layers {
        for dir in &layer.dirs {
            if !dirs.iter().any(|d| d == dir) {
                dirs.push(dir.clone());
            }
        }
        for file in &layer.files {
            upsert(
                &mut items,
                Item {
                    kind: ItemKind::File,
                    src: file.src.clone(),
                    dest: file.dest.clone(),
                    mode: String::new(),
                },
            );
        }
        for copy in &layer.copies {
            upsert(
                &mut items,
                Item {
                    kind: ItemKind::Copy,
                    src: copy.src.clone(),
                    dest: copy.dest.clone(),
                    mode: copy.mode.clone(),
                },
            );
        }
        if let Some(frag) = &layer.gitignore_append {
            upsert(
                &mut items,
                Item {
                    kind: ItemKind::Append,
                    src: frag.clone(),
                    dest: ".gitignore".to_string(),
                    mode: String::new(),
                },
            );
        }
        for keep in &layer.keep {
            upsert(
                &mut items,
                Item {
                    kind: ItemKind::Keep,
                    src: String::new(),
                    dest: keep.clone(),
                    mode: String::new(),
                },
            );
        }
    }
    (dirs, items)
}

fn upsert(items: &mut Vec<Item>, item: Item) {
    let key = item.key();
    if let Some(existing) = items.iter_mut().find(|i| i.key() == key) {
        *existing = item;
    } else {
        items.push(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_catalog() -> Catalog {
        Catalog::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("catalog.yaml")).unwrap()
    }

    #[test]
    fn list_names_the_default_lab_org() {
        let text = repo_catalog().format_list();
        assert!(text.contains("shape    paper-line"), "{text}");
        assert!(text.contains("runtime  uv"), "{text}");
        assert!(text.contains("org      lab"), "{text}");
        assert!(text.contains("Default lab overlay"), "{text}");
        assert!(!text.contains("csymd"), "{text}");
        assert!(text.contains("Paper:"), "{text}");
    }

    #[test]
    fn default_lab_org_fills_org_vars() {
        let cat = repo_catalog();
        let (shape, runtime, org) = cat.apply_preset(None, None, None, None).unwrap();
        assert_eq!(shape, "paper-line");
        assert_eq!(runtime, "uv");
        assert_eq!(org, "lab");
        let resolved = cat
            .resolve(Resolve {
                shape: &shape,
                runtime: &runtime,
                org: &org,
                github_org: None,
                project_name: "chp-coupling",
            })
            .unwrap();
        assert_eq!(resolved.vars["GITHUB_ORG"], "lab-org");
        assert_eq!(resolved.vars["LAB_URL"], "https://lab.example");
        assert_eq!(resolved.vars["LAB_NAME"], "Lab name");
        assert!(resolved.vars["CLONE_SNIPPET"].contains("lab-org/chp-coupling"));
        assert!(resolved.vars["LAB_LINE"].contains("lab.example"));
        assert!(resolved.vars["RUNTIME_NOTE"].contains("uv sync"));
        assert!(resolved.items.iter().any(|i| i.dest == "pyproject.toml"));
        assert!(resolved.items.iter().any(|i| i.dest == "src/__PACKAGE__/__init__.py"));
        assert!(!resolved.dirs.iter().any(|d| d == "analysis" || d.starts_with("docs")));
    }

    #[test]
    fn explicit_runtime_keeps_the_default_lab_org() {
        let cat = repo_catalog();
        let (shape, runtime, org) = cat.apply_preset(None, None, Some("none"), None).unwrap();
        assert_eq!(runtime, "none");
        assert_eq!(shape, "paper-line");
        assert_eq!(org, "lab");
        let resolved = cat
            .resolve(Resolve {
                shape: &shape,
                runtime: &runtime,
                org: &org,
                github_org: None,
                project_name: "demo",
            })
            .unwrap();
        assert!(!resolved.items.iter().any(|i| i.dest == "pyproject.toml"));
        assert_eq!(resolved.vars["RUNTIME_NOTE"], "");
        assert!(resolved.vars["LAB_LINE"].contains("lab.example"));
    }

    #[test]
    fn preset_fills_empty_flags_and_explicit_flag_wins() {
        let cat = Catalog::parse(
            r#"
version: 1
kit:
  name: symproject
defaults:
  shape: paper-line
  runtime: uv
  org: lab
presets:
  demo:
    description: demo combo
    shape: paper-line
    runtime: uv
    org: lab
shapes: {}
runtimes: {}
orgs: {}
paper:
  description: paper
"#,
        )
        .unwrap();
        let (shape, runtime, org) = cat.apply_preset(Some("demo"), None, None, None).unwrap();
        assert_eq!(shape, "paper-line");
        assert_eq!(runtime, "uv");
        assert_eq!(org, "lab");
        let (shape, runtime, org) = cat.apply_preset(Some("demo"), None, Some("none"), None).unwrap();
        assert_eq!(runtime, "none");
        assert_eq!(shape, "paper-line");
        assert_eq!(org, "lab");
        let err = cat.apply_preset(Some("missing"), None, None, None).unwrap_err();
        assert!(err.to_string().contains("unknown --preset 'missing'"), "{err}");
        assert!(err.to_string().contains("demo"), "{err}");
    }

    #[test]
    fn org_none_leaves_the_lab_line_blank() {
        let resolved = repo_catalog()
            .resolve(Resolve {
                shape: "paper-line",
                runtime: "none",
                org: "none",
                github_org: None,
                project_name: "demo",
            })
            .unwrap();
        assert_eq!(resolved.vars["LAB_LINE"], "");
        assert_eq!(resolved.vars["CLONE_SNIPPET"], "git clone <repository-url>");
        assert_eq!(resolved.vars["REPO_WEB"], "the project repository");
    }

    #[test]
    fn github_org_override() {
        let cat = repo_catalog();
        let resolved = cat
            .resolve(Resolve {
                shape: "paper-line",
                runtime: "none",
                org: "none",
                github_org: Some("example-lab"),
                project_name: "demo-study",
            })
            .unwrap();
        assert_eq!(
            resolved.vars["CLONE_SNIPPET"],
            "git clone https://github.com/example-lab/demo-study.git"
        );
        assert_eq!(resolved.vars["REPO_WEB"], "https://github.com/example-lab/demo-study");
    }

    #[test]
    fn unknown_shape_names_known_ids() {
        let err = repo_catalog()
            .resolve(Resolve {
                shape: "nope",
                runtime: "none",
                org: "none",
                github_org: None,
                project_name: "x",
            })
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown --shape 'nope'"), "{msg}");
        assert!(msg.contains("paper-line"), "{msg}");
    }

    #[test]
    fn paper_destinations_use_slug_token() {
        let (dirs, items) = repo_catalog().paper();
        assert!(dirs.iter().any(|d| d == "papers/__SLUG__/analysis"));
        assert!(items.iter().any(|i| i.dest == "papers/__SLUG__/notes/claims.md"));
        assert!(items.iter().any(|i| i.dest == "results/__SLUG__/.gitkeep"));
    }
}
