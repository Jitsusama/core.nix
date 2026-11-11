# The windows daemon, built from this directory, its tests run as it builds,
# and given the launcher's words, they read Screens' from them too. It reads
# the desktop entries through the programs daemon's crate (../programs),
# serves its rows in the form every daemon's take (../ready-rows), and its
# socket's tests ask it with a crate every daemon's tests share
# (../daemon-client), so the source is cut from the modules folder holding
# them all, and built from this one.
{
  lib,
  rustPlatform,
  systemd,
  words ? null,
}:
rustPlatform.buildRustPackage rec {
  pname = "windows";
  env = lib.optionalAttrs (words != null) { WINDOWS_TEST_WORDS = "${words}"; };
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../programs/Cargo.toml
      ../programs/src
      ../ready-rows/Cargo.toml
      ../ready-rows/src
      ../daemon-client/Cargo.toml
      ../daemon-client/src
    ];
  };
  sourceRoot = "${src.name}/windows";
  cargoLock.lockFile = ./Cargo.lock;
  # A test starts the daemon as systemd does for windows.socket, through
  # systemd-socket-activate, handing it the socket it answers on.
  nativeCheckInputs = [ systemd ];
  meta = {
    description = "Holds niri's windows ready for the shell";
    license = lib.licenses.mit;
    mainProgram = "windows";
    platforms = lib.platforms.linux;
  };
}
