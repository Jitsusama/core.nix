# The people daemon, built from this directory, its tests run as it builds.
# Its socket's tests ask it with a crate every daemon's tests share
# (../daemon-client), and it keeps its rows as every daemon does
# (../ready-rows), so the source is cut from the modules folder holding
# them all, and built from this one.
{
  lib,
  rustPlatform,
  systemd,
}:
rustPlatform.buildRustPackage rec {
  pname = "people";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The crate's own tests, and not the QML case beside them, which the
      # shell's engine runs (checks.nix).
      ./tests/daemon.rs
      ./tests/words.json
      ../daemon-client/Cargo.toml
      ../daemon-client/src
      ../ready-rows/Cargo.toml
      ../ready-rows/src
    ];
  };
  sourceRoot = "${src.name}/people";
  cargoLock.lockFile = ./Cargo.lock;
  # A test starts the daemon as systemd does for people.socket, through
  # systemd-socket-activate, handing it the socket it answers on.
  nativeCheckInputs = [ systemd ];
  meta = {
    description = "Holds everyone Joel deals with ready for the shell, from each People source";
    license = lib.licenses.mit;
    mainProgram = "people";
    platforms = lib.platforms.linux;
  };
}
