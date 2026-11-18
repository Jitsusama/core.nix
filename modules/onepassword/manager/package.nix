# 1Password as a password manager of Passwords, built from this directory,
# its tests run as it builds. Its command line takes the shape every
# command's does (../../terminal-kit), so the source is cut from the
# modules folder holding both, and built from this one.
{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "onepassword-manager";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/onepassword/manager";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Reads 1Password's items through op for Passwords, never passing on a secret unasked";
    license = lib.licenses.mit;
    mainProgram = "passwords-1password";
    platforms = lib.platforms.linux;
  };
}
