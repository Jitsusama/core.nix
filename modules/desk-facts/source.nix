# desk-facts' Rust: facts.rs, the crate's root, and beside it a file for each
# kind of device and each layer several kinds share, with a module's children
# in a folder of its name. The program and its tests both build from this, so
# they read the same files, and a change to the Nix beside them rebuilds
# neither. A test fails on a file its module doesn't name.
{ lib }:
lib.fileset.toSource {
  root = ./.;
  fileset = lib.fileset.fileFilter (file: file.hasExt "rs") ./.;
}
