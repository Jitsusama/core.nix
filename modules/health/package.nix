# health, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "health";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Keeps the day's tally of what the machine's journals said";
    license = lib.licenses.mit;
    mainProgram = "health";
    # It reads systemd's journals.
    platforms = lib.platforms.linux;
  };
}
