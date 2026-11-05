# How the clock is kept, for the launcher's Time, built from this
# directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "timekeeping";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The shell's checks stand in for the reader with this day, which a
      # test holds to what the reader says of it.
      ./calendars-reading.json
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says how chrony keeps the clock, for the launcher's Time";
    license = lib.licenses.mit;
    mainProgram = "timekeeping";
    platforms = lib.platforms.linux;
  };
}
