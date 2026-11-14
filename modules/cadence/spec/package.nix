# cadence-spec, the daemon serving Joel's axes to the launcher's Spec,
# built from this directory, its tests run as it builds. It serves with
# the serving side every daemon shares (../../daemon-serve), in the form
# every daemon's rows take (../../ready-rows), takes its shape from the
# terminal kit (../../terminal-kit), and its test asks it with the client
# every daemon's tests share (../../daemon-client), so the source is cut
# from the modules folder holding them all, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "cadence-spec";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../../daemon-client/Cargo.toml
      ../../daemon-client/src
      ../../daemon-serve/Cargo.toml
      ../../daemon-serve/src
      ../../ready-rows/Cargo.toml
      ../../ready-rows/src
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/cadence/spec";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Serves Joel's axes to the launcher's Spec, as the rows cadence makes";
    license = lib.licenses.mit;
    mainProgram = "cadence-spec";
    platforms = lib.platforms.linux;
  };
}
