# desk-facts' Rust: facts.rs, the crate's root, and beside it a file for each
# kind of device and each layer several kinds share. The program and its tests
# both build from this, so they read the same files, and a change to the Nix
# beside them rebuilds neither. A test fails on a file facts.rs doesn't name.
{ lib }:
lib.fileset.toSource {
  root = ./.;
  fileset = lib.fileset.fileFilter (file: file.hasExt "rs") ./.;
}
