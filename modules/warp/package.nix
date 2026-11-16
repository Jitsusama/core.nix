# warp, built from this directory, its tests run as it builds. It takes its
# shape from the terminal kit (../terminal-kit) and its socket from
# daemon-serve (../daemon-serve), so the source is cut from the folder
# holding all three.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "warp";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The tests read warp-cli's answers and the record drawn from them.
      ./contract
      ./warp.json
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/warp";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says how WARP's link stands, read from warp-cli";
    license = lib.licenses.mit;
    mainProgram = "warp";
  };
}
