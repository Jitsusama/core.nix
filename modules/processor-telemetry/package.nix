# processor-telemetry, built from this directory's crate. Its tests run as it
# builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "processor-telemetry";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says the processor's own temperatures and what holds it back";
    license = lib.licenses.mit;
    mainProgram = "processor-telemetry";
    platforms = lib.platforms.linux;
  };
}
