# The notices daemon, built from this directory, its tests run as it
# builds. It serves with the serving side every daemon shares
# (../daemon-serve), and its test asks it with the client every daemon's
# tests share (../daemon-client), so the source is cut from the modules
# folder holding all three, and built from this one. Its test starts a
# session bus of its own, so dbus-daemon is there as the tests run.
{
  dbus,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "notices";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./senders.json
      ./src
      ./tests
      ../daemon-client/Cargo.toml
      ../daemon-client/src
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
    ];
  };
  sourceRoot = "${src.name}/notices";
  cargoLock.lockFile = ./Cargo.lock;
  nativeCheckInputs = [ dbus ];
  meta = {
    description = "Holds the desktop's notices and serves them to the shell";
    license = lib.licenses.mit;
    mainProgram = "notices";
    platforms = lib.platforms.linux;
  };
}
