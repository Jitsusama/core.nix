# update-to-main, built from this directory, its tests run as it builds
# against real git, gpg and ssh-keygen.
{
  git,
  gnupg,
  lib,
  openssh,
  runtimeShell,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "update-to-main";
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
  nativeCheckInputs = [
    git
    gnupg
    openssh
  ];
  # The tests' stand-ins for Nix and a system's switch run under the
  # build's own shell, since the sandbox has no /usr/bin/env.
  env.STAND_IN_SHELL = runtimeShell;
  meta = {
    description = "Updates its machine to its repository's main when asked";
    license = lib.licenses.mit;
    mainProgram = "update-to-main";
  };
}
