# google, built from this directory, its tests run as it builds.
{
  lib,
  rustPlatform,
  systemd,
}:
rustPlatform.buildRustPackage {
  pname = "google";
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
  # A test starts google as systemd does for its daemon socket, through
  # systemd-socket-activate, handing it the socket it answers on.
  nativeCheckInputs = [ systemd ];
  meta = {
    description = "Holds the Google account for the desktop: Joel's meetings and the mail that wants him, for the shell";
    license = lib.licenses.mit;
    mainProgram = "google";
    # It serves the shell over a Unix socket, on the Linux desktop.
    platforms = lib.platforms.linux;
  };
}
