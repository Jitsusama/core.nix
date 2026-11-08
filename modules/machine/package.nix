# The machine daemon, built from this directory, its tests run as it builds.
{
  lib,
  rustPlatform,
  systemd,
}:
rustPlatform.buildRustPackage {
  pname = "machine";
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
