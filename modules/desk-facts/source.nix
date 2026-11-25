# desk-facts' Rust: facts.rs, the crate's root, and the file of each kind of
# device beside it. The program and its tests both build from this, so they
# read the same files, and a change to the Nix beside them rebuilds neither.
{ lib }:
lib.fileset.toSource {
  root = ./.;
  fileset = lib.fileset.fileFilter (file: file.hasExt "rs") ./.;
}
