# eero, built from this directory, its tests run as it builds. Its rows take
# the form every daemon's do (../ready-rows), so the source is cut from the
# modules folder holding both, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "eero";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The tests read eero's answers and the model drawn from them.
      ./contract
      ./home-network.json
      # The theme the terminal's colours are tested on.
      ./tests/osaka-jade.json
      # The launcher's words the rows are tested in.
      ./tests/words.json
      # The rows the desktop's tests serve, held to those the daemon makes.
      ./tests/rows.json
      ../ready-rows/Cargo.toml
      ../ready-rows/src
    ];
  };
  sourceRoot = "${src.name}/eero";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Reads the home network's eero from eero's cloud, at home or away";
    license = lib.licenses.mit;
    mainProgram = "eero";
    # It seals its session with systemd's credential service.
    platforms = lib.platforms.linux;
  };
}
