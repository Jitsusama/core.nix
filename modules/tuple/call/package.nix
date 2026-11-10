# tuple-call, built from this directory, its tests run as it builds. Its
# socket's tests ask it with a crate every daemon's tests share
# (../../daemon-client), so the source is cut from the modules folder
# holding both, and built from this one.
{
  lib,
  rustPlatform,
  systemd,
}:
rustPlatform.buildRustPackage rec {
  pname = "tuple-call";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../../daemon-client/Cargo.toml
      ../../daemon-client/src
    ];
  };
  sourceRoot = "${src.name}/tuple/call";
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
