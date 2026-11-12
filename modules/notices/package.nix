# The notices daemon, built from this directory, its tests run as it
# builds. It serves with the serving side every daemon shares
# (../daemon-serve) in the one form every daemon's rows take
# (../ready-rows), names senders by the desktop entries the windows
# daemon reads (../windows, ../programs), and its test asks it with the
# client every daemon's tests share (../daemon-client), so the source is
# cut from the modules folder holding them all, and built from this one.
# Its tests read the launcher's words.json, which its rows are said in,
# and start a session bus of their own, so dbus-daemon is there.
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
      ../chrome/session/Cargo.toml
      ../chrome/session/src
      ../daemon-client/Cargo.toml
      ../daemon-client/src
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
      ../programs/Cargo.toml
      ../programs/src
      ../ready-rows/Cargo.toml
      ../ready-rows/src
      ../windows/Cargo.toml
      ../windows/src
    ];
  };
  sourceRoot = "${src.name}/notices";
  # The launcher's words, joined from each place's file, put where its
  # sources and tests read words.json (../quickshell/launcher/words.nix).
  postUnpack = ''
    chmod -R u+w ${src.name}
    mkdir -p ${src.name}/quickshell/launcher
    cp ${(import ../quickshell/launcher/words.nix).file} ${src.name}/quickshell/launcher/words.json
  '';
  cargoLock.lockFile = ./Cargo.lock;
  nativeCheckInputs = [ dbus ];
  meta = {
    description = "Holds the desktop's notices and serves them to the shell";
    license = lib.licenses.mit;
    mainProgram = "notices";
    platforms = lib.platforms.linux;
  };
}
