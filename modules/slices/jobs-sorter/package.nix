# The sorter that moves a terminal's busy work into the jobs slice, built
# from this directory, its tests run as it builds. It takes what counts as
# a call from modules/calls' crate, so the source holds both, from
# modules, and the build runs in this one.
{ lib, rustPlatform }:
let
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../../calls/Cargo.toml
      ../../calls/src
    ];
  };
in
rustPlatform.buildRustPackage {
  pname = "jobs-sorter";
  version = "0.1.0";
  inherit src;
  sourceRoot = "${src.name}/slices/jobs-sorter";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Sorts work into the jobs slice, each piece in a scope of its own";
    license = lib.licenses.mit;
    mainProgram = "jobs-sorter";
  };
}
