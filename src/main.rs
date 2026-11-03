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
            println!("Lab template: {}", root.join("templates/lab.yml").display());
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
                    lab: args.lab,
                    paper: args.paper,
                    paper_title: args.paper_title,
                    with_agents: !args.no_agents,
                    adapters: args.adapters,
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
            scaffold::set_lab(&outdir, &args.lab_file, args.dry_run)
        }
    }
}
