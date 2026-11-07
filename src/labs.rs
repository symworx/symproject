// Copyright (c) 2026, Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Local lab files under `$XDG_CONFIG_HOME/symproject/labs`.

use std::{
    env,
    fs,
    io::{
        self,
        BufRead,
        Write,
    },
    path::{
        Path,
        PathBuf,
    },
};

use crate::{
    error::{
        Error,
        Result,
    },
    lab::{
        self,
        LabFile,
    },
    term_menu::{
        self,
        Choice,
        MenuFail,
    },
};

const DEFAULT_ISSUE: &str = "Track each paper in whatever tracker you already use.";

pub fn labs_dir() -> Result<PathBuf> {
    if let Some(path) = env::var_os("SYMPROJECT_LABS") {
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }
    let base = env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    let Some(base) = base else {
        return Err(Error::Msg("HOME is not set; cannot find the labs directory".into()));
    };
    Ok(base.join("symproject/labs"))
}

pub fn resolve_lab_arg(path: &Path) -> PathBuf {
    if path.exists() || path.components().count() != 1 {
        return path.to_path_buf();
    }
    let Some(name) = path.file_name() else {
        return path.to_path_buf();
    };
    let stem = name.to_string_lossy();
    let stem = stem.strip_suffix(".yml").unwrap_or(&stem);
    let Ok(dir) = labs_dir() else {
        return path.to_path_buf();
    };
    let candidate = dir.join(format!("{stem}.yml"));
    if candidate.is_file() {
        candidate
    } else {
        path.to_path_buf()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabEntry {
    pub name: String,
    pub path: PathBuf,
    pub lab: LabFile,
}

pub fn list_labs(dir: &Path) -> Result<Vec<LabEntry>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yml") {
            continue;
        }
        names.push(path);
    }
    names.sort();
    let mut labs = Vec::new();
    for path in names {
        let lab = lab::load_lab(&path)?;
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| lab.github_org.clone());
        labs.push(LabEntry { name, path, lab });
    }
    Ok(labs)
}

pub fn run_labs_interactive(dir: &Path) -> Result<()> {
    let stdin_tty = std::io::IsTerminal::is_terminal(&io::stdin());
    let stdout_tty = std::io::IsTerminal::is_terminal(&io::stdout());
    if stdin_tty && stdout_tty && term_menu::raw_mode_works() {
        return run_labs_arrows(dir);
    }
    run_labs(dir, stdin_tty, &mut io::stdin().lock(), &mut io::stdout())
}

pub fn run_labs(dir: &Path, interactive: bool, reader: &mut impl BufRead, out: &mut impl Write) -> Result<()> {
    if interactive {
        writeln!(
            out,
            "Shortcuts: symproject create lab, symproject edit lab, symproject delete lab"
        )?;
        writeln!(out)?;
    }
    loop {
        print_index(dir, out)?;
        if !interactive {
            writeln!(out, "new:    symproject create lab")?;
            writeln!(out, "edit:   symproject edit lab <name>")?;
            writeln!(out, "delete: symproject delete lab <name>")?;
            return Ok(());
        }
        writeln!(out, "new")?;
        writeln!(out, "edit")?;
        writeln!(out, "delete")?;
        writeln!(out, "quit")?;
        let choice = prompt(out, reader, "> ")?.to_ascii_lowercase();
        match choice.as_str() {
            "new" => {
                if let Err(err) = prompt_new(reader, out).and_then(|lab| store_new(dir, &lab, out)) {
                    writeln!(out, "error: {err}")?;
                }
            }
            "edit" => {
                if let Err(err) = menu_edit(dir, reader, out) {
                    writeln!(out, "error: {err}")?;
                }
            }
            "delete" => {
                if let Err(err) = menu_delete(dir, reader, out) {
                    writeln!(out, "error: {err}")?;
                }
            }
            "quit" | "q" | "" => return Ok(()),
            other => writeln!(out, "choose new, edit, delete, or quit (got '{other}')")?,
        }
    }
}

fn menu_edit(dir: &Path, reader: &mut impl BufRead, out: &mut impl Write) -> Result<()> {
    let Some(entry) = pick_lab(dir, reader, out)? else {
        writeln!(out, "No labs yet.")?;
        return Ok(());
    };
    let updated = prompt_edit(reader, out, &entry.lab)?;
    let path = store_edit(&entry, &updated)?;
    writeln!(out, "updated: {}", path.display())?;
    Ok(())
}

fn menu_delete(dir: &Path, reader: &mut impl BufRead, out: &mut impl Write) -> Result<()> {
    let Some(entry) = pick_lab(dir, reader, out)? else {
        writeln!(out, "No labs yet.")?;
        return Ok(());
    };
    if !confirm_delete(reader, out, &entry.name)? {
        writeln!(out, "left in place: {}", entry.path.display())?;
        return Ok(());
    }
    fs::remove_file(&entry.path)?;
    writeln!(out, "deleted: {}", entry.path.display())?;
    Ok(())
}

fn run_labs_arrows(dir: &Path) -> Result<()> {
    let mut out = io::stdout();
    writeln!(
        out,
        "Shortcuts: symproject create lab, symproject edit lab, symproject delete lab"
    )?;
    writeln!(out)?;
    writeln!(out, "Arrow keys move. Enter selects. q or Ctrl-C quits.")?;
    writeln!(out)?;
    let actions = ["new", "edit", "delete", "quit"];
    loop {
        print_index(dir, &mut out)?;
        let choice = match term_menu::select_index(&mut out, &actions).map_err(menu_error)? {
            Choice::Chosen(index) => actions[index],
            Choice::Back | Choice::Quit => return Ok(()),
        };
        match choice {
            "new" => {
                let mut reader = io::stdin().lock();
                if let Err(err) = prompt_new(&mut reader, &mut out).and_then(|lab| store_new(dir, &lab, &mut out)) {
                    writeln!(out, "error: {err}")?;
                }
            }
            "edit" => match edit_from_arrows(dir, &mut out) {
                Ok(ArrowStep::Continue) => {}
                Ok(ArrowStep::Quit) => return Ok(()),
                Err(err) => writeln!(out, "error: {err}")?,
            },
            "delete" => match delete_from_arrows(dir, &mut out) {
                Ok(ArrowStep::Continue) => {}
                Ok(ArrowStep::Quit) => return Ok(()),
                Err(err) => writeln!(out, "error: {err}")?,
            },
            "quit" => return Ok(()),
            _ => {}
        }
    }
}

enum ArrowStep {
    Continue,
    Quit,
}

enum ArrowPick {
    Chosen(Box<LabEntry>),
    Empty,
    Back,
    Quit,
}

fn edit_from_arrows(dir: &Path, out: &mut impl Write) -> Result<ArrowStep> {
    match pick_arrows(dir, out)? {
        ArrowPick::Empty => {
            writeln!(out, "No labs yet.")?;
            Ok(ArrowStep::Continue)
        }
        ArrowPick::Back => Ok(ArrowStep::Continue),
        ArrowPick::Quit => Ok(ArrowStep::Quit),
        ArrowPick::Chosen(entry) => {
            let mut reader = io::stdin().lock();
            let updated = prompt_edit(&mut reader, out, &entry.lab)?;
            let path = store_edit(&entry, &updated)?;
            writeln!(out, "updated: {}", path.display())?;
            Ok(ArrowStep::Continue)
        }
    }
}

fn delete_from_arrows(dir: &Path, out: &mut impl Write) -> Result<ArrowStep> {
    match pick_arrows(dir, out)? {
        ArrowPick::Empty => {
            writeln!(out, "No labs yet.")?;
            Ok(ArrowStep::Continue)
        }
        ArrowPick::Back => Ok(ArrowStep::Continue),
        ArrowPick::Quit => Ok(ArrowStep::Quit),
        ArrowPick::Chosen(entry) => {
            let mut reader = io::stdin().lock();
            if !confirm_delete(&mut reader, out, &entry.name)? {
                writeln!(out, "left in place: {}", entry.path.display())?;
                return Ok(ArrowStep::Continue);
            }
            fs::remove_file(&entry.path)?;
            writeln!(out, "deleted: {}", entry.path.display())?;
            Ok(ArrowStep::Continue)
        }
    }
}

fn pick_arrows(dir: &Path, out: &mut impl Write) -> Result<ArrowPick> {
    let labs = list_labs(dir)?;
    if labs.is_empty() {
        return Ok(ArrowPick::Empty);
    }
    let labels: Vec<String> = labs
        .iter()
        .map(|entry| format!("{}    {}", entry.name, summary(&entry.lab)))
        .collect();
    match term_menu::select_index(out, &labels).map_err(menu_error)? {
        Choice::Chosen(index) => {
            let Some(entry) = labs.into_iter().nth(index) else {
                return Err(Error::Msg("menu index was out of range".into()));
            };
            Ok(ArrowPick::Chosen(Box::new(entry)))
        }
        Choice::Back => Ok(ArrowPick::Back),
        Choice::Quit => Ok(ArrowPick::Quit),
    }
}

fn menu_error(err: MenuFail) -> Error {
    match err {
        MenuFail::NotATerminal => Error::Msg("terminal raw mode unavailable".into()),
        MenuFail::Io(err) => Error::Io(err),
    }
}

pub fn create_interactive(dir: &Path, reader: &mut impl BufRead, out: &mut impl Write) -> Result<PathBuf> {
    let lab = prompt_new(reader, out)?;
    store_new(dir, &lab, out)
}

pub fn create_from_fields(dir: &Path, lab: &LabFile) -> Result<PathBuf> {
    let path = lab_path(dir, &lab.github_org);
    if path.exists() {
        return Err(Error::Msg(format!("lab already exists: {}", path.display())));
    }
    lab::save_lab(&path, lab)?;
    Ok(path)
}

pub fn edit_command(
    dir: &Path,
    name: Option<&str>,
    patch: LabPatch,
    interactive: bool,
    reader: &mut impl BufRead,
    out: &mut impl Write,
) -> Result<()> {
    let Some(name) = name else {
        if !interactive || !patch.is_empty() {
            return Err(Error::Msg("pass a lab name: symproject edit lab <name>".into()));
        }
        let Some(entry) = pick_lab(dir, reader, out)? else {
            writeln!(out, "No labs yet.")?;
            return Ok(());
        };
        return edit_lab(dir, &entry.name, patch, interactive, reader, out);
    };
    edit_lab(dir, name, patch, interactive, reader, out)
}

pub fn delete_command(
    dir: &Path,
    name: Option<&str>,
    yes: bool,
    interactive: bool,
    reader: &mut impl BufRead,
    out: &mut impl Write,
) -> Result<()> {
    let Some(name) = name else {
        if !interactive {
            return Err(Error::Msg("pass a lab name: symproject delete lab <name>".into()));
        }
        let Some(entry) = pick_lab(dir, reader, out)? else {
            writeln!(out, "No labs yet.")?;
            return Ok(());
        };
        return delete_lab(dir, &entry.name, yes, interactive, reader, out);
    };
    delete_lab(dir, name, yes, interactive, reader, out)
}

pub fn edit_lab(
    dir: &Path,
    name: &str,
    patch: LabPatch,
    interactive: bool,
    reader: &mut impl BufRead,
    out: &mut impl Write,
) -> Result<()> {
    let entry = find_lab(dir, name)?;
    let updated = if patch.is_empty() {
        if !interactive {
            return Err(Error::Msg(format!(
                "pass a field to change, or run `symproject edit lab {}` in a terminal",
                entry.name
            )));
        }
        prompt_edit(reader, out, &entry.lab)?
    } else {
        patch.apply(entry.lab.clone())?
    };
    let path = store_edit(&entry, &updated)?;
    writeln!(out, "updated: {}", path.display())?;
    Ok(())
}

pub fn delete_lab(
    dir: &Path,
    name: &str,
    yes: bool,
    interactive: bool,
    reader: &mut impl BufRead,
    out: &mut impl Write,
) -> Result<()> {
    let entry = find_lab(dir, name)?;
    let confirmed = if yes {
        true
    } else if !interactive {
        return Err(Error::Msg(format!("pass --yes to delete {}", entry.name)));
    } else {
        confirm_delete(reader, out, &entry.name)?
    };
    if !confirmed {
        writeln!(out, "left in place: {}", entry.path.display())?;
        return Ok(());
    }
    fs::remove_file(&entry.path)?;
    writeln!(out, "deleted: {}", entry.path.display())?;
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct LabPatch {
    pub institution_name: Option<String>,
    pub lab_name: Option<String>,
    pub github_org: Option<String>,
    pub lab_url: Option<String>,
    pub investigators: Option<Vec<String>>,
}

impl LabPatch {
    fn is_empty(&self) -> bool {
        self.institution_name.is_none()
            && self.lab_name.is_none()
            && self.github_org.is_none()
            && self.lab_url.is_none()
            && self.investigators.is_none()
    }

    fn apply(self, mut lab: LabFile) -> Result<LabFile> {
        if let Some(value) = self.institution_name {
            lab.institution_name = Some(require_value("institution name", &value)?);
        }
        if let Some(value) = self.lab_name {
            lab.lab_name = require_value("lab name", &value)?;
        }
        if let Some(value) = self.github_org {
            lab.github_org = require_org(&value)?;
        }
        if let Some(value) = self.lab_url {
            lab.lab_url = require_url(&value)?;
        }
        if let Some(names) = self.investigators {
            lab.investigators = clean_investigators(&names)?;
        }
        Ok(lab)
    }
}

fn print_index(dir: &Path, out: &mut impl Write) -> Result<()> {
    writeln!(out, "Labs in {}", dir.display())?;
    let labs = list_labs(dir)?;
    if labs.is_empty() {
        writeln!(out, "  (none)")?;
    }
    for entry in labs {
        writeln!(out, "  {}    {}", entry.name, summary(&entry.lab))?;
    }
    writeln!(out)?;
    Ok(())
}

fn summary(lab: &LabFile) -> String {
    match &lab.institution_name {
        Some(institution) => format!("{}, {institution}", lab.lab_name),
        None => lab.lab_name.clone(),
    }
}

fn pick_lab(dir: &Path, reader: &mut impl BufRead, out: &mut impl Write) -> Result<Option<LabEntry>> {
    let labs = list_labs(dir)?;
    if labs.is_empty() {
        return Ok(None);
    }
    for entry in &labs {
        writeln!(out, "  {}", entry.name)?;
    }
    let name = prompt(out, reader, "Lab name: ")?;
    Ok(Some(find_lab(dir, &name)?))
}

fn confirm_delete(reader: &mut impl BufRead, out: &mut impl Write, name: &str) -> Result<bool> {
    let typed = prompt(out, reader, &format!("Type {name} to delete: "))?;
    Ok(typed == name)
}

fn prompt_new(reader: &mut impl BufRead, out: &mut impl Write) -> Result<LabFile> {
    let institution = require_value("institution name", &prompt(out, reader, "Institution name: ")?)?;
    let lab_name = require_value("lab name", &prompt(out, reader, "Lab name: ")?)?;
    let org_input = prompt(out, reader, "GitHub org: ")?;
    let github_org = if org_input.is_empty() {
        slug_org(&lab_name)?
    } else {
        require_org(&org_input)?
    };
    let lab_url = require_url(&prompt(out, reader, "Lab URL: ")?)?;
    writeln!(out, "Investigators (one name per line, blank line when done):")?;
    let mut investigators = Vec::new();
    loop {
        let name = prompt(out, reader, "  ")?;
        if name.is_empty() {
            break;
        }
        if investigators.iter().any(|have: &String| have == &name) {
            return Err(Error::Msg(format!("duplicate investigator '{name}'")));
        }
        investigators.push(name);
    }
    Ok(LabFile {
        institution_name: Some(institution),
        github_org,
        lab_name,
        lab_url,
        lab_line: None,
        issue_line: Some(DEFAULT_ISSUE.to_string()),
        investigators,
    })
}

fn prompt_edit(reader: &mut impl BufRead, out: &mut impl Write, current: &LabFile) -> Result<LabFile> {
    let institution = prompt_keep(
        out,
        reader,
        "Institution name",
        current.institution_name.as_deref().unwrap_or(""),
    )?;
    let lab_name = prompt_keep(out, reader, "Lab name", &current.lab_name)?;
    let github_org = prompt_keep(out, reader, "GitHub org", &current.github_org)?;
    let lab_url = prompt_keep(out, reader, "Lab URL", &current.lab_url)?;
    let lab_name = if lab_name.is_empty() {
        current.lab_name.clone()
    } else {
        require_value("lab name", &lab_name)?
    };
    let github_org = if github_org.is_empty() {
        current.github_org.clone()
    } else {
        require_org(&github_org)?
    };
    let lab_url = if lab_url.is_empty() {
        current.lab_url.clone()
    } else {
        require_url(&lab_url)?
    };
    let institution_name = if institution.is_empty() {
        current.institution_name.clone()
    } else {
        Some(require_value("institution name", &institution)?)
    };
    writeln!(out, "Investigators:")?;
    for (index, name) in current.investigators.iter().enumerate() {
        writeln!(out, "  {}. {name}", index + 1)?;
    }
    writeln!(out, "add, remove <n>, or blank to finish:")?;
    let mut investigators = current.investigators.clone();
    loop {
        let line = prompt(out, reader, "  ")?;
        if line.is_empty() {
            break;
        }
        if let Some(name) = line.strip_prefix("add ") {
            let name = require_value("investigator", name)?;
            if investigators.iter().any(|have| have == &name) {
                return Err(Error::Msg(format!("duplicate investigator '{name}'")));
            }
            investigators.push(name);
            continue;
        }
        if let Some(raw) = line.strip_prefix("remove ") {
            let number: usize = raw
                .trim()
                .parse()
                .map_err(|_| Error::Msg(format!("not a number: {raw}")))?;
            if number == 0 || number > investigators.len() {
                return Err(Error::Msg(format!("no investigator {number}")));
            }
            investigators.remove(number - 1);
            continue;
        }
        return Err(Error::Msg(format!("use add, remove, or a blank line (got '{line}')")));
    }
    Ok(LabFile {
        institution_name,
        github_org,
        lab_name,
        lab_url,
        lab_line: current.lab_line.clone(),
        issue_line: current.issue_line.clone(),
        investigators,
    })
}

fn prompt_keep(out: &mut impl Write, reader: &mut impl BufRead, label: &str, current: &str) -> Result<String> {
    prompt(out, reader, &format!("{label} [{current}]: "))
}

fn store_new(dir: &Path, lab: &LabFile, out: &mut impl Write) -> Result<PathBuf> {
    let path = create_from_fields(dir, lab)?;
    writeln!(out, "created: {}", path.display())?;
    writeln!(out, "{}", lab.rendered_lab_line())?;
    Ok(path)
}

fn store_edit(entry: &LabEntry, lab: &LabFile) -> Result<PathBuf> {
    let path = lab_path(entry.path.parent().unwrap_or(Path::new(".")), &lab.github_org);
    if path != entry.path && path.exists() {
        return Err(Error::Msg(format!("lab already exists: {}", path.display())));
    }
    lab::save_lab(&path, lab)?;
    if path != entry.path {
        fs::remove_file(&entry.path)?;
    }
    Ok(path)
}

fn find_lab(dir: &Path, name: &str) -> Result<LabEntry> {
    let stem = name.strip_suffix(".yml").unwrap_or(name);
    list_labs(dir)?
        .into_iter()
        .find(|entry| entry.name == stem || entry.lab.github_org == stem)
        .ok_or_else(|| Error::LabMissing(dir.join(format!("{stem}.yml"))))
}

fn lab_path(dir: &Path, github_org: &str) -> PathBuf {
    dir.join(format!("{github_org}.yml"))
}

fn prompt(out: &mut impl Write, reader: &mut impl BufRead, label: &str) -> Result<String> {
    write!(out, "{label}")?;
    out.flush()?;
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Err(Error::Msg("unexpected end of input".into()));
    }
    Ok(line.trim().to_string())
}

fn require_value(field: &str, value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.contains(['\n', '\r']) {
        return Err(Error::Msg(format!("{field} must be a single non-empty line")));
    }
    Ok(value.to_string())
}

fn require_org(value: &str) -> Result<String> {
    let value = require_value("GitHub org", value)?;
    let mut chars = value.chars();
    let ok = matches!(chars.next(), Some(ch) if ch.is_ascii_alphanumeric())
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');
    if !ok {
        return Err(Error::Msg("GitHub org must be letters, digits, '-' or '_'".into()));
    }
    Ok(value)
}

fn require_url(value: &str) -> Result<String> {
    let value = require_value("lab URL", value)?;
    let rest = value.strip_prefix("https://").or_else(|| value.strip_prefix("http://"));
    match rest {
        Some(rest) if !rest.is_empty() && !rest.chars().any(|ch| ch.is_whitespace()) => Ok(value),
        _ => Err(Error::Msg("lab URL must be an http(s) URL".into())),
    }
}

fn slug_org(lab_name: &str) -> Result<String> {
    let mut out = String::new();
    for ch in lab_name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, ' ' | '-' | '_') && !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    require_org(&out).map_err(|_| Error::Msg(format!("cannot derive a GitHub org from '{lab_name}'")))
}

fn clean_investigators(names: &[String]) -> Result<Vec<String>> {
    let mut investigators = Vec::new();
    for name in names {
        let name = require_value("investigator", name)?;
        if investigators.iter().any(|have: &String| have == &name) {
            return Err(Error::Msg(format!("duplicate investigator '{name}'")));
        }
        investigators.push(name);
    }
    Ok(investigators)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn script(text: &str) -> Cursor<Vec<u8>> {
        Cursor::new(text.as_bytes().to_vec())
    }

    #[test]
    fn menu_new_writes_a_lab_and_edit_and_delete_change_it() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let mut input = script(
            "new\nState University\nCog Neuro\ncog-neuro\nhttps://example.edu/cog\nAda Lovelace\nGrace Hopper\n\nquit\n",
        );
        let mut out = Vec::new();
        run_labs(dir, true, &mut input, &mut out).unwrap();
        let path = dir.join("cog-neuro.yml");
        let lab = lab::load_lab(&path).unwrap();
        assert_eq!(lab.institution_name.as_deref(), Some("State University"));
        assert_eq!(
            lab.investigators,
            vec!["Ada Lovelace".to_string(), "Grace Hopper".to_string()]
        );
        assert_eq!(
            lab.rendered_lab_line(),
            "Lab: [Cog Neuro](https://example.edu/cog), State University."
        );

        let mut input = script("\n\n\n\nadd Katherine Johnson\n\n");
        let mut out = Vec::new();
        edit_lab(dir, "cog-neuro", LabPatch::default(), true, &mut input, &mut out).unwrap();
        let lab = lab::load_lab(&path).unwrap();
        assert_eq!(lab.investigators.len(), 3);

        let mut input = script("nope\n");
        let mut out = Vec::new();
        delete_lab(dir, "cog-neuro", false, true, &mut input, &mut out).unwrap();
        assert!(path.is_file());
        let mut input = script("cog-neuro\n");
        delete_lab(dir, "cog-neuro", false, true, &mut input, &mut out).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn plain_list_names_the_shortcuts() {
        let tmp = tempfile::tempdir().unwrap();
        let mut input = script("");
        let mut out = Vec::new();
        run_labs(tmp.path(), false, &mut input, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("symproject create lab"), "{text}");
        assert!(text.contains("symproject edit lab <name>"), "{text}");
        assert!(text.contains("symproject delete lab <name>"), "{text}");
        assert!(text.contains("(none)"), "{text}");
    }

    #[test]
    fn field_edit_and_yes_delete() {
        let tmp = tempfile::tempdir().unwrap();
        create_from_fields(tmp.path(), &sample_lab()).unwrap();
        let patch = LabPatch {
            institution_name: Some("Other University".into()),
            ..LabPatch::default()
        };
        let mut input = script("");
        let mut out = Vec::new();
        edit_lab(tmp.path(), "cog-neuro", patch, false, &mut input, &mut out).unwrap();
        let loaded = lab::load_lab(&tmp.path().join("cog-neuro.yml")).unwrap();
        assert_eq!(loaded.institution_name.as_deref(), Some("Other University"));
        delete_lab(tmp.path(), "cog-neuro", true, false, &mut input, &mut out).unwrap();
        assert!(!tmp.path().join("cog-neuro.yml").exists());
    }

    #[test]
    fn renaming_the_github_org_replaces_the_file() {
        let tmp = tempfile::tempdir().unwrap();
        let lab = sample_lab();
        create_from_fields(tmp.path(), &lab).unwrap();
        let patch = LabPatch {
            github_org: Some("other-org".into()),
            ..LabPatch::default()
        };
        let mut input = script("");
        let mut out = Vec::new();
        edit_lab(tmp.path(), "cog-neuro", patch, false, &mut input, &mut out).unwrap();
        assert!(!tmp.path().join("cog-neuro.yml").exists());
        let loaded = lab::load_lab(&tmp.path().join("other-org.yml")).unwrap();
        assert_eq!(loaded.github_org, "other-org");
        assert_eq!(loaded.lab_name, "Cog Neuro");
    }

    #[test]
    fn plain_edit_and_delete_require_a_flag() {
        let tmp = tempfile::tempdir().unwrap();
        let path = create_from_fields(tmp.path(), &sample_lab()).unwrap();
        let mut input = script("");
        let mut out = Vec::new();
        let err = edit_lab(
            tmp.path(),
            "cog-neuro",
            LabPatch::default(),
            false,
            &mut input,
            &mut out,
        )
        .unwrap_err();
        assert!(err.to_string().contains("pass a field"), "{err}");
        let err = delete_lab(tmp.path(), "cog-neuro", false, false, &mut input, &mut out).unwrap_err();
        assert!(err.to_string().contains("--yes"), "{err}");
        assert!(path.is_file());
        let err = create_from_fields(tmp.path(), &sample_lab()).unwrap_err();
        assert!(err.to_string().contains("already exists"), "{err}");
    }

    fn sample_lab() -> LabFile {
        LabFile {
            institution_name: Some("State University".into()),
            github_org: "cog-neuro".into(),
            lab_name: "Cog Neuro".into(),
            lab_url: "https://example.edu/cog".into(),
            lab_line: None,
            issue_line: Some(DEFAULT_ISSUE.into()),
            investigators: vec!["Ada Lovelace".into()],
        }
    }
}
