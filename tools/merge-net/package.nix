# merge-net, built from this directory's crate. Its tests run as it builds.
# It reads which crates read which from modules/crate-reach, and takes its
# help from the terminal kit (modules/terminal-kit), so the source is cut
# from the repository's root, and built from this folder.
# The quarantine list isn't part of it: the net reads it from main, so a
# test put on it moves no check.
{
  callPackage,
  git,
  lib,
  rustPlatform,
}:
let
  # `merge-net needs` runs this one by its store path, since the profile
  # it's installed in holds none: without one it named every VM test. Not
  # a wrapper, which would leave the watcher never matching what's installed.
  changed-checks = callPackage ../changed-checks/package.nix { };
in
rustPlatform.buildRustPackage rec {
  pname = "merge-net";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./lock-after.txt
      ./must-run.txt
      ./src
      ./tests
      ../../modules/crate-reach/Cargo.toml
      ../../modules/crate-reach/src
      ../../modules/terminal-kit/Cargo.toml
      ../../modules/terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/tools/merge-net";
  cargoLock.lockFile = ./Cargo.lock;
  env.MERGE_NET_CHANGED_CHECKS = lib.getExe changed-checks;
  # The patch id's test makes a repo of its own.
  nativeCheckInputs = [ git ];
  # A build that would look changed-checks up on PATH fails here.
  postInstall = ''
    grep -q -F ${lib.getExe changed-checks} $out/bin/merge-net
  '';
  meta = {
    description = "Runs the VM tests each merge to main changed, and reverts a merge that breaks one";
    license = lib.licenses.mit;
    mainProgram = "merge-net";
  };
}
