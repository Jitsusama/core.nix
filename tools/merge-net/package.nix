# merge-net, built from this directory's crate. Its tests run as it builds.
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
  # it's installed in holds no changed-checks.
  changed-checks = callPackage ../changed-checks/package.nix { };
in
rustPlatform.buildRustPackage {
  pname = "merge-net";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./must-run.txt
      ./src
    ];
  };
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
