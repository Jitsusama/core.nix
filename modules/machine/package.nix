# The machine daemon, built from this directory, its tests run as it builds.
# Its rows take the form every daemon's do (../ready-rows), it serves with
# the serving side every daemon shares (../daemon-serve), and its socket's
# tests ask it with a crate every daemon's tests share (../daemon-client),
# so the source is cut from the modules folder holding all four, and built
# from this one.
{
  lib,
  rustPlatform,
  systemd,
}:
rustPlatform.buildRustPackage rec {
  pname = "machine";
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
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
      ../ready-rows/Cargo.toml
      ../ready-rows/src
    ];
  };
  sourceRoot = "${src.name}/machine";
  cargoLock.lockFile = ./Cargo.lock;
  # A test starts the daemon as systemd does for machine.socket, through
  # systemd-socket-activate, handing it the socket it answers on.
  nativeCheckInputs = [ systemd ];
  meta = {
    description = "Holds how this machine is doing ready for the shell, read from the kernel";
    license = lib.licenses.mit;
    mainProgram = "machine";
    platforms = lib.platforms.linux;
  };
}
