# How the clock is kept, for the launcher's Time, built from this
# directory, its tests run as it builds. When the sun sets is a crate it
# shares with the backlight's night light (../../sun), so the source is cut
# from the modules folder holding both, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "timekeeping";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The shell's checks stand in for the reader with this day, which a
      # test holds to what the reader says of it.
      ./calendars-reading.json
      ../../sun/Cargo.toml
      ../../sun/src
    ];
  };
  sourceRoot = "${src.name}/quickshell/timekeeping";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says how chrony keeps the clock, for the launcher's Time";
    license = lib.licenses.mit;
    mainProgram = "timekeeping";
    platforms = lib.platforms.linux;
  };
}
