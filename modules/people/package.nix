# The people daemon, built from this directory, its tests run as it builds.
{
  lib,
  rustPlatform,
  systemd,
}:
rustPlatform.buildRustPackage {
  pname = "people";
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
