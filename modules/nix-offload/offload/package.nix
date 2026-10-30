# nix-offload, which lends the machine a builder while its queue holds
# builds back, built from this directory, its tests run as it builds.
{
  git,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "nix-offload";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  # Its tests bundle a real git tree's commits.
  nativeCheckInputs = [ git ];
  meta = {
    description = "Lends the machine a builder while its queue has builds one could take";
    license = lib.licenses.mit;
    mainProgram = "nix-offload";
  };
}
