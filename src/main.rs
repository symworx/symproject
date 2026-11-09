// Copyright (c) 2026, Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

use std::{
    env,
    path::PathBuf,
    process::ExitCode,
};

use clap::{
    Args,
    CommandFactory,
    Parser,
    Subcommand,
};
use symproject::{
    assets::find_root,
    error::Result,
    scaffold::{
        self,
        NewProject,
    },
};

#[derive(Parser)]
#[command(
    name = "symproject",
    about = "Scaffold a research-project repository from catalog.yaml (paper line + runtime + org).",
    after_help = "\
Labs:
  Local identity files: institution, name, GitHub org name, GitHub URL, lab URL, and investigators.

  labs        Menu for those files. Arrow keys choose new, edit, or delete
  create lab  Write a lab file. Shortcut for new
  edit lab    Change a lab file. Shortcut for edit
  delete lab  Remove a lab file. Shortcut for delete
",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Shapes, runtimes, orgs, and presets
    List,
    /// Show usage
    Help,
    /// Scaffold a research-project repository
    New(Box<NewArgs>),
    /// Add papers/<slug>/ and results/<slug>/ to an existing project
    Paper(Box<PaperArgs>),
    /// Replace the lab line, issue line, and clone URL from a lab.yml file
    SetLab(Box<SetLabArgs>),
    /// Fill missing scaffold files in an existing project
    Update(Box<UpdateArgs>),
    /// Write a lab file. Shortcut for the new choice in `labs`
    #[command(hide = true)]
    Create(CreateArgs),
    /// Menu for local lab files. Arrow keys choose new, edit, or delete
    #[command(hide = true)]
    Labs,
    /// Change a lab file. Shortcut for the edit choice in `labs`
    #[command(hide = true)]
    Edit(EditArgs),
    /// Remove a lab file. Shortcut for the delete choice in `labs`
    #[command(hide = true)]
    Delete(DeleteArgs),
}

#[derive(Args)]
struct NewArgs {
    /// Repo / directory name, e.g. chp-coupling
    project_name: String,
    /// Destination (default: ./<project-name> under the working directory)
    outdir: Option<PathBuf>,
    /// Human title
    #[arg(long)]
    title: Option<String>,
    /// Import package under src/ (default: project name with '-' changed to '_')
    #[arg(long)]
    package: Option<String>,
    /// Folder spine (default from catalog)
    #[arg(long)]
    shape: Option<String>,
    /// Language runtime stubs: none or uv
    #[arg(long)]
    runtime: Option<String>,
    /// Identity overlay (default from catalog)
    #[arg(long)]
    org: Option<String>,
    /// Named combo of shape, runtime, and org. Other flags override
    #[arg(long)]
    preset: Option<String>,
    /// GitHub org for clone URLs. Overrides --lab and the org
    #[arg(long = "github-org")]
    github_org: Option<String>,
    /// Lab identity file. Overrides the default lab org. See templates/lab.yml
    #[arg(long)]
    lab: Option<PathBuf>,
    /// Also write a paper stub for this slug
    #[arg(long)]
    paper: Option<String>,
    /// Title for --paper (default: the slug)
    #[arg(long = "paper-title")]
    paper_title: Option<String>,
    /// Skip nested `symkit install`
    #[arg(long = "no-agents")]
    no_agents: bool,
    /// Passed to nested symkit (grok, claude, codex, all, none)
    #[arg(long)]
    adapters: Option<String>,
    /// Do not create migration-docs/ and do not refresh its .gitkeep exceptions
    #[arg(long = "no-migration-docs")]
    no_migration_docs: bool,
}

#[derive(Args)]
struct CreateArgs {
    #[command(subcommand)]
    command: CreateCommand,
}

#[derive(Subcommand)]
enum CreateCommand {
    /// Write a lab file. In a terminal, prompt. Otherwise copy templates/lab.yml
    Lab(CreateLabArgs),
}

#[derive(Args)]
struct CreateLabArgs {
    /// Template destination. With identity flags, the lab file to write
    path: Option<PathBuf>,
    /// Institution. With the other identity flags, writes a lab file instead of the template
    #[arg(long)]
    institution: Option<String>,
    /// Lab name
    #[arg(long = "lab-name")]
    lab_name: Option<String>,
    /// GitHub org name, such as cog-neuro. A https://github.com/<org> URL is accepted
    #[arg(long = "github-org")]
    github_org: Option<String>,
    /// GitHub URL for the org. Defaults to https://github.com/<org>
    #[arg(long = "github-url")]
    github_url: Option<String>,
    /// Lab home page
    #[arg(long = "lab-url")]
    lab_url: Option<String>,
    /// Affiliated faculty or investigator. Repeat for each person
    #[arg(long = "investigator")]
    investigator: Vec<String>,
}

#[derive(Args)]
struct EditArgs {
    #[command(subcommand)]
    command: EditCommand,
}

#[derive(Subcommand)]
enum EditCommand {
    /// Edit one lab. In a terminal with no name, pick from the list
    Lab(EditLabArgs),
}

#[derive(Args)]
struct EditLabArgs {
    /// Lab name, without .yml
    name: Option<String>,
    #[arg(long)]
    institution: Option<String>,
    #[arg(long = "lab-name")]
    lab_name: Option<String>,
    /// GitHub org name, such as cog-neuro. A https://github.com/<org> URL is accepted
    #[arg(long = "github-org")]
    github_org: Option<String>,
    /// GitHub URL for the org. Defaults to https://github.com/<org>
    #[arg(long = "github-url")]
    github_url: Option<String>,
    #[arg(long = "lab-url")]
    lab_url: Option<String>,
    /// Replace the investigator list. Repeat for each person
    #[arg(long = "investigator")]
    investigator: Vec<String>,
    /// Replace the investigator list with an empty list
    #[arg(long = "clear-investigators")]
    clear_investigators: bool,
}

#[derive(Args)]
struct DeleteArgs {
    #[command(subcommand)]
    command: DeleteCommand,
}

#[derive(Subcommand)]
enum DeleteCommand {
    /// Delete one lab file. Does not change projects already stamped with it
    Lab(DeleteLabArgs),
}

#[derive(Args)]
struct DeleteLabArgs {
    /// Lab name, without .yml
    name: Option<String>,
    /// Delete without typing the name again
    #[arg(long)]
    yes: bool,
}

#[derive(Args)]
struct SetLabArgs {
    /// Path to a lab.yml file
    lab_file: PathBuf,
    /// Project root (default: working directory)
    outdir: Option<PathBuf>,
    /// Show replacements without writing
    #[arg(long = "dry-run")]
    dry_run: bool,
}

#[derive(Args)]
struct UpdateArgs {
    /// Project root (default: working directory)
    outdir: Option<PathBuf>,
    /// Title for files this run creates. Defaults to the README heading
    #[arg(long)]
    title: Option<String>,
    /// Import package under src/ when it is missing or src/ has more than one
    #[arg(long)]
    package: Option<String>,
    /// Folder spine (default from catalog)
    #[arg(long)]
    shape: Option<String>,
    /// Language runtime stubs: none or uv. Defaults to uv when pyproject.toml exists
    #[arg(long)]
    runtime: Option<String>,
    /// Do not refresh migration-docs/ .gitkeep exceptions
    #[arg(long = "no-migration-docs")]
    no_migration_docs: bool,
}

#[derive(Args)]
struct PaperArgs {
    /// Paper slug, e.g. multirate-fusion
    slug: String,
    /// Project root (default: working directory)
    outdir: Option<PathBuf>,
    /// Human title
    #[arg(long)]
    title: Option<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None | Some(Command::Help) => {
            Cli::command().print_long_help()?;
            println!();
            Ok(())
        }
        Some(Command::List) => {
            let root = find_root()?;
            let catalog = symproject::catalog::Catalog::load(&root.join("catalog.yaml"))?;
            print!("{}", catalog.format_list());
            println!("Labs: symproject labs");
            println!();
            Ok(())
        }
        Some(Command::New(args)) => {
            let root = find_root()?;
            let outdir = match args.outdir {
                Some(path) => path,
                None => env::current_dir()?.join(&args.project_name),
            };
            scaffold::scaffold(
                &root,
                &NewProject {
                    project_name: args.project_name,
                    outdir,
                    title: args.title.unwrap_or_default(),
                    package: args.package,
                    shape: args.shape,
                    runtime: args.runtime,
                    org: args.org,
                    preset: args.preset,
                    github_org: args.github_org,
                    lab: args.lab.map(|path| symproject::labs::resolve_lab_arg(&path)),
                    paper: args.paper,
                    paper_title: args.paper_title,
                    with_agents: !args.no_agents,
                    adapters: args.adapters,
                    migration_docs: !args.no_migration_docs,
                },
            )
        }
        Some(Command::Paper(args)) => {
            let root = find_root()?;
            let outdir = match args.outdir {
                Some(path) => path,
                None => env::current_dir()?,
            };
            let title = args.title.unwrap_or_default();
            if title.is_empty() {
                return Err(symproject::error::Error::MissingTitle);
            }
            scaffold::add_paper(&root, &outdir, &args.slug, &title)
        }
        Some(Command::SetLab(args)) => {
            let outdir = match args.outdir {
                Some(path) => path,
                None => env::current_dir()?,
            };
            let lab_file = symproject::labs::resolve_lab_arg(&args.lab_file);
            scaffold::set_lab(&outdir, &lab_file, args.dry_run)
        }
        Some(Command::Update(args)) => {
            let root = find_root()?;
            let outdir = match args.outdir {
                Some(path) => path,
                None => env::current_dir()?,
            };
            scaffold::update(
                &root,
                &scaffold::UpdateProject {
                    outdir,
                    title: args.title,
                    package: args.package,
                    shape: args.shape,
                    runtime: args.runtime,
                    migration_docs: !args.no_migration_docs,
                },
            )
        }
        Some(Command::Create(args)) => {
            let root = find_root()?;
            match args.command {
                CreateCommand::Lab(lab) => create_lab_command(&root, lab),
            }
        }
        Some(Command::Labs) => {
            let dir = symproject::labs::labs_dir()?;
            symproject::labs::run_labs_interactive(&dir)?;
            Ok(())
        }
        Some(Command::Edit(args)) => match args.command {
            EditCommand::Lab(lab) => {
                let dir = symproject::labs::labs_dir()?;
                let interactive = std::io::IsTerminal::is_terminal(&std::io::stdin());
                let investigators = if lab.clear_investigators || !lab.investigator.is_empty() {
                    Some(lab.investigator)
                } else {
                    None
                };
                symproject::labs::edit_command(
                    &dir,
                    lab.name.as_deref(),
                    symproject::labs::LabPatch {
                        institution_name: lab.institution,
                        lab_name: lab.lab_name,
                        github_org: lab.github_org,
                        github_url: lab.github_url,
                        lab_url: lab.lab_url,
                        investigators,
                    },
                    interactive,
                    &mut std::io::stdin().lock(),
                    &mut std::io::stdout(),
                )?;
                Ok(())
            }
        },
        Some(Command::Delete(args)) => match args.command {
            DeleteCommand::Lab(lab) => {
                let dir = symproject::labs::labs_dir()?;
                let interactive = std::io::IsTerminal::is_terminal(&std::io::stdin());
                symproject::labs::delete_command(
                    &dir,
                    lab.name.as_deref(),
                    lab.yes,
                    interactive,
                    &mut std::io::stdin().lock(),
                    &mut std::io::stdout(),
                )?;
                Ok(())
            }
        },
    }
}

fn create_lab_command(root: &std::path::Path, lab: CreateLabArgs) -> Result<()> {
    use symproject::lab::LabFile;
    let flagged = lab.institution.is_some()
        || lab.lab_name.is_some()
        || lab.github_org.is_some()
        || lab.github_url.is_some()
        || lab.lab_url.is_some()
        || !lab.investigator.is_empty();
    if flagged {
        let institution = lab
            .institution
            .ok_or_else(|| symproject::error::Error::Msg("pass --institution".into()))?;
        let lab_name = lab
            .lab_name
            .ok_or_else(|| symproject::error::Error::Msg("pass --lab-name".into()))?;
        let github_org = lab
            .github_org
            .ok_or_else(|| symproject::error::Error::Msg("pass --github-org".into()))?;
        let (github_org, github_url) = symproject::lab::github_name_and_url(&github_org, lab.github_url.as_deref())?;
        let lab_url = lab
            .lab_url
            .ok_or_else(|| symproject::error::Error::Msg("pass --lab-url".into()))?;
        let record = LabFile {
            institution_name: Some(institution),
            github_org,
            github_url: Some(github_url),
            lab_name,
            lab_url,
            lab_line: None,
            issue_line: Some("Track each paper in whatever tracker you already use.".into()),
            investigators: lab.investigator,
        };
        let path = match lab.path {
            Some(path) if path.is_dir() => symproject::labs::create_from_fields(&path, &record)?,
            Some(path) => {
                if path.exists() {
                    return Err(symproject::error::Error::Msg(format!(
                        "lab already exists: {}",
                        path.display()
                    )));
                }
                symproject::lab::save_lab(&path, &record)?;
                path
            }
            None => symproject::labs::create_from_fields(&symproject::labs::labs_dir()?, &record)?,
        };
        println!("created: {}", path.display());
        println!("{}", record.rendered_lab_line());
        return Ok(());
    }
    if lab.path.is_none() && std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        let dir = symproject::labs::labs_dir()?;
        symproject::labs::create_interactive(&dir, &mut std::io::stdin().lock(), &mut std::io::stdout())?;
        return Ok(());
    }
    let dest = match lab.path {
        Some(path) => path,
        None => env::current_dir()?.join("lab.yml"),
    };
    scaffold::create_lab(root, &dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn help_puts_lab_commands_in_their_own_section() {
        let mut cmd = Cli::command().term_width(120);
        let mut buf = Vec::new();
        cmd.write_long_help(&mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        let (commands, labs) = text.split_once("Labs:").expect("labs heading");
        assert!(commands.contains("Commands:"), "{text}");
        assert!(commands.contains("set-lab"), "{text}");
        assert!(!commands.contains("\n  labs"), "{commands}");
        assert!(!commands.contains("\n  create"), "{commands}");
        assert!(!commands.contains("\n  edit"), "{commands}");
        assert!(!commands.contains("\n  delete"), "{commands}");
        assert!(labs.contains("Local identity files:"), "{labs}");
        assert!(labs.contains("\n  labs        Menu for those files."), "{labs}");
        assert!(labs.contains("\n  create lab  Write a lab file."), "{labs}");
        assert!(labs.contains("\n  edit lab    Change a lab file."), "{labs}");
        assert!(labs.contains("\n  delete lab  Remove a lab file."), "{labs}");
    }
}
