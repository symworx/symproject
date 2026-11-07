// Copyright (c) 2026, Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Lab identity (`templates/lab.yml`) and the holdings recorded in a project.

use std::{
    collections::BTreeMap,
    fs,
    path::Path,
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    catalog::fill_repo_urls,
    error::{
        Error,
        Result,
    },
};

const MIN_HOLDING_CHARS: usize = 8;

/// Files `set-lab` is allowed to rewrite. Order is the sidecar order.
pub const PROJECT_FILES: &[&str] = &["README.md", "CONTRIBUTING.md"];

const HOLDING_FILES: &[(&str, &[&str])] = &[
    ("CLONE_SNIPPET", &["README.md"]),
    ("ISSUE_LINE", &["README.md", "CONTRIBUTING.md"]),
    ("LAB_LINE", &["README.md"]),
    ("REPO_WEB", &["README.md"]),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabFile {
    pub institution_name: Option<String>,
    pub github_org: String,
    /// Org page. When absent, clone URLs use `https://github.com/<github_org>`.
    pub github_url: Option<String>,
    pub lab_name: String,
    pub lab_url: String,
    pub lab_line: Option<String>,
    pub issue_line: Option<String>,
    pub investigators: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sidecar {
    pub version: u32,
    pub project_name: String,
    pub org: String,
    pub holdings: BTreeMap<String, String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabPlan {
    pub files: BTreeMap<String, String>,
    pub holdings: BTreeMap<String, String>,
    pub notes: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LabRaw {
    version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    institution_name: Option<String>,
    github_org: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    github_url: Option<String>,
    lab_name: String,
    lab_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lab_line: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    issue_line: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    investigators: Vec<String>,
}

impl LabFile {
    pub fn rendered_lab_line(&self) -> String {
        match &self.lab_line {
            Some(line) => line.clone(),
            None => match &self.institution_name {
                Some(institution) => format!("Lab: [{}]({}), {institution}.", self.lab_name, self.lab_url),
                None => format!("Lab: [{}]({}).", self.lab_name, self.lab_url),
            },
        }
    }
}

pub fn save_lab(path: &Path, lab: &LabFile) -> Result<()> {
    let raw = LabRaw {
        version: 1,
        institution_name: lab.institution_name.clone(),
        github_org: lab.github_org.clone(),
        github_url: lab.github_url.clone(),
        lab_name: lab.lab_name.clone(),
        lab_url: lab.lab_url.clone(),
        lab_line: lab.lab_line.clone(),
        issue_line: lab.issue_line.clone(),
        investigators: lab.investigators.clone(),
    };
    let body = serde_yaml::to_string(&raw).map_err(|err| Error::LabInvalid {
        path: path.to_path_buf(),
        message: err.to_string(),
    })?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let text = format!("# Local lab file. Pass it to `symproject new --lab` or `set-lab`.\n{body}");
    fs::write(path, text)?;
    Ok(())
}

pub fn load_lab(path: &Path) -> Result<LabFile> {
    let text = fs::read_to_string(path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            Error::LabMissing(path.to_path_buf())
        } else {
            Error::Io(err)
        }
    })?;
    parse_lab(path, &text)
}

pub fn load_sidecar(path: &Path) -> Result<Sidecar> {
    let text = fs::read_to_string(path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            Error::SidecarMissing(path.to_path_buf())
        } else {
            Error::Io(err)
        }
    })?;
    parse_sidecar(&text)
}

pub fn write_sidecar(path: &Path, sidecar: &Sidecar) -> Result<()> {
    let text = serde_yaml::to_string(sidecar).map_err(|err| Error::SidecarInvalid(err.to_string()))?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, text)?;
    Ok(())
}

pub fn holdings_from_vars(vars: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut holdings = BTreeMap::new();
    for key in ["CLONE_SNIPPET", "ISSUE_LINE", "LAB_LINE", "REPO_WEB"] {
        if let Some(value) = vars.get(key).filter(|value| !value.is_empty()) {
            holdings.insert(key.to_string(), value.clone());
        }
    }
    holdings
}

pub fn overlay_lab(vars: &mut BTreeMap<String, String>, lab: &LabFile, project_name: &str) {
    vars.insert("GITHUB_ORG".to_string(), lab.github_org.clone());
    if let Some(url) = &lab.github_url {
        vars.insert("GITHUB_URL".to_string(), url.clone());
    }
    vars.insert("LAB_NAME".to_string(), lab.lab_name.clone());
    vars.insert("LAB_URL".to_string(), lab.lab_url.clone());
    vars.insert("LAB_LINE".to_string(), lab.rendered_lab_line());
    if let Some(issue) = &lab.issue_line {
        vars.insert("ISSUE_LINE".to_string(), issue.clone());
    }
    fill_repo_urls(vars, project_name);
}

pub fn next_holdings(
    lab: &LabFile,
    project_name: &str,
    current: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut vars = BTreeMap::new();
    vars.insert("GITHUB_ORG".to_string(), lab.github_org.clone());
    if let Some(url) = &lab.github_url {
        vars.insert("GITHUB_URL".to_string(), url.clone());
    }
    fill_repo_urls(&mut vars, project_name);
    let mut next = BTreeMap::new();
    next.insert("LAB_LINE".to_string(), lab.rendered_lab_line());
    if let Some(issue) = &lab.issue_line {
        next.insert("ISSUE_LINE".to_string(), issue.clone());
    } else if let Some(issue) = current.get("ISSUE_LINE") {
        next.insert("ISSUE_LINE".to_string(), issue.clone());
    }
    if let Some(value) = vars.get("REPO_WEB") {
        next.insert("REPO_WEB".to_string(), value.clone());
    }
    if let Some(value) = vars.get("CLONE_SNIPPET") {
        next.insert("CLONE_SNIPPET".to_string(), value.clone());
    }
    next
}

pub fn plan_lab(current: &BTreeMap<String, String>, sidecar: &Sidecar, lab: &LabFile) -> Result<LabPlan> {
    if sidecar.org == "none" {
        return Err(Error::OrgNotReplaceable(sidecar.org.clone()));
    }
    if !sidecar.holdings.contains_key("LAB_LINE") {
        return Err(Error::SidecarInvalid("missing LAB_LINE holding".to_string()));
    }
    let next = next_holdings(lab, &sidecar.project_name, &sidecar.holdings);
    let mut warnings = Vec::new();
    let mut applied: Vec<&str> = Vec::new();

    for (key, _) in HOLDING_FILES {
        let Some(old) = sidecar.holdings.get(*key) else {
            continue;
        };
        let Some(new) = next.get(*key) else {
            continue;
        };
        if old == new {
            continue;
        }
        assert_searchable(key, old, &sidecar.project_name)?;
        assert_searchable(key, new, &sidecar.project_name)?;
        let targets = target_files(key, &sidecar.files);
        if targets.is_empty() {
            warnings.push(format!("{key} is not listed in .symproject.yaml files; left unchanged"));
            continue;
        }
        let missing: Vec<&str> = targets
            .iter()
            .copied()
            .filter(|file| {
                current
                    .get(*file)
                    .map(|text| !text.contains(old.as_str()))
                    .unwrap_or(true)
            })
            .collect();
        if !missing.is_empty() {
            warnings.push(format!("{key} not found in {}; left unchanged", missing.join(", ")));
            continue;
        }
        applied.push(key);
    }

    let mut files = current.clone();
    for file in &sidecar.files {
        let Some(text) = files.get_mut(file) else {
            continue;
        };
        let mut reps: Vec<(String, String)> = applied
            .iter()
            .copied()
            .filter(|key| files_for(key).contains(&file.as_str()))
            .filter_map(|key| Some((sidecar.holdings.get(key)?.clone(), next.get(key)?.clone())))
            .collect();
        // Longer holdings first so a clone URL is not rewritten as a bare repo URL.
        reps.sort_by(|left, right| right.0.len().cmp(&left.0.len()).then(left.0.cmp(&right.0)));
        for (old, new) in reps {
            *text = text.replace(&old, &new);
        }
    }

    let mut holdings = sidecar.holdings.clone();
    for key in &applied {
        if let Some(value) = next.get(*key) {
            holdings.insert((*key).to_string(), value.clone());
        }
    }

    let mut notes = Vec::new();
    for file in &sidecar.files {
        for (key, _) in HOLDING_FILES {
            if applied.contains(key) && files_for(key).contains(&file.as_str()) {
                notes.push(format!("{file}: {key}"));
            }
        }
    }

    Ok(LabPlan {
        files,
        holdings,
        notes,
        warnings,
    })
}

fn parse_lab(path: &Path, text: &str) -> Result<LabFile> {
    let raw: LabRaw = serde_yaml::from_str(text).map_err(|err| Error::LabInvalid {
        path: path.to_path_buf(),
        message: err.to_string(),
    })?;
    if raw.version != 1 {
        return Err(Error::LabInvalid {
            path: path.to_path_buf(),
            message: format!("unsupported version {} (expected 1)", raw.version),
        });
    }
    let institution_name = match raw.institution_name {
        Some(value) => Some(require_line(path, "institution_name", &value)?),
        None => None,
    };
    let github_org = require_line(path, "github_org", &raw.github_org)?;
    if !valid_github_org(&github_org) {
        return Err(Error::LabInvalid {
            path: path.to_path_buf(),
            message: "github_org must be a GitHub name: letters, digits, '-' or '_'".to_string(),
        });
    }
    let github_url = match raw.github_url {
        Some(value) => Some(canonical_org_url(path, &value, Some(&github_org))?),
        None => None,
    };
    let lab_name = require_line(path, "lab_name", &raw.lab_name)?;
    let lab_url = require_line(path, "lab_url", &raw.lab_url)?;
    if !valid_http_url(&lab_url) {
        return Err(Error::LabInvalid {
            path: path.to_path_buf(),
            message: "lab_url must be an http(s) URL".to_string(),
        });
    }
    let lab_line = match raw.lab_line {
        Some(value) => Some(require_line(path, "lab_line", &value)?),
        None => None,
    };
    let issue_line = match raw.issue_line {
        Some(value) => Some(require_line(path, "issue_line", &value)?),
        None => None,
    };
    let mut investigators = Vec::new();
    for (index, name) in raw.investigators.into_iter().enumerate() {
        let name = require_line(path, "investigators", &name).map_err(|err| match err {
            Error::LabInvalid { path, .. } => Error::LabInvalid {
                path,
                message: format!("investigators[{index}] must be a single non-empty line"),
            },
            other => other,
        })?;
        if investigators.iter().any(|have: &String| have == &name) {
            return Err(Error::LabInvalid {
                path: path.to_path_buf(),
                message: format!("duplicate investigator '{name}'"),
            });
        }
        investigators.push(name);
    }
    if lab_line.is_none() && lab_name.chars().any(|ch| matches!(ch, '[' | ']' | '(' | ')')) {
        return Err(Error::LabInvalid {
            path: path.to_path_buf(),
            message: "lab_name cannot contain brackets when lab_line is omitted".to_string(),
        });
    }
    Ok(LabFile {
        institution_name,
        github_org,
        github_url,
        lab_name,
        lab_url,
        lab_line,
        issue_line,
        investigators,
    })
}

fn parse_sidecar(text: &str) -> Result<Sidecar> {
    let sidecar: Sidecar = serde_yaml::from_str(text).map_err(|err| Error::SidecarInvalid(err.to_string()))?;
    if sidecar.version != 1 {
        return Err(Error::SidecarInvalid(format!(
            "unsupported version {} (expected 1)",
            sidecar.version
        )));
    }
    if sidecar.project_name.is_empty() {
        return Err(Error::SidecarInvalid("project_name is empty".to_string()));
    }
    if sidecar.files.is_empty() {
        return Err(Error::SidecarInvalid("files list is empty".to_string()));
    }
    Ok(sidecar)
}

fn require_line(path: &Path, field: &str, value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.contains(['\n', '\r']) {
        return Err(Error::LabInvalid {
            path: path.to_path_buf(),
            message: format!("{field} must be a single non-empty line"),
        });
    }
    Ok(value.to_string())
}

/// `github_org` is a GitHub name. A pasted `https://host/<org>` URL is accepted.
/// `url_input` overrides the org page. An empty URL becomes `https://github.com/<org>`.
pub fn github_name_and_url(org_input: &str, url_input: Option<&str>) -> Result<(String, String)> {
    let org_input = org_input.trim();
    if org_input.contains('@') && !org_input.contains("://") {
        return Err(Error::Msg(
            "GitHub org is a name, or an http(s) URL such as https://github.com/<org>".into(),
        ));
    }
    let (name, derived) = if is_github_url(org_input) {
        let url = org_url(org_input, None)?;
        let name = org_segment(&url)?;
        (name, url)
    } else if !valid_github_org(org_input) {
        return Err(Error::Msg(
            "GitHub org must be a name (letters, digits, '-' or '_'), or an http(s) URL".into(),
        ));
    } else {
        (org_input.to_string(), format!("https://github.com/{org_input}"))
    };
    let url = match url_input.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => org_url(value, Some(&name))?,
        None => derived,
    };
    Ok((name, url))
}

pub fn assign_github(lab: &mut LabFile, org_input: Option<&str>, url_input: Option<&str>) -> Result<()> {
    let org_input = org_input.map(str::trim).filter(|value| !value.is_empty());
    let url_input = url_input.map(str::trim).filter(|value| !value.is_empty());
    if org_input.is_none() && url_input.is_none() {
        return Ok(());
    }
    if let Some(value) = org_input.filter(|value| is_github_url(value)) {
        let (name, url) = github_name_and_url(value, url_input)?;
        lab.github_org = name;
        lab.github_url = Some(url);
        return Ok(());
    }
    let name = match org_input {
        Some(value) => github_name_and_url(value, None)?.0,
        None => lab.github_org.clone(),
    };
    let url = match url_input {
        Some(value) => github_name_and_url(&name, Some(value))?.1,
        None if name == lab.github_org => lab
            .github_url
            .clone()
            .unwrap_or_else(|| format!("https://github.com/{name}")),
        None => {
            let previous = lab
                .github_url
                .clone()
                .unwrap_or_else(|| format!("https://github.com/{}", lab.github_org));
            if let Some(prefix) = previous.strip_suffix(&format!("/{}", lab.github_org)) {
                format!("{prefix}/{name}")
            } else {
                format!("https://github.com/{name}")
            }
        }
    };
    let (name, url) = github_name_and_url(&name, Some(&url))?;
    lab.github_org = name;
    lab.github_url = Some(url);
    Ok(())
}

fn org_url(value: &str, expected_org: Option<&str>) -> Result<String> {
    canonical_org_url(Path::new("github_url"), value, expected_org).map_err(|err| match err {
        Error::LabInvalid { message, .. } => Error::Msg(message),
        other => other,
    })
}

pub fn is_github_url(value: &str) -> bool {
    let value = value.trim();
    value.contains("://") || value.starts_with("github.com/") || value.starts_with("www.github.com/")
}

fn org_segment(url: &str) -> Result<String> {
    url.rsplit('/')
        .next()
        .filter(|segment| valid_github_org(segment))
        .map(str::to_string)
        .ok_or_else(|| Error::Msg("GitHub URL needs an org name".into()))
}

fn canonical_org_url(path: &Path, value: &str, expected_org: Option<&str>) -> Result<String> {
    let mut text = value.trim().trim_end_matches('/').to_string();
    if !text.contains("://") {
        if text.starts_with("github.com/") || text.starts_with("www.github.com/") {
            text = format!("https://{text}");
        } else {
            return Err(Error::LabInvalid {
                path: path.to_path_buf(),
                message: "GitHub URL must be an http(s) URL".into(),
            });
        }
    }
    let scheme = if let Some(rest) = text.strip_prefix("https://") {
        rest
    } else if let Some(rest) = text.strip_prefix("http://") {
        rest
    } else {
        return Err(Error::LabInvalid {
            path: path.to_path_buf(),
            message: "GitHub URL must be an http(s) URL".into(),
        });
    };
    if scheme.chars().any(char::is_whitespace) {
        return Err(Error::LabInvalid {
            path: path.to_path_buf(),
            message: "GitHub URL must be an http(s) URL".into(),
        });
    }
    let Some((host, path_part)) = scheme.split_once('/') else {
        return Err(Error::LabInvalid {
            path: path.to_path_buf(),
            message: "GitHub URL needs an org name".into(),
        });
    };
    if host.is_empty() {
        return Err(Error::LabInvalid {
            path: path.to_path_buf(),
            message: "GitHub URL needs a host".into(),
        });
    }
    let first = path_part.split('/').next().unwrap_or("");
    let segment = first.strip_suffix(".git").unwrap_or(first);
    if !valid_github_org(segment) {
        return Err(Error::LabInvalid {
            path: path.to_path_buf(),
            message: "GitHub URL needs an org name".into(),
        });
    }
    if let Some(expected) = expected_org {
        if segment != expected {
            return Err(Error::LabInvalid {
                path: path.to_path_buf(),
                message: format!("GitHub URL must use org '{expected}'"),
            });
        }
    }
    let scheme_name = if text.starts_with("http://") { "http" } else { "https" };
    Ok(format!("{scheme_name}://{host}/{segment}"))
}

fn valid_github_org(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(ch) if ch.is_ascii_alphanumeric() => chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'),
        _ => false,
    }
}

fn valid_http_url(value: &str) -> bool {
    let rest = value.strip_prefix("https://").or_else(|| value.strip_prefix("http://"));
    match rest {
        Some(rest) => !rest.is_empty() && !rest.chars().any(|ch| ch.is_whitespace()),
        None => false,
    }
}

fn files_for(key: &str) -> &'static [&'static str] {
    HOLDING_FILES
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, files)| *files)
        .unwrap_or(&[])
}

fn target_files(key: &str, allow: &[String]) -> Vec<&'static str> {
    files_for(key)
        .iter()
        .copied()
        .filter(|name| allow.iter().any(|listed| listed == *name))
        .collect()
}

fn assert_searchable(key: &str, value: &str, project_name: &str) -> Result<()> {
    if value.chars().count() < MIN_HOLDING_CHARS || value == project_name || value.contains(['\n', '\r']) {
        return Err(Error::UnsafeHolding(key.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::{
        Path,
        PathBuf,
    };

    use super::*;
    use crate::catalog::{
        Catalog,
        Resolve,
    };

    fn sample_sidecar(lab_line: &str) -> Sidecar {
        let mut holdings = BTreeMap::new();
        holdings.insert(
            "CLONE_SNIPPET".to_string(),
            "git clone https://github.com/lab-org/demo-study.git".to_string(),
        );
        holdings.insert(
            "ISSUE_LINE".to_string(),
            "Track each paper in whatever tracker you already use.".to_string(),
        );
        holdings.insert("LAB_LINE".to_string(), lab_line.to_string());
        holdings.insert(
            "REPO_WEB".to_string(),
            "https://github.com/lab-org/demo-study".to_string(),
        );
        Sidecar {
            version: 1,
            project_name: "demo-study".to_string(),
            org: "lab".to_string(),
            holdings,
            files: vec!["README.md".to_string(), "CONTRIBUTING.md".to_string()],
        }
    }

    fn other_lab(issue: Option<&str>) -> LabFile {
        LabFile {
            institution_name: None,
            github_org: "other-org".to_string(),
            github_url: None,
            lab_name: "Other Lab".to_string(),
            lab_url: "https://other.example".to_string(),
            lab_line: None,
            issue_line: issue.map(str::to_string),
            investigators: Vec::new(),
        }
    }

    fn readme_for(lab_line: &str, issue: &str) -> String {
        format!(
            "# Demo\n\n{lab_line}\n\n{issue}\n\nRepository: https://github.com/lab-org/demo-study\n\n```bash\ngit clone https://github.com/lab-org/demo-study.git\n```\n\nKeep this sentence.\n"
        )
    }

    #[test]
    fn template_matches_the_lab_org() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let lab = load_lab(&root.join("templates/lab.yml")).unwrap();
        let cat = Catalog::load(&root.join("catalog.yaml")).unwrap();
        let (shape, runtime, org) = cat.apply_preset(None, None, None, Some("lab")).unwrap();
        assert_eq!(shape, "paper-line");
        assert_eq!(runtime, "uv");
        assert_eq!(org, "lab");
        let resolved = cat
            .resolve(Resolve {
                shape: &shape,
                runtime: &runtime,
                org: &org,
                github_org: None,
                project_name: "demo-study",
            })
            .unwrap();
        assert_eq!(resolved.vars["GITHUB_ORG"], lab.github_org);
        assert_eq!(resolved.vars["LAB_NAME"], lab.lab_name);
        assert_eq!(resolved.vars["LAB_URL"], lab.lab_url);
        assert_eq!(resolved.vars["LAB_LINE"], lab.rendered_lab_line());
        assert_eq!(resolved.vars["ISSUE_LINE"], lab.issue_line.as_deref().unwrap());
        assert_eq!(resolved.vars["REPO_WEB"], "https://github.com/lab-org/demo-study");
        assert_eq!(
            resolved.vars["CLONE_SNIPPET"],
            "git clone https://github.com/lab-org/demo-study.git"
        );
    }

    #[test]
    fn omitted_lab_line_is_derived_and_blank_line_is_rejected() {
        let lab = parse_lab(
            Path::new("lab.yml"),
            "version: 1\ngithub_org: lab-org\nlab_name: Lab name\nlab_url: https://lab.example\n",
        )
        .unwrap();
        assert_eq!(lab.rendered_lab_line(), "Lab: [Lab name](https://lab.example).");
        assert!(lab.github_url.is_none());
        assert!(lab.issue_line.is_none());
        let (name, url) = github_name_and_url("https://github.com/Cog-Neuro/some-repo", None).unwrap();
        assert_eq!(name, "Cog-Neuro");
        assert_eq!(url, "https://github.com/Cog-Neuro");
        let (name, url) = github_name_and_url("cog-neuro", Some("https://git.example.edu/cog-neuro/")).unwrap();
        assert_eq!(name, "cog-neuro");
        assert_eq!(url, "https://git.example.edu/cog-neuro");
        let err = github_name_and_url("cog-neuro", Some("https://github.com/other")).unwrap_err();
        assert!(err.to_string().contains("must use org 'cog-neuro'"), "{err}");
        let mut custom = other_lab(None);
        custom.github_org = "cog-neuro".to_string();
        custom.github_url = Some("https://git.example.edu/cog-neuro".to_string());
        let mut vars = BTreeMap::new();
        overlay_lab(&mut vars, &custom, "demo-study");
        assert_eq!(vars["REPO_WEB"], "https://git.example.edu/cog-neuro/demo-study");
        assert_eq!(
            vars["CLONE_SNIPPET"],
            "git clone https://git.example.edu/cog-neuro/demo-study.git"
        );
        let err = parse_lab(
            Path::new("lab.yml"),
            "version: 1\ngithub_org: lab-org\nlab_name: Lab name\nlab_url: https://lab.example\nlab_line: \"\"\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("lab_line"), "{err}");
        let err = parse_lab(
            Path::new("lab.yml"),
            "version: 2\ngithub_org: lab-org\nlab_name: Lab name\nlab_url: https://lab.example\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("unsupported version"), "{err}");
    }

    #[test]
    fn sidecar_roundtrip() {
        let sidecar = sample_sidecar("Lab: [Lab name](https://lab.example).");
        let text = serde_yaml::to_string(&sidecar).unwrap();
        assert_eq!(parse_sidecar(&text).unwrap(), sidecar);
    }

    #[test]
    fn longer_holding_wins_when_the_lab_line_contains_the_repo_url() {
        let lab_line = "Lab: [Lab name](https://github.com/lab-org/demo-study).";
        let issue = "Track each paper in whatever tracker you already use.";
        let sidecar = sample_sidecar(lab_line);
        let mut files = BTreeMap::new();
        files.insert("README.md".to_string(), readme_for(lab_line, issue));
        files.insert("CONTRIBUTING.md".to_string(), format!("Intro\n\n{issue}\n"));
        let plan = plan_lab(&files, &sidecar, &other_lab(Some("Tickets live in the lab tracker."))).unwrap();
        let readme = &plan.files["README.md"];
        assert!(readme.contains("Lab: [Other Lab](https://other.example)."), "{readme}");
        assert!(
            readme.contains("Repository: https://github.com/other-org/demo-study\n"),
            "{readme}"
        );
        assert!(
            readme.contains("git clone https://github.com/other-org/demo-study.git\n"),
            "{readme}"
        );
        assert!(readme.contains("Keep this sentence."), "{readme}");
        assert!(readme.contains("Tickets live in the lab tracker."), "{readme}");
        assert!(!readme.contains("lab-org"), "{readme}");
        assert!(
            !readme.contains("Lab: [Lab name](https://github.com/other-org/demo-study)."),
            "{readme}"
        );
        assert!(plan.files["CONTRIBUTING.md"].contains("Tickets live in the lab tracker."));
        assert_eq!(plan.holdings["LAB_LINE"], "Lab: [Other Lab](https://other.example).");
        assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);
    }

    #[test]
    fn omitted_issue_line_keeps_the_recorded_sentence() {
        let lab_line = "Lab: [Lab name](https://lab.example).";
        let issue = "Track each paper in whatever tracker you already use.";
        let sidecar = sample_sidecar(lab_line);
        let mut files = BTreeMap::new();
        files.insert("README.md".to_string(), readme_for(lab_line, issue));
        files.insert("CONTRIBUTING.md".to_string(), format!("Intro\n\n{issue}\n"));
        let plan = plan_lab(&files, &sidecar, &other_lab(None)).unwrap();
        assert!(plan.files["CONTRIBUTING.md"].contains(issue));
        assert!(!plan.notes.iter().any(|note| note.contains("ISSUE_LINE")));
        assert_eq!(plan.holdings["ISSUE_LINE"], issue);
    }

    #[test]
    fn missing_lab_line_is_not_rewritten() {
        let lab_line = "Lab: [Lab name](https://lab.example).";
        let issue = "Track each paper in whatever tracker you already use.";
        let sidecar = sample_sidecar(lab_line);
        let mut files = BTreeMap::new();
        files.insert(
            "README.md".to_string(),
            readme_for("Edited lab sentence for this repo.", issue),
        );
        files.insert("CONTRIBUTING.md".to_string(), format!("Intro\n\n{issue}\n"));
        let plan = plan_lab(&files, &sidecar, &other_lab(Some("Tickets live in the lab tracker."))).unwrap();
        assert!(plan.files["README.md"].contains("Edited lab sentence for this repo."));
        assert!(!plan.files["README.md"].contains("other.example"));
        assert!(plan.files["CONTRIBUTING.md"].contains("Tickets live in the lab tracker."));
        assert_eq!(plan.holdings["LAB_LINE"], lab_line);
        assert!(
            plan.warnings.iter().any(|warning| warning.contains("LAB_LINE")),
            "{:?}",
            plan.warnings
        );
    }

    #[test]
    fn neutral_org_and_short_holding_fail() {
        let mut sidecar = sample_sidecar("Lab: [Lab name](https://lab.example).");
        sidecar.org = "none".to_string();
        let err = plan_lab(&BTreeMap::new(), &sidecar, &other_lab(None)).unwrap_err();
        assert!(err.to_string().contains("no replaceable lab line"), "{err}");

        let mut sidecar = sample_sidecar("short");
        sidecar.holdings.insert("LAB_LINE".to_string(), "short".to_string());
        let err = plan_lab(&BTreeMap::new(), &sidecar, &other_lab(None)).unwrap_err();
        assert!(err.to_string().contains("refusing to replace"), "{err}");
    }
}
