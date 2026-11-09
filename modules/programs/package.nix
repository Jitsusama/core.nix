# The programs daemon, built from this directory, its tests run as it builds.
{
  lib,
  rustPlatform,
  systemd,
}:
rustPlatform.buildRustPackage {
  pname = "programs";
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
  # A test starts the daemon as systemd does for programs.socket, through
  # systemd-socket-activate, handing it the socket it answers on.
  nativeCheckInputs = [ systemd ];
  meta = {
    description = "Holds what's installed and what each program had open ready for the shell";
    license = lib.licenses.mit;
    mainProgram = "programs";
    platforms = lib.platforms.linux;
  };
}
