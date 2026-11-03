# tuple-call, built from this directory, its tests run as it builds.
{
  lib,
  rustPlatform,
  systemd,
}:
rustPlatform.buildRustPackage {
  pname = "tuple-call";
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
  # A test starts the daemon as systemd does for tuple-call.socket,
  # through systemd-socket-activate, handing it the socket it answers on.
  nativeCheckInputs = [ systemd ];
  meta = {
    description = "Holds the Tuple call Joel's in ready for the shell, and does its acts";
    license = lib.licenses.mit;
    mainProgram = "tuple-call";
    platforms = lib.platforms.linux;
  };
}
