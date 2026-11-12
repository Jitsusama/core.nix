# The secret-asks daemon, built from this directory, its tests run as it
# builds. It serves with the serving side every daemon shares
# (../daemon-serve) in the one form every daemon's rows take
# (../ready-rows), and its test asks it with the client every daemon's
# tests share (../daemon-client), so the source is cut from the modules
# folder holding all four, and built from this one. Its tests make git
# trees for an ask to run in, and read the launcher's words.json, which
# the daemon says its rows in.
{
  git,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "secret-asks";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../daemon-client/Cargo.toml
      ../daemon-client/src
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
      ../quickshell/launcher/words.json
      ../ready-rows/Cargo.toml
      ../ready-rows/src
    ];
  };
  sourceRoot = "${src.name}/secret-asks";
  cargoLock.lockFile = ./Cargo.lock;
  nativeCheckInputs = [ git ];
  meta = {
    description = "Asks Joel in the launcher before releasing one throwaway secret to a checker";
    license = lib.licenses.mit;
    mainProgram = "secret-asks";
    platforms = lib.platforms.linux;
  };
}
