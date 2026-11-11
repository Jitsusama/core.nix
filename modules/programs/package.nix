# The programs daemon, built from this directory, its tests run as it builds.
# Its socket's tests ask it with a crate every daemon's tests share
# (../daemon-client), so the source is cut from the modules folder holding
# both, and built from this one.
{
  lib,
  rustPlatform,
  systemd,
  git,
}:
rustPlatform.buildRustPackage rec {
  pname = "programs";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../daemon-client/Cargo.toml
      ../daemon-client/src
    ];
  };
  sourceRoot = "${src.name}/programs";
  cargoLock.lockFile = ./Cargo.lock;
  # A test starts the daemon as systemd does for programs.socket, through
  # systemd-socket-activate, handing it the socket it answers on, and
  # one makes a git repository whose files it lists with git.
  nativeCheckInputs = [
    systemd
    git
  ];
  meta = {
    description = "Holds what's installed and what each program had open ready for the shell";
    license = lib.licenses.mit;
    mainProgram = "programs";
    platforms = lib.platforms.linux;
  };
}
