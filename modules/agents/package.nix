# The agents daemon, built from this directory, its tests run as it
# builds. It words rows from the launcher's own vocabulary and serves
# them with the serving side every daemon shares (../daemon-serve), so
# the source is cut from the modules folder holding all three, and built
# from this one.
{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "agents";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests/words.json
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
      ../quickshell/launcher/words.json
    ];
  };
  sourceRoot = "${src.name}/agents";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Makes Agents' rows ready for the launcher";
    license = lib.licenses.mit;
    mainProgram = "agents";
    platforms = lib.platforms.linux;
  };
}
