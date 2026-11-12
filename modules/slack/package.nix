# slack-people and slack-unreads, built from this directory, their tests run
# as they build. Their rows take the form every daemon's do (../ready-rows),
# served by the side every daemon shares (../daemon-serve), its tests asking
# through the client every daemon's tests share (../daemon-client), so the
# source is cut from the modules folder holding them, and built from this
# one.
{
  lib,
  rustPlatform,
  systemd,
}:
rustPlatform.buildRustPackage rec {
  pname = "slack-people";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The stand-in and the launcher's words the cases read: the QML cases
      # beside them aren't the crate's.
      ./tests/stand_in.rs
      ./tests/words.json
      ../ready-rows/Cargo.toml
      ../ready-rows/src
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
      ../daemon-client/Cargo.toml
      ../daemon-client/src
    ];
  };
  sourceRoot = "${src.name}/slack";
  cargoLock.lockFile = ./Cargo.lock;
  # A test starts slack-unreads as systemd does for its socket, through
  # systemd-socket-activate, handing it the socket it answers on.
  nativeCheckInputs = [ systemd ];
  meta = {
    description = "Says who Slack knows by an address, and what in Slack wants Joel, only reading";
    license = lib.licenses.mit;
    mainProgram = "slack-people";
    platforms = lib.platforms.linux;
  };
}
