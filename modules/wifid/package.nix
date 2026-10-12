# wifid and its sign-in window, built from this directory, their tests run as
# they build.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "wifid";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./daemon
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Joins Wi-Fi networks and finds the ones that want a sign-in";
    license = lib.licenses.mit;
    mainProgram = "wifid";
    platforms = lib.platforms.linux;
  };
}
