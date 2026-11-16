# nix-offload, which lends the machine a builder while its queue holds
# builds back, built from this directory, its tests run as it builds.
{
  bubblewrap,
  cacert,
  callPackage,
  coreutils,
  dash,
  git,
  lib,
  rustPlatform,
  stdenv,
  systemd,
  util-linux,
  zsh,
}:
let
  changed-checks = callPackage ../../../tools/changed-checks/package.nix { };
in
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
    # A run kept to the lent builders runs this changed-checks, built from
    # the same core as nix-offload, never the one its commit names
    # (src/changed.rs).
    NIX_OFFLOAD_CHANGED_CHECKS = lib.getExe changed-checks;
    # A gate's answer runs git from the store, and checks the forge's
    # certificate against these, inside a fence that has nothing else of
    # the machine's (src/gate/fence.rs).
    NIX_OFFLOAD_GIT = lib.getExe git;
    NIX_OFFLOAD_CACERT = "${cacert}/etc/ssl/certs/ca-bundle.crt";
  }
  # The fence a gate's answer runs git in, which only Linux has.
  // lib.optionalAttrs stdenv.hostPlatform.isLinux {
    NIX_OFFLOAD_BWRAP = lib.getExe bubblewrap;
  }
  # Each upload runs in a scope of its own, whose memory bounds the cache
  # its reads of the store fill (src/there.rs).
  // lib.optionalAttrs (lib.meta.availableOn stdenv.hostPlatform systemd) {
    NIX_OFFLOAD_SYSTEMD_RUN = lib.getExe' systemd "systemd-run";
  };
  # Its tests bundle a real git tree's commits, and start a run apart with
  # setsid as a builder does, in dash as well, since most builders' sh is
  # dash (src/command.rs), and send a run far through zsh, a Mac's login
  # shell (src/there.rs).
  nativeCheckInputs = [
    dash
    git
    util-linux
    zsh
  ];
  meta = {
    description = "Lends the machine a builder while its queue has builds one could take";
    license = lib.licenses.mit;
    mainProgram = "nix-offload";
  };
}
