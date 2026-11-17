# model-relay, built from this directory, its tests run as it builds. It
# takes its shape from the terminal kit (../terminal-kit), so the source is
# cut from the folder holding both.
{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "model-relay";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/model-relay";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Carries pi's model requests to their origin over a few shared connections";
    license = lib.licenses.mit;
    mainProgram = "model-relay";
    platforms = lib.platforms.linux;
  };
}
