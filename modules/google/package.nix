# google, built from this directory. Its tests run as it builds once it's
# handed the launcher's vocabulary, `words`, which its rows are said in and
# its tests read whole rather than keep a copy of; a machine's own build,
# handed none, runs none, as google-reads-the-calendar runs them.
{
  lib,
  rustPlatform,
  systemd,
  words ? null,
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
  doCheck = words != null;
  env = lib.optionalAttrs (words != null) { LAUNCHER_WORDS = "${words}"; };
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
