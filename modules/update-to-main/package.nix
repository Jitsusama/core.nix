# update-to-main, built from this directory, its tests run as it builds
# against real git, gpg, ssh-keygen, tar and curl. It takes its shape from
# the terminal kit (../terminal-kit), so the source is cut from the folder
# holding both.
{
  curl,
  git,
  gnupg,
  gnutar,
  lib,
  openssh,
  runtimeShell,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "update-to-main";
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
  sourceRoot = "${src.name}/update-to-main";
  cargoLock.lockFile = ./Cargo.lock;
  nativeCheckInputs = [
    curl
    git
    gnupg
    gnutar
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
