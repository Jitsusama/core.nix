//! repos: every repository Joel has, each listed once however many copies
//! of it lie on the disk (docs/glossary.md's Code words).
//!
//! A repository is one history, known by its key, where its remote keeps
//! it (`github:Jitsusama/core.nix`). A checkout is its clone on the
//! machine, and a tree a folder cut from a checkout to work in: a worktree
//! at a branch or a snapshot at a commit. Under each root, `read` finds
//! every checkout and, through each, every tree, wherever it lies, and
//! makes one record of them, `repos.repositories/1` (model.rs), which
//! every view reads: the command's screens here, and the launcher's Code
//! place through the programs daemon.
//!
//! Git's own files are the one source for now. GitHub and GitLab, read for
//! each repository's changes and checks, come as sources beside it, each
//! keeping its own words in a section of its own, as net's do.

pub mod keys;
pub mod model;
pub mod read;
mod reftable;
