# nix-offload, which lends the machine a builder while its queue holds
# builds back, built from this directory, its tests run as it builds.
{
  coreutils,
  git,
  lib,
  rustPlatform,
  stdenv,
  systemd,
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
  # A proof through the daemon takes this df along to measure its
  # builder's store, whose sandbox has no df of its own (src/proof.rs).
  env = {
    NIX_OFFLOAD_COREUTILS = "${coreutils}";
  }
  # Each upload runs in a scope of its own, whose memory bounds the cache
  # its reads of the store fill (src/there.rs).
  // lib.optionalAttrs (lib.meta.availableOn stdenv.hostPlatform systemd) {
    NIX_OFFLOAD_SYSTEMD_RUN = lib.getExe' systemd "systemd-run";
  };
  # Its tests bundle a real git tree's commits.
  nativeCheckInputs = [ git ];
  meta = {
    description = "Lends the machine a builder while its queue has builds one could take";
    license = lib.licenses.mit;
    mainProgram = "nix-offload";
  };
}
