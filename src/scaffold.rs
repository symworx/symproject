// Copyright (c) 2026, Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{
        Component,
        Path,
        PathBuf,
    },
    process::{
        Command,
        Stdio,
    },
};

use crate::{
    assets::is_root,
    catalog::{
        self,
        Catalog,
        Item,
        ItemKind,
        Resolve,
    },
    error::{
        Error,
        Result,
    },
    lab,
};

const UV_GITIGNORE_MARK: &str = "BEGIN symproject runtime uv";
const MIG_BEGIN: &str = "# BEGIN symproject migration-docs";
const MIG_END: &str = "# END symproject migration-docs";

pub struct NewProject {
    pub project_name: String,
    pub outdir: PathBuf,
    pub title: String,
    pub package: Option<String>,
    pub shape: Option<String>,
    pub runtime: Option<String>,
    pub org: Option<String>,
    pub preset: Option<String>,
    pub github_org: Option<String>,
    pub lab: Option<PathBuf>,
    pub paper: Option<String>,
    pub paper_title: Option<String>,
    pub with_agents: bool,
    pub adapters: Option<String>,
    pub migration_docs: bool,
}

pub fn scaffold(root: &Path, req: &NewProject) -> Result<()> {
    if !valid_slug(&req.project_name) {
        return Err(Error::InvalidProjectName(req.project_name.clone()));
    }
    if req.title.is_empty() {
        return Err(Error::MissingTitle);
    }
    let package = match req.package.as_deref().filter(|s| !s.is_empty()) {
        Some(name) => {
            if !valid_package(name) {
                return Err(Error::InvalidPackageName(name.to_string()));
            }
            name.to_string()
        }
        None => {
            let derived = req.project_name.replace('-', "_");
            if !valid_package(&derived) {
                return Err(Error::PackageNeedsFlag(req.project_name.clone()));
            }
            derived
        }
    };
    if let Some(slug) = req.paper.as_deref().filter(|s| !s.is_empty()) {
        if !valid_slug(slug) {
            return Err(Error::InvalidPaperSlug(slug.to_string()));
        }
    }
    refuse_kit(root, &req.outdir)?;

    let template_dir = root.join("templates");
    if !template_dir.is_dir() {
        return Err(Error::TemplatesMissing(template_dir));
    }
    let catalog = Catalog::load(&root.join("catalog.yaml"))?;
    let (shape, runtime, mut org) = catalog.apply_preset(
        req.preset.as_deref(),
        req.shape.as_deref(),
        req.runtime.as_deref(),
        req.org.as_deref(),
    )?;
    if req.lab.is_some() {
        let explicit_none = req.org.as_deref().is_some_and(|value| value == "none");
        if explicit_none {
            return Err(Error::LabWithNeutralOrg);
        }
        if org == "none" {
            org = "lab".to_string();
        }
    }
    let resolved = catalog.resolve(Resolve {
        shape: &shape,
        runtime: &runtime,
        org: &org,
        github_org: req.github_org.as_deref(),
        project_name: &req.project_name,
    })?;

    let mut vars = resolved.vars.clone();
    if let Some(lab_path) = req.lab.as_deref() {
        let lab_file = lab::load_lab(lab_path)?;
        lab::overlay_lab(&mut vars, &lab_file, &req.project_name);
        log(&format!("Lab file: {}", lab_path.display()));
    }
    if let Some(ghorg) = req.github_org.as_deref().filter(|value| !value.is_empty()) {
        vars.insert("GITHUB_ORG".to_string(), ghorg.to_string());
        catalog::fill_repo_urls(&mut vars, &req.project_name);
    }
    vars.insert("PROJECT_NAME".to_string(), req.project_name.clone());
    vars.insert("PROJECT_TITLE".to_string(), req.title.clone());
    vars.insert("PACKAGE".to_string(), package);

    log(&format!("Scaffolding {} — {}", req.project_name, req.title));
    log(&format!("Destination: {}", req.outdir.display()));
    log(&format!(
        "Catalog: shape={} runtime={} org={}",
        resolved.shape, resolved.runtime, resolved.org
    ));
    if req.with_agents {
        log("Agents: nested symkit install (pass --no-agents to skip)");
    } else {
        log("Agents: no");
    }

    let (mut created, mut skipped) = apply_tree(&template_dir, &req.outdir, &resolved.dirs, &resolved.items, &vars)?;
    if req.migration_docs {
        write_migration(&template_dir, &req.outdir, &mut created, &mut skipped)?;
    } else {
        log("migration-docs: skipped (--no-migration-docs)");
    }
    write_project_sidecar(&req.outdir, &req.project_name, &resolved.org, &vars)?;

    if resolved.runtime == "uv" {
        maybe_uv_lock(&req.outdir, &mut created, &mut skipped)?;
    }
    log(&format!("Tree: created={created} skipped={skipped}"));

    init_git(&req.outdir)?;

    if let Some(slug) = req.paper.as_deref().filter(|s| !s.is_empty()) {
        let title = req.paper_title.as_deref().filter(|s| !s.is_empty()).unwrap_or(slug);
        add_paper(root, &req.outdir, slug, title)?;
    }

    install_agents(req)?;

    log("Next: edit the aim in README.md, then add papers with `symproject paper <slug> --title \"...\"`.");
    if req.lab.is_some() {
        log("Lab file applied. Later changes: symproject set-lab <file>.");
    } else if resolved.org == "lab" {
        log("Placeholder lab line. symproject labs, then symproject set-lab <file>.");
    } else {
        log("No lab line. symproject labs, then pass --lab on the first new.");
    }
    log("Work on nb/<slug>. main holds the settled layout.");
    log("Do not commit data/private/, data/open/, or generated files under results/.");
    Ok(())
}

pub struct UpdateProject {
    pub outdir: PathBuf,
    pub title: Option<String>,
    pub package: Option<String>,
    pub shape: Option<String>,
    pub runtime: Option<String>,
    pub migration_docs: bool,
}

/// Fill scaffold files that are missing from a project that already has `.symproject.yaml`.
pub fn update(root: &Path, req: &UpdateProject) -> Result<()> {
    refuse_kit(root, &req.outdir)?;
    let sidecar = lab::load_sidecar(&req.outdir.join(".symproject.yaml"))?;
    if !valid_slug(&sidecar.project_name) {
        return Err(Error::InvalidProjectName(sidecar.project_name));
    }
    let package = package_for(&req.outdir, req.package.as_deref(), &sidecar.project_name)?;
    let title = title_for(&req.outdir, &sidecar.project_name, req.title.as_deref());
    let runtime = runtime_for(&req.outdir, req.runtime.as_deref());

    let template_dir = root.join("templates");
    if !template_dir.is_dir() {
        return Err(Error::TemplatesMissing(template_dir));
    }
    let catalog = Catalog::load(&root.join("catalog.yaml"))?;
    let (shape, runtime, org) = catalog.apply_preset(None, req.shape.as_deref(), Some(&runtime), Some(&sidecar.org))?;
    let resolved = catalog.resolve(Resolve {
        shape: &shape,
        runtime: &runtime,
        org: &org,
        github_org: None,
        project_name: &sidecar.project_name,
    })?;

    let mut vars = resolved.vars.clone();
    for (key, value) in &sidecar.holdings {
        vars.insert(key.clone(), value.clone());
    }
    vars.insert("PROJECT_NAME".to_string(), sidecar.project_name.clone());
    vars.insert("PROJECT_TITLE".to_string(), title.clone());
    vars.insert("PACKAGE".to_string(), package);

    log(&format!("Updating {}", sidecar.project_name));
    log(&format!("Destination: {}", req.outdir.display()));
    log(&format!(
        "Catalog: shape={} runtime={} org={}",
        resolved.shape, resolved.runtime, resolved.org
    ));

    let (mut created, mut skipped) = apply_tree(&template_dir, &req.outdir, &resolved.dirs, &resolved.items, &vars)?;
    if req.migration_docs {
        write_migration(&template_dir, &req.outdir, &mut created, &mut skipped)?;
    } else {
        log("migration-docs: skipped (--no-migration-docs)");
    }
    if resolved.runtime == "uv" {
        maybe_uv_lock(&req.outdir, &mut created, &mut skipped)?;
    }
    log(&format!("Tree: created={created} skipped={skipped}"));
    log("Existing files were left in place. set-lab changes the lab line.");
    Ok(())
}

pub fn create_lab(root: &Path, dest: &Path) -> Result<PathBuf> {
    let src = root.join("templates/lab.yml");
    if !src.is_file() {
        return Err(Error::MissingTemplate(src.display().to_string()));
    }
    let dest = if dest.is_dir() {
        dest.join("lab.yml")
    } else {
        dest.to_path_buf()
    };
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() && parent.exists() && is_root(parent) {
            let shown = fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
            return Err(Error::KitRoot(shown));
        }
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    if dest.exists() {
        log(&format!("skip (exists): {}", dest.display()));
        log("lab.yml was not replaced. Edit it, then pass --lab or set-lab.");
        return Ok(dest);
    }
    fs::copy(&src, &dest)?;
    log(&format!("created: {}", dest.display()));
    log("Edit github_org, lab_name, lab_url, and issue_line.");
    log(&format!(
        "Then: symproject new <name> --title \"...\" --lab {}",
        dest.display()
    ));
    log(&format!(
        "Or, on a project that already has a lab line: symproject set-lab {}",
        dest.display()
    ));
    Ok(dest)
}

pub fn set_lab(outdir: &Path, lab_path: &Path, dry_run: bool) -> Result<()> {
    if outdir.exists() && is_root(outdir) {
        let out_canon = fs::canonicalize(outdir).unwrap_or_else(|_| outdir.to_path_buf());
        return Err(Error::KitRoot(out_canon));
    }
    let sidecar_path = outdir.join(".symproject.yaml");
    let sidecar = lab::load_sidecar(&sidecar_path)?;
    if !valid_slug(&sidecar.project_name) {
        return Err(Error::InvalidProjectName(sidecar.project_name));
    }
    for file in &sidecar.files {
        check_rel(file)?;
    }
    let lab_file = lab::load_lab(lab_path)?;
    let mut current = BTreeMap::new();
    for file in &sidecar.files {
        let path = outdir.join(file);
        if !path.is_file() {
            return Err(Error::MissingProjectFile(path));
        }
        current.insert(file.clone(), fs::read_to_string(path)?);
    }
    let plan = lab::plan_lab(&current, &sidecar, &lab_file)?;
    let label = if dry_run { "set-lab (dry-run)" } else { "set-lab" };
    log(&format!("{label}: {}", sidecar.project_name));
    for note in &plan.notes {
        log(note);
    }
    for warning in &plan.warnings {
        warn(warning);
    }
    let files_dirty = plan.files.iter().any(|(name, text)| current.get(name) != Some(text));
    let holdings_dirty = plan.holdings != sidecar.holdings;
    if !files_dirty && !holdings_dirty {
        if plan.warnings.is_empty() {
            log("lab holdings already match");
        }
        return Ok(());
    }
    if dry_run {
        return Ok(());
    }
    for (file, text) in &plan.files {
        if current.get(file) != Some(text) {
            fs::write(outdir.join(file), text)?;
        }
    }
    if holdings_dirty {
        let mut updated = sidecar;
        updated.holdings = plan.holdings;
        lab::write_sidecar(&sidecar_path, &updated)?;
    }
    Ok(())
}

fn write_project_sidecar(outdir: &Path, project_name: &str, org: &str, vars: &BTreeMap<String, String>) -> Result<()> {
    let path = outdir.join(".symproject.yaml");
    if path.exists() {
        log("skip (exists): .symproject.yaml");
        return Ok(());
    }
    lab::write_sidecar(
        &path,
        &lab::Sidecar {
            version: 1,
            project_name: project_name.to_string(),
            org: org.to_string(),
            holdings: lab::holdings_from_vars(vars),
            files: lab::PROJECT_FILES.iter().map(|name| (*name).to_string()).collect(),
        },
    )?;
    log("created: .symproject.yaml");
    Ok(())
}

pub fn add_paper(root: &Path, outdir: &Path, slug: &str, title: &str) -> Result<()> {
    if !valid_slug(slug) {
        return Err(Error::InvalidPaperSlug(slug.to_string()));
    }
    if title.is_empty() {
        return Err(Error::MissingTitle);
    }
    refuse_kit(root, outdir)?;
    let template_dir = root.join("templates");
    if !template_dir.is_dir() {
        return Err(Error::TemplatesMissing(template_dir));
    }
    let catalog = Catalog::load(&root.join("catalog.yaml"))?;
    let (dirs, items) = catalog.paper();
    let mut vars = BTreeMap::new();
    vars.insert("SLUG".to_string(), slug.to_string());
    vars.insert("PAPER_TITLE".to_string(), title.to_string());

    log(&format!("Paper: {slug} — {title}"));
    log(&format!("Destination: {}", outdir.display()));
    let (created, skipped) = apply_tree(&template_dir, outdir, &dirs, &items, &vars)?;
    log(&format!("Paper tree: created={created} skipped={skipped}"));
    println!();
    println!("Add this row to the paper-line table in README.md:");
    println!();
    println!("| {title} | — | Draft — `papers/{slug}/` on `nb/{slug}` |");
    println!();
    Ok(())
}

fn refuse_kit(root: &Path, outdir: &Path) -> Result<()> {
    if outdir.exists() && is_root(outdir) {
        let out_canon = fs::canonicalize(outdir).unwrap_or_else(|_| outdir.to_path_buf());
        let root_canon = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        if out_canon == root_canon {
            return Err(Error::KitRoot(out_canon));
        }
    }
    Ok(())
}

fn apply_tree(
    template_dir: &Path,
    outdir: &Path,
    dirs: &[String],
    items: &[Item],
    vars: &BTreeMap<String, String>,
) -> Result<(u32, u32)> {
    let mut created = 0u32;
    let mut skipped = 0u32;
    for dir in dirs {
        let rel = render(dir, vars);
        check_rel(&rel)?;
        fs::create_dir_all(outdir.join(&rel))?;
    }
    for item in items {
        let dest = render(&item.dest, vars);
        check_rel(&dest)?;
        match item.kind {
            ItemKind::File => install_file(template_dir, outdir, &item.src, &dest, vars, &mut created, &mut skipped)?,
            ItemKind::Copy => install_copy(
                template_dir,
                outdir,
                &item.src,
                &dest,
                &item.mode,
                &mut created,
                &mut skipped,
            )?,
            ItemKind::Append => append_fragment(template_dir, outdir, &item.src, &dest, &mut created, &mut skipped)?,
            ItemKind::Keep => keep_file(outdir, &dest, &mut created, &mut skipped)?,
        }
    }
    Ok((created, skipped))
}

pub(crate) fn valid_slug(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn package_for(outdir: &Path, flag: Option<&str>, project_name: &str) -> Result<String> {
    if let Some(name) = flag.map(str::trim).filter(|name| !name.is_empty()) {
        if !valid_package(name) {
            return Err(Error::InvalidPackageName(name.to_string()));
        }
        return Ok(name.to_string());
    }
    let mut found = Vec::new();
    let src = outdir.join("src");
    if src.is_dir() {
        for entry in fs::read_dir(&src)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || name == "__pycache__" {
                continue;
            }
            found.push(name.into_owned());
        }
    }
    found.sort();
    match found.as_slice() {
        [one] => {
            if !valid_package(one) {
                return Err(Error::InvalidPackageName(one.clone()));
            }
            Ok(one.clone())
        }
        [] => {
            let derived = project_name.replace('-', "_");
            if !valid_package(&derived) {
                return Err(Error::PackageNeedsFlag(project_name.to_string()));
            }
            Ok(derived)
        }
        many => Err(Error::Msg(format!(
            "src/ has more than one package ({}); pass --package",
            many.join(", ")
        ))),
    }
}

fn title_for(outdir: &Path, project_name: &str, flag: Option<&str>) -> String {
    if let Some(title) = flag.map(str::trim).filter(|title| !title.is_empty()) {
        return title.to_string();
    }
    if let Ok(text) = fs::read_to_string(outdir.join("README.md")) {
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("# ") else {
                continue;
            };
            let title = rest.trim().trim_start_matches('#').trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }
    project_name.to_string()
}

fn runtime_for(outdir: &Path, flag: Option<&str>) -> String {
    if let Some(name) = flag.map(str::trim).filter(|name| !name.is_empty()) {
        return name.to_string();
    }
    if outdir.join("pyproject.toml").is_file() {
        "uv".to_string()
    } else {
        "none".to_string()
    }
}

pub(crate) fn valid_package(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => chars.all(|c| c.is_ascii_alphanumeric() || c == '_'),
        _ => false,
    }
}

fn check_rel(path: &str) -> Result<()> {
    let p = Path::new(path);
    if path.is_empty() || p.is_absolute() || p.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(Error::UnsafePath(path.to_string()));
    }
    Ok(())
}

fn log(msg: &str) {
    println!(">>> {msg}");
}

fn warn(msg: &str) {
    eprintln!("warning: {msg}");
}

pub(crate) fn render(text: &str, vars: &BTreeMap<String, String>) -> String {
    let mut keys: Vec<&String> = vars.keys().collect();
    keys.sort_by_key(|key| std::cmp::Reverse(key.len()));
    let mut text = text.to_string();
    for key in keys {
        let needle = format!("__{key}__");
        if let Some(value) = vars.get(key) {
            text = text.replace(&needle, value);
        }
    }
    text
}

fn install_file(
    template_dir: &Path,
    outdir: &Path,
    src_rel: &str,
    dest_rel: &str,
    vars: &BTreeMap<String, String>,
    created: &mut u32,
    skipped: &mut u32,
) -> Result<()> {
    let src = template_dir.join(src_rel);
    let dest = outdir.join(dest_rel);
    if !src.is_file() {
        return Err(Error::MissingTemplate(src_rel.to_string()));
    }
    if dest.exists() {
        log(&format!("skip (exists): {dest_rel}"));
        *skipped += 1;
        return Ok(());
    }
    let text = fs::read_to_string(&src)?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&dest, render(&text, vars))?;
    log(&format!("created: {dest_rel}"));
    *created += 1;
    Ok(())
}

fn install_copy(
    template_dir: &Path,
    outdir: &Path,
    src_rel: &str,
    dest_rel: &str,
    mode: &str,
    created: &mut u32,
    skipped: &mut u32,
) -> Result<()> {
    let src = template_dir.join(src_rel);
    let dest = outdir.join(dest_rel);
    if !src.is_file() {
        return Err(Error::MissingTemplate(src_rel.to_string()));
    }
    if dest.exists() {
        log(&format!("skip (exists): {dest_rel}"));
        *skipped += 1;
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&src, &dest)?;
    if mode == "exec" {
        make_exec(&dest)?;
    }
    log(&format!("created: {dest_rel}"));
    *created += 1;
    Ok(())
}

fn append_fragment(
    template_dir: &Path,
    outdir: &Path,
    src_rel: &str,
    dest_rel: &str,
    created: &mut u32,
    skipped: &mut u32,
) -> Result<()> {
    let src = template_dir.join(src_rel);
    let dest = outdir.join(dest_rel);
    if !src.is_file() {
        return Err(Error::MissingTemplate(src_rel.to_string()));
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let mark = UV_GITIGNORE_MARK.as_bytes();
    if dest.is_file() {
        let existing = fs::read(&dest)?;
        if existing.windows(mark.len()).any(|w| w == mark) {
            log(&format!("skip (exists): {dest_rel} runtime fragment"));
            *skipped += 1;
            return Ok(());
        }
    }
    let mut file = fs::OpenOptions::new().create(true).append(true).open(&dest)?;
    file.write_all(b"\n")?;
    file.write_all(&fs::read(src)?)?;
    log(&format!("appended: {dest_rel} ← {src_rel}"));
    *created += 1;
    Ok(())
}

fn keep_file(outdir: &Path, dest_rel: &str, created: &mut u32, skipped: &mut u32) -> Result<()> {
    let dest = outdir.join(dest_rel);
    if dest.exists() {
        log(&format!("skip (exists): {dest_rel}"));
        *skipped += 1;
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&dest, "")?;
    log(&format!("created: {dest_rel}"));
    *created += 1;
    Ok(())
}

fn write_migration(template_dir: &Path, outdir: &Path, created: &mut u32, skipped: &mut u32) -> Result<()> {
    log("migration-docs: local only; back the directory up elsewhere");
    install_copy(
        template_dir,
        outdir,
        "shapes/paper-line/migration-docs/README.md.tpl",
        "migration-docs/README.md",
        "",
        created,
        skipped,
    )?;
    install_copy(
        template_dir,
        outdir,
        "shapes/paper-line/migration-docs/gitkeep.tpl",
        "migration-docs/.gitkeep",
        "",
        created,
        skipped,
    )?;
    sync_migration_gitignore(outdir, created, skipped)
}

fn sync_migration_gitignore(outdir: &Path, created: &mut u32, skipped: &mut u32) -> Result<()> {
    let gitkeep = fs::read_to_string(outdir.join("migration-docs/.gitkeep"))?;
    let extras = parse_gitkeep(&gitkeep)?;
    let block = migration_block(&extras);
    let path = outdir.join(".gitignore");
    let existing = if path.is_file() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };
    let next = upsert_marked_block(&existing, MIG_BEGIN, MIG_END, &block)?;
    if next == existing {
        log("skip (exists): .gitignore migration-docs");
        *skipped += 1;
        return Ok(());
    }
    fs::write(&path, next)?;
    log("updated: .gitignore migration-docs");
    *created += 1;
    Ok(())
}

/// Paths listed in `.gitkeep`, relative to `migration-docs/`.
fn parse_gitkeep(text: &str) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    for raw in text.lines() {
        let line = raw.trim().trim_start_matches('!').trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("migration-docs/").unwrap_or(line);
        let rel = check_migration_rel(line)?;
        if rel == "README.md" || rel == ".gitkeep" || paths.iter().any(|p| p == &rel) {
            continue;
        }
        paths.push(rel);
    }
    Ok(paths)
}

fn check_migration_rel(line: &str) -> Result<String> {
    if line.contains('\\') || line.starts_with('/') {
        return Err(Error::Msg(format!("migration-docs/.gitkeep: unsafe path '{line}'")));
    }
    let mut parts = Vec::new();
    for part in Path::new(line).components() {
        match part {
            Component::Normal(name) => {
                let name = name.to_string_lossy();
                if name.contains('*') || name.contains('?') || name.contains('[') {
                    return Err(Error::Msg(format!("migration-docs/.gitkeep: unsafe path '{line}'")));
                }
                parts.push(name.to_string());
            }
            _ => return Err(Error::Msg(format!("migration-docs/.gitkeep: unsafe path '{line}'"))),
        }
    }
    if parts.is_empty() {
        return Err(Error::Msg(format!("migration-docs/.gitkeep: unsafe path '{line}'")));
    }
    Ok(parts.join("/"))
}

fn migration_block(extras: &[String]) -> String {
    let mut lines = vec![
        MIG_BEGIN.to_string(),
        "# Local only. Back this directory up elsewhere.".to_string(),
        "# Track another file by listing it in migration-docs/.gitkeep, then re-run new.".to_string(),
        "migration-docs/**".to_string(),
        "!migration-docs/README.md".to_string(),
        "!migration-docs/.gitkeep".to_string(),
    ];
    let mut seen = Vec::new();
    for rel in extras {
        let path = Path::new(rel);
        let ancestors: Vec<_> = path
            .parent()
            .into_iter()
            .flat_map(|parent| parent.ancestors())
            .collect();
        for dir in ancestors.into_iter().rev() {
            if dir.as_os_str().is_empty() {
                continue;
            }
            let pattern = format!("!migration-docs/{}/", dir.display());
            if !seen.contains(&pattern) {
                seen.push(pattern.clone());
                lines.push(pattern);
            }
        }
        lines.push(format!("!migration-docs/{rel}"));
    }
    lines.push(MIG_END.to_string());
    let mut block = lines.join("\n");
    block.push('\n');
    block
}

fn upsert_marked_block(text: &str, begin: &str, end: &str, block: &str) -> Result<String> {
    let Some(start) = text.find(begin) else {
        let mut out = text.to_string();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.is_empty() && !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(block);
        if !block.ends_with('\n') {
            out.push('\n');
        }
        return Ok(out);
    };
    let Some(rel_end) = text[start..].find(end) else {
        return Err(Error::Msg(".gitignore migration-docs marker is incomplete".into()));
    };
    let mut end_pos = start + rel_end + end.len();
    if text[end_pos..].starts_with('\n') {
        end_pos += 1;
    }
    let mut out = String::new();
    out.push_str(&text[..start]);
    out.push_str(block);
    if !block.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&text[end_pos..]);
    Ok(out)
}

fn maybe_uv_lock(outdir: &Path, created: &mut u32, skipped: &mut u32) -> Result<()> {
    let pyproject = outdir.join("pyproject.toml");
    let lock = outdir.join("uv.lock");
    if pyproject.is_file() && on_path("uv") {
        if lock.is_file() {
            log("skip (exists): uv.lock");
            *skipped += 1;
            return Ok(());
        }
        log("writing uv.lock");
        let status = Command::new("uv").arg("lock").current_dir(outdir).status()?;
        if !status.success() {
            return Err(Error::Msg(format!("uv lock failed ({status})")));
        }
        *created += 1;
        Ok(())
    } else if !lock.is_file() {
        warn(&format!(
            "uv not on PATH; skip uv.lock. Run: cd {} && uv lock",
            outdir.display()
        ));
        Ok(())
    } else {
        Ok(())
    }
}

fn init_git(outdir: &Path) -> Result<()> {
    if outdir.join(".git").is_dir() {
        log("git: already initialized");
        return Ok(());
    }
    let status = Command::new("git")
        .arg("-C")
        .arg(outdir)
        .arg("init")
        .arg("-b")
        .arg("main")
        .stdout(Stdio::null())
        .status()?;
    if !status.success() {
        return Err(Error::Msg(format!("git init failed ({status})")));
    }
    log("git init (default branch main)");
    Ok(())
}

fn install_agents(req: &NewProject) -> Result<()> {
    if !req.with_agents {
        log("agents: skipped (--no-agents)");
        return Ok(());
    }
    let mut argv = vec![
        "symkit".to_string(),
        "install".to_string(),
        req.outdir.display().to_string(),
        "--harness".to_string(),
        "research".to_string(),
        "--role".to_string(),
        "researcher".to_string(),
        "--yes".to_string(),
    ];
    if let Some(adapters) = req.adapters.as_deref().filter(|s| !s.is_empty()) {
        argv.push("--adapters".to_string());
        argv.push(adapters.to_string());
    }
    if !on_path("symkit") {
        warn("symkit not on PATH; skip agent install.");
        warn(&format!("Next: {}", argv.join(" ")));
        return Ok(());
    }
    log("Installing research harness (researcher; no --scaffold, no --docs)");
    let mut cmd = Command::new("symkit");
    cmd.args(&argv[1..]);
    let status = cmd.status()?;
    if !status.success() {
        return Err(Error::Msg(format!("symkit install failed ({status})")));
    }
    Ok(())
}

fn on_path(bin: &str) -> bool {
    match Command::new(bin)
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(_) => true,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => false,
    }
}

#[cfg(unix)]
fn make_exec(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_exec(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    fn kit() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn new_req(outdir: PathBuf, name: &str, runtime: &str) -> NewProject {
        NewProject {
            project_name: name.to_string(),
            outdir,
            title: "Demo line".to_string(),
            package: None,
            shape: None,
            runtime: Some(runtime.to_string()),
            org: None,
            preset: None,
            github_org: None,
            lab: None,
            paper: None,
            paper_title: None,
            with_agents: false,
            adapters: None,
            migration_docs: true,
        }
    }

    #[test]
    fn render_replaces_longer_keys_first() {
        let mut vars = BTreeMap::new();
        vars.insert("PACKAGE".to_string(), "coupling".to_string());
        vars.insert("PROJECT_NAME".to_string(), "chp-coupling".to_string());
        let text = render("src/__PACKAGE__ of __PROJECT_NAME__", &vars);
        assert_eq!(text, "src/coupling of chp-coupling");
    }

    #[test]
    fn slug_and_package_rules() {
        assert!(valid_slug("chp-coupling"));
        assert!(valid_slug("multirate-fusion"));
        assert!(!valid_slug(""));
        assert!(!valid_slug("-nope"));
        assert!(!valid_slug("has space"));
        assert!(valid_package("chp_coupling"));
        assert!(valid_package("coupling"));
        assert!(!valid_package("chp-coupling"));
        assert!(!valid_package("1abc"));
    }

    #[test]
    fn new_runtime_none_writes_spine_and_skips_on_rerun() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("demo-study");
        scaffold(&kit(), &new_req(out.clone(), "demo-study", "none")).unwrap();
        assert!(out.join("src/demo_study/__init__.py").is_file());
        assert!(out.join("src/demo_study/README.md").is_file());
        assert!(out.join("papers/README.md").is_file());
        assert!(out.join("results/README.md").is_file());
        assert!(out.join("data/README.md").is_file());
        assert!(out.join("data/samples/.gitkeep").is_file());
        assert!(out.join("AGENTS.md").is_file());
        assert!(out.join("docs/design.md").is_file());
        assert!(out.join("docs/funding/README.md").is_file());
        assert!(out.join("docs/methods/README.md").is_file());
        assert!(out.join("analysis/README.md").is_file());
        assert!(!out.join("docs/aims.md").exists());
        let agents = fs::read_to_string(out.join("AGENTS.md")).unwrap();
        assert!(agents.contains("notes/claims.md"), "{agents}");
        assert!(agents.contains("Do not add `docs/aims.md`"), "{agents}");
        assert!(out.join("migration-docs/README.md").is_file());
        assert!(out.join("migration-docs/.gitkeep").is_file());
        let gitignore = fs::read_to_string(out.join(".gitignore")).unwrap();
        assert_eq!(gitignore.matches(MIG_BEGIN).count(), 1, "{gitignore}");
        assert!(gitignore.contains("migration-docs/**"), "{gitignore}");
        assert!(!out.join("pyproject.toml").exists());
        let branch = Command::new("git")
            .arg("-C")
            .arg(&out)
            .arg("branch")
            .arg("--show-current")
            .output()
            .unwrap();
        assert_eq!(String::from_utf8(branch.stdout).unwrap().trim(), "main");
        let readme = fs::read_to_string(out.join("README.md")).unwrap();
        assert!(readme.contains("Demo line"));
        assert!(readme.contains("git clone <repository-url>"), "{readme}");
        assert!(!readme.contains("lab.example"), "{readme}");
        assert!(!readme.contains("__PROJECT_TITLE__"));
        assert!(out.join(".symproject.yaml").is_file());
        fs::write(out.join("README.md"), "keep-me\n").unwrap();
        scaffold(&kit(), &new_req(out.clone(), "demo-study", "none")).unwrap();
        assert_eq!(fs::read_to_string(out.join("README.md")).unwrap(), "keep-me\n");
    }

    #[test]
    fn package_flag_and_paper_stub() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("chp-coupling");
        let mut req = new_req(out.clone(), "chp-coupling", "none");
        req.package = Some("coupling".to_string());
        req.paper = Some("multirate-fusion".to_string());
        req.paper_title = Some("Multirate fusion".to_string());
        scaffold(&kit(), &req).unwrap();
        assert!(out.join("src/coupling/__init__.py").is_file());
        assert!(!out.join("src/chp_coupling").exists());
        assert!(out.join("papers/multirate-fusion/manuscript.md").is_file());
        assert!(out.join("papers/multirate-fusion/notes/claims.md").is_file());
        assert!(out.join("papers/multirate-fusion/analysis/README.md").is_file());
        assert!(out.join("results/multirate-fusion/.gitkeep").is_file());
        let claims = fs::read_to_string(out.join("papers/multirate-fusion/notes/claims.md")).unwrap();
        assert!(claims.contains("Multirate fusion"));
        assert!(claims.contains("## Plan"), "{claims}");
    }

    #[test]
    fn missing_title_and_bad_names_fail() {
        let tmp = tempfile::tempdir().unwrap();
        let mut req = new_req(tmp.path().join("x"), "ok-name", "none");
        req.title.clear();
        assert!(matches!(scaffold(&kit(), &req).unwrap_err(), Error::MissingTitle));
        req.title = "T".to_string();
        req.project_name = "has space".to_string();
        assert!(matches!(
            scaffold(&kit(), &req).unwrap_err(),
            Error::InvalidProjectName(_)
        ));
        req.project_name = "ok-name".to_string();
        req.package = Some("not-py".to_string());
        assert!(matches!(
            scaffold(&kit(), &req).unwrap_err(),
            Error::InvalidPackageName(_)
        ));
    }

    #[test]
    fn migration_tracks_a_path_listed_in_gitkeep() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("demo-study");
        let req = new_req(out.clone(), "demo-study", "none");
        scaffold(&kit(), &req).unwrap();
        assert!(out.join("migration-docs/README.md").is_file());
        let readme = fs::read_to_string(out.join("migration-docs/README.md")).unwrap();
        assert!(readme.contains("Back it up somewhere else"), "{readme}");
        let gitignore = fs::read_to_string(out.join(".gitignore")).unwrap();
        assert_eq!(gitignore.matches(MIG_BEGIN).count(), 1, "{gitignore}");
        assert!(gitignore.contains("migration-docs/**"), "{gitignore}");
        assert!(!gitignore.contains("!migration-docs/legacy.pdf"), "{gitignore}");

        let mut keep = fs::read_to_string(out.join("migration-docs/.gitkeep")).unwrap();
        keep.push_str("legacy.pdf\nscans/visit.pdf\n../secret\n");
        fs::write(out.join("migration-docs/.gitkeep"), &keep).unwrap();
        let err = scaffold(&kit(), &req).unwrap_err();
        assert!(err.to_string().contains("unsafe path"), "{err}");

        keep = fs::read_to_string(out.join("migration-docs/.gitkeep")).unwrap();
        let keep = keep.replace("../secret\n", "");
        fs::write(out.join("migration-docs/.gitkeep"), keep).unwrap();
        let before = fs::read_to_string(out.join("migration-docs/README.md")).unwrap();
        scaffold(&kit(), &req).unwrap();
        let gitignore = fs::read_to_string(out.join(".gitignore")).unwrap();
        assert_eq!(gitignore.matches(MIG_BEGIN).count(), 1, "{gitignore}");
        assert!(gitignore.contains("!migration-docs/legacy.pdf"), "{gitignore}");
        assert!(gitignore.contains("!migration-docs/scans/"), "{gitignore}");
        assert!(gitignore.contains("!migration-docs/scans/visit.pdf"), "{gitignore}");
        assert_eq!(
            fs::read_to_string(out.join("migration-docs/README.md")).unwrap(),
            before
        );
        scaffold(&kit(), &req).unwrap();
        assert_eq!(
            fs::read_to_string(out.join(".gitignore"))
                .unwrap()
                .matches(MIG_BEGIN)
                .count(),
            1
        );
    }

    #[test]
    fn no_migration_docs_skips_create_and_refresh() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("demo-study");
        let mut req = new_req(out.clone(), "demo-study", "none");
        req.migration_docs = false;
        scaffold(&kit(), &req).unwrap();
        assert!(!out.join("migration-docs").exists());
        let gitignore = fs::read_to_string(out.join(".gitignore")).unwrap();
        assert!(gitignore.contains("migration-docs/**"), "{gitignore}");

        req.migration_docs = true;
        scaffold(&kit(), &req).unwrap();
        assert!(out.join("migration-docs/README.md").is_file());
        let mut keep = fs::read_to_string(out.join("migration-docs/.gitkeep")).unwrap();
        keep.push_str("legacy.pdf\n");
        fs::write(out.join("migration-docs/.gitkeep"), keep).unwrap();
        req.migration_docs = false;
        scaffold(&kit(), &req).unwrap();
        let gitignore = fs::read_to_string(out.join(".gitignore")).unwrap();
        assert!(!gitignore.contains("!migration-docs/legacy.pdf"), "{gitignore}");
        assert!(out.join("migration-docs/README.md").is_file());
    }

    fn write_lab(dir: &Path, github_org: &str, lab_name: &str, lab_url: &str, issue: &str) -> PathBuf {
        let path = dir.join(format!("{github_org}.yml"));
        let text = format!(
            "version: 1\ngithub_org: {github_org}\nlab_name: \"{lab_name}\"\nlab_url: {lab_url}\nissue_line: \"{issue}\"\n"
        );
        fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn set_lab_replaces_holdings_and_a_second_file_replaces_them_again() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("demo-study");
        let mut req = new_req(out.clone(), "demo-study", "none");
        req.org = Some("lab".to_string());
        scaffold(&kit(), &req).unwrap();
        add_paper(&kit(), &out, "multirate-fusion", "Multirate fusion").unwrap();
        let claims = fs::read_to_string(out.join("papers/multirate-fusion/notes/claims.md")).unwrap();
        fs::write(out.join("README.md"), {
            let mut text = fs::read_to_string(out.join("README.md")).unwrap();
            text.push_str("keep-me\n");
            text
        })
        .unwrap();

        let first = write_lab(
            tmp.path(),
            "other-org",
            "Other Lab",
            "https://other.example",
            "Tickets live in the lab tracker.",
        );
        let before = fs::read_to_string(out.join("README.md")).unwrap();
        set_lab(&out, &first, true).unwrap();
        assert_eq!(fs::read_to_string(out.join("README.md")).unwrap(), before);

        set_lab(&out, &first, false).unwrap();
        let readme = fs::read_to_string(out.join("README.md")).unwrap();
        assert!(readme.contains("Lab: [Other Lab](https://other.example)."), "{readme}");
        assert!(
            readme.contains("git clone https://github.com/other-org/demo-study.git"),
            "{readme}"
        );
        assert!(readme.contains("Tickets live in the lab tracker."), "{readme}");
        assert!(readme.contains("keep-me"), "{readme}");
        assert!(readme.contains("Demo line"), "{readme}");
        assert!(!readme.contains("lab.example"), "{readme}");
        let contributing = fs::read_to_string(out.join("CONTRIBUTING.md")).unwrap();
        assert!(
            contributing.contains("Tickets live in the lab tracker."),
            "{contributing}"
        );
        assert_eq!(
            fs::read_to_string(out.join("papers/multirate-fusion/notes/claims.md")).unwrap(),
            claims
        );

        set_lab(&out, &first, false).unwrap();
        assert_eq!(fs::read_to_string(out.join("README.md")).unwrap(), readme);

        let second = write_lab(
            tmp.path(),
            "third-org",
            "Third Lab",
            "https://third.example",
            "Use the lab board.",
        );
        set_lab(&out, &second, false).unwrap();
        let readme = fs::read_to_string(out.join("README.md")).unwrap();
        assert!(readme.contains("github.com/third-org/demo-study"), "{readme}");
        assert!(readme.contains("Lab: [Third Lab](https://third.example)."), "{readme}");
        assert!(!readme.contains("other-org"), "{readme}");
        assert!(readme.contains("keep-me"), "{readme}");
    }

    #[test]
    fn set_lab_leaves_a_missing_holding_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("demo-study");
        let mut req = new_req(out.clone(), "demo-study", "none");
        req.org = Some("lab".to_string());
        scaffold(&kit(), &req).unwrap();
        let mut readme = fs::read_to_string(out.join("README.md")).unwrap();
        let old = "Lab: [Lab name](https://lab.example).";
        assert!(readme.contains(old), "{readme}");
        readme = readme.replace(old, "Edited lab sentence for this repo.");
        fs::write(out.join("README.md"), &readme).unwrap();
        let lab_file = write_lab(
            tmp.path(),
            "other-org",
            "Other Lab",
            "https://other.example",
            "Tickets live in the lab tracker.",
        );
        set_lab(&out, &lab_file, false).unwrap();
        let readme = fs::read_to_string(out.join("README.md")).unwrap();
        assert!(readme.contains("Edited lab sentence for this repo."), "{readme}");
        assert!(!readme.contains("other.example"), "{readme}");
        assert!(readme.contains("github.com/other-org/demo-study"), "{readme}");
        let sidecar = fs::read_to_string(out.join(".symproject.yaml")).unwrap();
        assert!(sidecar.contains("https://lab.example"), "{sidecar}");
        assert!(sidecar.contains("https://github.com/other-org/demo-study"), "{sidecar}");
    }

    #[test]
    fn org_none_refuses_set_lab() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("neutral");
        let mut req = new_req(out.clone(), "neutral", "none");
        req.org = Some("none".to_string());
        scaffold(&kit(), &req).unwrap();
        let readme = fs::read_to_string(out.join("README.md")).unwrap();
        assert!(!readme.contains("lab.example"), "{readme}");
        let lab_file = write_lab(
            tmp.path(),
            "other-org",
            "Other Lab",
            "https://other.example",
            "Tickets live in the lab tracker.",
        );
        let err = set_lab(&out, &lab_file, false).unwrap_err();
        assert!(err.to_string().contains("no replaceable lab line"), "{err}");
    }

    #[test]
    fn new_lab_file_stamps_the_first_write_and_a_rerun_keeps_it() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("stamped");
        let first = write_lab(
            tmp.path(),
            "other-org",
            "Other Lab",
            "https://other.example",
            "Tickets live in the lab tracker.",
        );
        let mut req = new_req(out.clone(), "stamped", "none");
        req.lab = Some(first);
        scaffold(&kit(), &req).unwrap();
        let readme = fs::read_to_string(out.join("README.md")).unwrap();
        assert!(readme.contains("https://other.example"), "{readme}");
        assert!(readme.contains("github.com/other-org/stamped"), "{readme}");

        let second = write_lab(
            tmp.path(),
            "third-org",
            "Third Lab",
            "https://third.example",
            "Use the lab board.",
        );
        req.lab = Some(second);
        scaffold(&kit(), &req).unwrap();
        let readme = fs::read_to_string(out.join("README.md")).unwrap();
        assert!(readme.contains("https://other.example"), "{readme}");
        assert!(!readme.contains("third-org"), "{readme}");
    }

    #[test]
    fn github_org_flag_overrides_the_lab_file() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("flagged");
        let lab_file = write_lab(
            tmp.path(),
            "other-org",
            "Other Lab",
            "https://other.example",
            "Tickets live in the lab tracker.",
        );
        let mut req = new_req(out.clone(), "flagged", "none");
        req.lab = Some(lab_file);
        req.github_org = Some("flag-org".to_string());
        scaffold(&kit(), &req).unwrap();
        let readme = fs::read_to_string(out.join("README.md")).unwrap();
        assert!(readme.contains("github.com/flag-org/flagged"), "{readme}");
        assert!(readme.contains("https://other.example"), "{readme}");
        assert!(!readme.contains("other-org"), "{readme}");
    }

    #[test]
    fn lab_file_with_org_none_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let lab_file = write_lab(
            tmp.path(),
            "other-org",
            "Other Lab",
            "https://other.example",
            "Tickets live in the lab tracker.",
        );
        let mut req = new_req(tmp.path().join("nope"), "nope", "none");
        req.org = Some("none".to_string());
        req.lab = Some(lab_file);
        let err = scaffold(&kit(), &req).unwrap_err();
        assert!(err.to_string().contains("--org none"), "{err}");
        assert!(!tmp.path().join("nope").exists());
    }

    #[test]
    fn create_lab_writes_a_local_copy_and_skips_a_second_time() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("lab.yml");
        let written = create_lab(&kit(), &dest).unwrap();
        assert_eq!(written, dest);
        let text = fs::read_to_string(&dest).unwrap();
        let template = fs::read_to_string(kit().join("templates/lab.yml")).unwrap();
        assert_eq!(text, template);
        assert!(text.contains("github_org: lab-org"), "{text}");
        assert!(!text.to_lowercase().contains("csymd"), "{text}");
        fs::write(&dest, "keep-me\n").unwrap();
        create_lab(&kit(), tmp.path()).unwrap();
        assert_eq!(fs::read_to_string(&dest).unwrap(), "keep-me\n");
        let err = create_lab(&kit(), &kit().join("lab.yml")).unwrap_err();
        assert!(err.to_string().contains("refusing to write"), "{err}");
    }

    #[test]
    fn set_lab_refuses_the_kit_root() {
        let err = set_lab(&kit(), &kit().join("templates/lab.yml"), false).unwrap_err();
        assert!(err.to_string().contains("refusing to write"), "{err}");
    }

    fn update_req(outdir: PathBuf) -> UpdateProject {
        UpdateProject {
            outdir,
            title: None,
            package: None,
            shape: None,
            runtime: None,
            migration_docs: true,
        }
    }

    #[test]
    fn update_restores_missing_files_and_keeps_the_lab_line() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("demo-study");
        let mut req = new_req(out.clone(), "demo-study", "none");
        req.org = Some("lab".to_string());
        scaffold(&kit(), &req).unwrap();
        let lab = write_lab(
            tmp.path(),
            "other-org",
            "Other Lab",
            "https://other.example",
            "Tickets live in the lab tracker.",
        );
        set_lab(&out, &lab, false).unwrap();
        let readme = fs::read_to_string(out.join("README.md")).unwrap();
        let sidecar = fs::read_to_string(out.join(".symproject.yaml")).unwrap();
        fs::remove_file(out.join("docs/design.md")).unwrap();
        fs::remove_file(out.join("CONTRIBUTING.md")).unwrap();
        let mut keep = fs::read_to_string(out.join("migration-docs/.gitkeep")).unwrap();
        keep.push_str("legacy.pdf\n");
        fs::write(out.join("migration-docs/.gitkeep"), keep).unwrap();

        update(&kit(), &update_req(out.clone())).unwrap();

        assert!(out.join("docs/design.md").is_file());
        let contributing = fs::read_to_string(out.join("CONTRIBUTING.md")).unwrap();
        assert!(
            contributing.contains("Tickets live in the lab tracker."),
            "{contributing}"
        );
        assert_eq!(fs::read_to_string(out.join("README.md")).unwrap(), readme);
        assert_eq!(fs::read_to_string(out.join(".symproject.yaml")).unwrap(), sidecar);
        assert!(!out.join("pyproject.toml").exists());
        let gitignore = fs::read_to_string(out.join(".gitignore")).unwrap();
        assert!(gitignore.contains("!migration-docs/legacy.pdf"), "{gitignore}");
        let branch = Command::new("git")
            .arg("-C")
            .arg(&out)
            .arg("branch")
            .arg("--show-current")
            .output()
            .unwrap();
        assert_eq!(String::from_utf8(branch.stdout).unwrap().trim(), "main");
    }

    #[test]
    fn update_requires_a_sidecar_and_one_package() {
        let tmp = tempfile::tempdir().unwrap();
        let err = update(&kit(), &update_req(tmp.path().to_path_buf())).unwrap_err();
        assert!(err.to_string().contains(".symproject.yaml"), "{err}");

        let out = tmp.path().join("demo-study");
        scaffold(&kit(), &new_req(out.clone(), "demo-study", "none")).unwrap();
        fs::create_dir(out.join("src/other_pkg")).unwrap();
        let err = update(&kit(), &update_req(out.clone())).unwrap_err();
        assert!(err.to_string().contains("pass --package"), "{err}");

        let mut req = update_req(out.clone());
        req.package = Some("other_pkg".to_string());
        req.migration_docs = false;
        fs::remove_file(out.join("docs/design.md")).unwrap();
        let gitignore = fs::read_to_string(out.join(".gitignore")).unwrap();
        update(&kit(), &req).unwrap();
        assert!(out.join("docs/design.md").is_file());
        assert_eq!(fs::read_to_string(out.join(".gitignore")).unwrap(), gitignore);

        let err = update(&kit(), &update_req(kit())).unwrap_err();
        assert!(err.to_string().contains("refusing to write"), "{err}");
    }
}
