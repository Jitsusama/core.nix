# transcripts-people, built from this directory, its tests run as it
# builds. It follows call-transcripts' daemon with the watch every daemon
# shares (../daemon-serve), says its rows in the one form (../ready-rows)
# and takes its shape from the terminal kit (../terminal-kit), so the
# source is cut from the modules folder holding them all, and built from
# this one. Its rows are said in the launcher's words.json, which its
# tests read as the people daemon hands it over.
{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "transcripts-people";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests/source.rs
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
      ../ready-rows/Cargo.toml
      ../ready-rows/src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/call-transcripts";
  # The launcher's words, joined from each place's file, put where its
  # tests read words.json (../quickshell/launcher/words.nix).
  postUnpack = ''
    chmod -R u+w ${src.name}
    mkdir -p ${src.name}/quickshell/launcher
    cp ${(import ../quickshell/launcher/words.nix).file} ${src.name}/quickshell/launcher/words.json
  '';
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says the calls written down with someone, for People";
    license = lib.licenses.mit;
    mainProgram = "transcripts-people";
    platforms = lib.platforms.linux;
  };
}
