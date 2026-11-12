# The quests daemon, built from this directory, its tests run as it
# builds. It serves with the serving side every daemon shares
# (../daemon-serve) in the one form every daemon's rows take
# (../ready-rows), and its test asks it with the client every daemon's
# tests share (../daemon-client), so the source is cut from the modules
# folder holding all four, and built from this one. Its tests read the
# launcher's words.json, which the summary is said in.
{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "quests";
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
      ../ready-rows/Cargo.toml
      ../ready-rows/src
    ];
  };
  sourceRoot = "${src.name}/quests";
  # The launcher's words, joined from each place's file, put where its
  # sources and tests read words.json (../quickshell/launcher/words.nix).
  postUnpack = ''
    chmod -R u+w ${src.name}
    mkdir -p ${src.name}/quickshell/launcher
    cp ${(import ../quickshell/launcher/words.nix).file} ${src.name}/quickshell/launcher/words.json
  '';
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Makes Quests' rows ready for the launcher";
    license = lib.licenses.mit;
    mainProgram = "quests";
    platforms = lib.platforms.linux;
  };
}
