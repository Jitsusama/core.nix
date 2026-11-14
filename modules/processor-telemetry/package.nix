# processor-telemetry, built from this directory's crate. Its tests run as it
# builds. It takes its shape from the terminal kit (../terminal-kit), so the
# source is cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "processor-telemetry";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/processor-telemetry";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says the processor's own temperatures and what holds it back";
    license = lib.licenses.mit;
    mainProgram = "processor-telemetry";
    platforms = lib.platforms.linux;
  };
}
