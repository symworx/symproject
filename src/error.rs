// Copyright (c) 2026, Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("catalog not found: {0}")]
    CatalogMissing(PathBuf),
    #[error("templates not found: {0}")]
    TemplatesMissing(PathBuf),
    #[error(
        "cannot find catalog.yaml and templates/ (set SYMPROJECT_ROOT, or run from a checkout; cargo-install uses an embedded copy)"
    )]
    RootNotFound,
    #[error("unknown --{kind} '{ident}' (known: {known})")]
    UnknownId {
        kind: &'static str,
        ident: String,
        known: String,
    },
    #[error("invalid project name: {0}")]
    InvalidProjectName(String),
    #[error("invalid paper slug: {0}")]
    InvalidPaperSlug(String),
    #[error("invalid --package '{0}' (use a Python identifier; the default replaces '-' in the project name with '_')")]
    InvalidPackageName(String),
    #[error("project name '{0}' is not a Python package name; pass --package")]
    PackageNeedsFlag(String),
    #[error("--title is required")]
    MissingTitle,
    #[error("unsafe destination path: {0}")]
    UnsafePath(String),
    #[error("refusing to write inside the symproject checkout: {0}")]
    KitRoot(PathBuf),
    #[error("missing template: {0}")]
    MissingTemplate(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("catalog.yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("{0}")]
    Msg(String),
}
