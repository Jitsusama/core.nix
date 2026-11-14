# The clock's time zone following where the laptop is, built from this
# directory, its tests run as it builds. It takes its shape from the
# terminal kit (../../terminal-kit), so the source is cut from the folder
# holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "clock-follows";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/quickshell/clock-follows";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Moves the clock's time zone to where the laptop is, and back when asked";
    license = lib.licenses.mit;
    mainProgram = "clock-follows";
    platforms = lib.platforms.linux;
  };
}
