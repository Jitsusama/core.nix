# The clipboard's history for the launcher, built from this directory, its
# tests run as it builds. It serves with the serving side every daemon
# shares (../../daemon-serve) in the one form every daemon's rows take
# (../../ready-rows), so the source is cut from the modules folder holding
# them, and built from this one. Its tests read the launcher's words.json,
# which its rows are said in.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "clipboard-history";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../../daemon-serve/Cargo.toml
      ../../daemon-serve/src
      ../../ready-rows/Cargo.toml
      ../../ready-rows/src
    ];
  };
  sourceRoot = "${src.name}/quickshell/clipboard";
  # The launcher's words, joined from each place's file, put where its
  # sources and tests read words.json (../quickshell/launcher/words.nix).
  postUnpack = ''
    chmod -R u+w ${src.name}
    mkdir -p ${src.name}/quickshell/launcher
    cp ${(import ../launcher/words.nix).file} ${src.name}/quickshell/launcher/words.json
  '';
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Keeps the clipboard's copies, text and pictures, for the launcher";
    license = lib.licenses.mit;
    mainProgram = "clipboard-history";
  };
}
